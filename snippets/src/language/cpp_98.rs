//! Implements an [`Extractor`] for the C++ programming language.
//!
//! # Standard
//!
//! C++ has evolved over the years via different [standards].
//! This implementation primarily targets parsing C++ 98.
//! Initially, this extractor will only support extracting bare functions from C++.
//! Methods and/or classes are planned for implementation.
//!
//! This is because we are using the grammar maintained by the [`tree-sitter`]
//! project for C++, [`tree-sitter-cpp`].
//!
//! The link provided in the readme links to a couple grammars.
//! It's unclear exactly which one the code in the repository corresponds to.
//! One of the linked grammars, however, is clearly labeled [`iso-14882:1998`].
//! The repository lists parsable [`node types`].
//!
//! That being said, this extractor should generally support newer versions
//! of the C++ programming language. This is because this extractor is only
//! concerned with functions, and a review of the later C++ standards
//! does not imply that function parsing has changed.
//!
//! # Targets
//! This extractor supports extracting the following as snippets from C++ files:
//!
//! * Function Signatures
//! * Function Bodies
//! * Full Functions Declarations
//!
//! "Function" means functions that are not a method definition as part of a class.
//!
//! [`Extractor`]: crate::Extractor
//! [`iso-14882:1998`]: https://www.externsoft.ch/download/cpp-iso.html
//! [`node types`]: https://github.com/tree-sitter/tree-sitter-cpp/blob/master/src/node-types.json
//! [`tree-sitter`]: https://github.com/tree-sitter/tree-sitter
//! [`tree-sitter-cpp`]: https://github.com/tree-sitter/tree-sitter-cpp
//! [standards]: https://en.wikipedia.org/wiki/C%2B%2B#History
use std::borrow::Cow;

use tap::{Pipe, Tap};
use tracing::{debug, warn};
use tree_sitter::Node;
use tree_sitter_traversal::{traverse, traverse_tree, Order};

use crate::debugging::ToDisplayEscaped;
use crate::text::normalize_space;
use crate::tree_sitter_consts::{NODE_KIND_FUNC_DEF, NODE_KIND_OPEN_BRACE};
use crate::{impl_language, impl_prelude::*};

use super::c99_tc3;
use super::normalize_code::normalize_code;
use super::normalize_comments::normalize_comments;
use super::snippet_context::SnippetContext;
/// This module implements support for CPP 98.
///
/// Review module documentation for more details.
#[derive(Copy, Clone)]
pub struct Language;

impl SnippetLanguage for Language {
    const NAME: &'static str = "cpp_98";
    const STRATEGY: LanguageStrategy = LanguageStrategy::Static;
}

impl_language!(Language);

/// Supports extracting snippets for CPP 98 source code.
pub struct Extractor;

impl From<Snippet<c99_tc3::Language>> for Snippet<Language> {
    fn from(
        Snippet {
            metadata,
            fingerprint,
            content,
            ..
        }: Snippet<c99_tc3::Language>,
    ) -> Snippet<Language> {
        Snippet::builder()
            .metadata(metadata)
            .fingerprint(fingerprint)
            .content(content)
            .build()
    }
}

impl From<&Snippet<c99_tc3::Language>> for Snippet<Language> {
    fn from(snippet: &Snippet<c99_tc3::Language>) -> Snippet<Language> {
        snippet.to_owned().into()
    }
}

// This extractor is basically a copy-paste of c99_tc3 extractor.
// If you make changes to this extractor, consider if they should also be made to the c99_tc3 extractor
// or if the functionality makes sense to be shared.
impl SnippetExtractor for Extractor {
    type Language = Language;

    fn extract(
        opts: &SnippetOptions,
        content: impl AsRef<[u8]>,
    ) -> Result<Vec<Snippet<Self::Language>>, ExtractorError> {
        let mut parser = init_parser()?;

        let content = content.as_ref();
        let Some(tree) = parser.parse(content, None) else {
            warn!("provided content did not parse to a tree");
            return Vec::new().pipe(Ok);
        };

        traverse_tree(&tree, Order::Pre)
            // Report syntax errors as warnings.
            // Always write a debugging line for each node, regardless of the kind of node.
            .inspect(|node| inspect_node(node, content))
            // Nodes that are not "named" are syntax,
            // which this function currently ignores.
            //
            // Reference:
            // https://tree-sitter.github.io/tree-sitter/using-parsers#named-vs-anonymous-nodes
            .filter(|node| node.is_named())
            // Hand each node off to be processed into possibly many snippets,
            // based on the provided options.
            .flat_map(|node| {
                let loc = node.byte_range().pipe(SnippetLocation::from);
                opts.cartesian_product()
                    .filter(move |(target, _, _)| matches_target(*target, node))
                    .map(move |(t, kind, method)| (t, SnippetMetadata::new(kind, method, loc)))
                    .filter_map(move |(target, meta)| extract(target, meta, node, content))
            })
            // Then just collect all the produced snippets and done!
            // `From<Iterator> for Result<T, E>` stops iteration on the first error as well.
            .collect()
    }
}

#[tracing::instrument(skip_all, fields(%target, kind = %meta.kind(), method = %meta.method(), location = %meta.location()))]
fn extract<L>(
    target: SnippetTarget,
    meta: SnippetMetadata,
    node: Node<'_>,
    content: &[u8],
) -> Option<Result<Snippet<L>, ExtractorError>> {
    match target {
        SnippetTarget::Function => extract_function(meta, node, content),
    }
}

#[tracing::instrument(skip_all)]
fn extract_function<L>(
    meta: SnippetMetadata,
    node: Node<'_>,
    content: &[u8],
) -> Option<Result<Snippet<L>, ExtractorError>> {
    // The raw content here is just extracted for debugging.
    let raw = meta.location().extract_from(content);
    debug!(raw = %raw.display_escaped());

    // The actual context, the part the snippet scanner cares about, is extracted here.
    // It also returns a new location so FOSSA can report a more accurate range for the snippet.
    let context = extract_context(meta, node, content)?;
    debug!(context = %context.content().display_escaped());

    // Transformations are applied on text extraction from the context.
    // A new location is _not_ generated by this function because the transformed text
    // won't have any more real of a relation to the original "range of text"
    // than the context's range.
    let text = extract_text(meta.method(), &context);
    debug!(text = %text.display_escaped());

    // The more exact location generated above overwrites the overall node location,
    // otherwise users would just always see the whole node.
    let meta = SnippetMetadata::new(meta.kind(), meta.method(), context.location());
    Snippet::from(meta, text)
        .tap(|snippet| debug!(fingerprint = %snippet.fingerprint()))
        .pipe(Ok)
        .pipe(Some)
}

/// Extracts the "context" of a node with the provided metadata.
///
/// This consists of:
/// - A vector of parsed [`Node`]s that make up the part of the subtree considered relevant.
/// - A [`SnippetLocation`] selecting the (usually constrained) span that makes up the context.
///
/// Both are returned instead of just one or the other because different text extractors care
/// about different sets of data. Specifically:
/// - [`SnippetMethod::Raw`] needs the text _as written_.
/// - [`SnippetTransform::Comment`] needs both; to find comments and then slice them out.
/// - [`SnippetTransform::Space`] needs the parsed form so it normalizes spaces.
/// - [`SnippetTransform::Code`] needs the parsed form so it normalizes spaces and slices comments.
#[tracing::instrument(skip_all)]
fn extract_context<'a>(
    meta: SnippetMetadata,
    node: Node<'a>,
    content: &'a [u8],
) -> Option<SnippetContext<'a>> {
    match meta.kind() {
        SnippetKind::Full => Some(SnippetContext::new(node, meta.location(), content)),
        SnippetKind::Body => {
            let parts = FunctionParts::from(meta, node, content);

            // Unless a delimiter was found, not much for this extractor to do.
            if parts.body.is_empty() {
                warn!("function body not found");
                return None;
            }

            // This node ends at the end of the function.
            // Since the end of the delimiter signifies the start, anything between is the body.
            let mut offset = parts.delimit_byte;
            let end = node.end_byte();

            // Spaces between the delimiter end and start of the actual
            // body content should not be significant.
            while offset < end && content[offset].is_ascii_whitespace() {
                offset += 1;
            }
            if offset == end {
                warn!("function body appears to be made up entirely of spaces");
                offset = parts.delimit_byte;
            }

            // The new location is meant to enable a more precise report of where
            // exactly the snippet was found in the file; otherwise the snippet
            // would be reported to have come from the whole function
            // instead of just this small part.
            let report_as = SnippetLocation::builder()
                .byte_offset(offset)
                .byte_len(end - offset)
                .build();

            // The context reports the nodes that made up this extracted snippet,
            // for future pipeline operations to use.
            SnippetContext::from_nodes(parts.body, report_as, content).pipe(Some)
        }
        SnippetKind::Signature => {
            let parts = FunctionParts::from(meta, node, content);

            // Unless a delimiter was found, not much for this extractor to do.
            if parts.signature.is_empty() {
                warn!("function signature not found");
                return None;
            }

            // This node starts at the start of the function.
            // Since the start of the delimiter signifies the end, anything between is the signature.
            let mut offset = parts.delimit_byte;
            let start = node.start_byte();

            // Spaces between the signature and delimiter are not significant.
            while offset > start && content[offset - 1].is_ascii_whitespace() {
                offset -= 1;
            }
            if offset == start {
                warn!("function signature appears to be made up entirely of spaces");
                offset = parts.delimit_byte;
            }

            // The new location is meant to enable a more precise report of where
            // exactly the snippet was found in the file; otherwise the snippet
            // would be reported to have come from the whole function
            // instead of just this small part.
            let report_as = SnippetLocation::builder()
                .byte_offset(meta.location().start_byte())
                .byte_len(offset - meta.location().start_byte())
                .build();

            // The context reports the nodes that made up this extracted snippet,
            // for future pipeline operations to use.
            SnippetContext::from_nodes(parts.signature, report_as, content).pipe(Some)
        }
    }
}

#[tracing::instrument(skip_all)]
fn extract_text<'a>(method: SnippetMethod, context: &'a SnippetContext) -> Cow<'a, [u8]> {
    match method {
        // For the happy path, raw snippets, no extra allocations!
        SnippetMethod::Raw => Cow::from(context.content()),
        // Any modification will require a new vector.
        SnippetMethod::Normalized(tf) => transform(tf, context),
    }
}

#[tracing::instrument(skip_all)]
fn transform<'a>(transform: SnippetTransform, context: &'a SnippetContext) -> Cow<'a, [u8]> {
    match transform {
        SnippetTransform::Code => normalize_code(context),
        SnippetTransform::Comment => normalize_comments(context).into(),
        SnippetTransform::Space => normalize_space(context.content()),
    }
}

/// Report whether the given treesitter node kind is a valid entrypoint for the target.
///
/// Defined here instead of on [`SnippetTarget`] because that type should be generic across
/// language parse strategies instead of being tied to treesitter-specific implementations.
#[tracing::instrument(level = "DEBUG", skip_all, fields(%target, node_kind = %node.kind()), ret)]
fn matches_target(target: SnippetTarget, node: Node<'_>) -> bool {
    match target {
        SnippetTarget::Function => node.kind() == NODE_KIND_FUNC_DEF,
    }
}

#[tracing::instrument(skip_all)]
fn inspect_node(node: &Node<'_>, content: &[u8]) {
    let location = node.byte_range().pipe(SnippetLocation::from);
    if node.is_error() {
        let start = node.start_position();
        let end = node.end_position();
        warn!(
            %location,
            content = %location.extract_from(content).display_escaped(),
            kind = %"syntax_error",
            line_start = start.row,
            line_end = end.row,
            col_start = start.column,
            col_end = end.column,
        );
    } else {
        debug!(
            %location,
            content = %location.extract_from(content).display_escaped(),
            kind = %node.kind(),
        );
    }
}

#[tracing::instrument]
fn init_parser() -> Result<tree_sitter::Parser, ExtractorError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_cpp::language())?;
    Ok(parser)
}

#[derive(Debug, Clone)]
struct FunctionParts<'a> {
    delimit_byte: usize,
    signature: Vec<Node<'a>>,
    body: Vec<Node<'a>>,
}

impl<'a> FunctionParts<'a> {
    /// Break a function into the nodes in the body and signature.
    ///
    /// As a performance optimization, if the metadata only asks for the signature,
    /// body nodes are not stored. They are still traversed, in case treesitter
    /// iterates over nodes out of order.
    fn from(meta: SnippetMetadata, node: Node<'a>, content: &'a [u8]) -> Self {
        let nodes = traverse(node.walk(), Order::Pre).inspect(|node| inspect_node(node, content));

        let mut signature = Vec::new();
        let mut body = Vec::new();
        let mut delimit_byte = None;
        let mut last_byte = node.start_byte();
        for node in nodes {
            match (node.kind(), delimit_byte) {
                // Just delimit on first open brace, this is C.
                (NODE_KIND_OPEN_BRACE, None) => {
                    delimit_byte = node.start_byte().pipe(Some);
                    if meta.kind == SnippetKind::Signature {
                        break;
                    } else {
                        body.push(node);
                    }
                }
                // If a delimiter has been found, push the node into body or signature,
                // depending on which side of the delimiter the node falls.
                (_, Some(delimit_byte)) => {
                    if node.start_byte() < delimit_byte {
                        signature.push(node);
                    } else if meta.kind != SnippetKind::Signature {
                        body.push(node);
                    }
                }
                // For no delimiter, default to signature, since this function starts
                // at the start of the function definition.
                (_, None) => signature.push(node),
            }
            last_byte = node.end_byte();
        }

        FunctionParts {
            signature,
            body,
            delimit_byte: delimit_byte.unwrap_or(last_byte),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_language_compatible() {
        let _ = init_parser().expect("parser language must be compatible");
    }
}
