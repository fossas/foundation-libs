//! Implements an [`Extractor`] for the C programming language.
//!
//! # Standard
//!
//! C has evolved over the years via different [standards].
//! This implementation primarily targets parsing [`C99`]
//! at the [`TC3`] revision.
//!
//! This is because we are using the grammar maintained by the [`tree-sitter`]
//! project for C, [`tree-sitter-c`], which in its readme states:
//!
//! > Adapted from this C99 grammar.
//!
//! The link provided in that readme doesn't link to a specific grammar,
//! but appears that it meant to do so; interpreting from the provided link
//! it appears to indicate the [`iso-9899-tc3`] grammar.
//!
//! That being said, this extractor should generally support newer versions
//! of the C programming language. This is because this extractor is only
//! concerned with functions, and a review of the later C standards
//! does not imply that function parsing has changed.
//!
//! [`Extractor`]: crate::Extractor
//! [standards]: https://en.wikipedia.org/wiki/C_(programming_language)#History
//! [`C99`]: https://en.wikipedia.org/wiki/C99
//! [`TC3`]: https://www.open-std.org/jtc1/sc22/wg14/
//! [`tree-sitter`]: https://github.com/tree-sitter/tree-sitter
//! [`tree-sitter-c`]: https://github.com/tree-sitter/tree-sitter-c
//! [`iso-9899-tc3`]: https://github.com/slebok/zoo/tree/master/zoo/c/c99/iso-9899-tc3

use std::borrow::Cow;

use itertools::{FoldWhile, Itertools};
use tap::{Pipe, Tap};
use tracing::{debug, warn};
use tree_sitter::Node;
use tree_sitter_traversal::{traverse, traverse_tree, Order};

use crate::debugging::ToDisplayEscaped;
use crate::text::normalize_space;
use crate::tree_sitter_consts::{NODE_KIND_COMMENT, NODE_KIND_FUNC_DEF, NODE_KIND_PARAM_LIST};
use crate::{impl_language, impl_prelude::*};

use super::snippet_context::SnippetContext;
use super::normalize_comments::normalize_comments;

/// This module implements support for C99 TC3.
///
/// Review module documentation for more details.
#[derive(Copy, Clone)]
pub struct Language;

impl SnippetLanguage for Language {
    const NAME: &'static str = "c99_tc3";
    const STRATEGY: LanguageStrategy = LanguageStrategy::Static;
}
impl_language!(Language);

/// Supports extracting snippets from C99 TC3 source code.
pub struct Extractor;

impl SnippetExtractor for Extractor {
    type Language = Language;

    #[tracing::instrument(skip_all, fields(kinds = %opts.kinds(), transforms = %opts.transforms(), content_len = content.as_ref().len()))]
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
    let context_bytes = context.content();
    debug!(context = %context_bytes.display_escaped());

    // Transformations are applied on text extraction from the context.
    // A new location is _not_ generated by this function because the transformed text
    // won't have any more real of a relation to the original "range of text"
    // than the context's range.
    let text = extract_text(meta.method(), &context);
    debug!(text = %text.display_escaped());

    // The more exact location generated above overwrites the overall node location,
    // otherwise users would just always see the whole node.
    let meta = SnippetMetadata::new(meta.kind(), meta.method(), *context.location());
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
        SnippetKind::Full => {
            let context = traverse(node.walk(), Order::Pre).collect();
            let location = meta.location();
            Some(SnippetContext::new(
                context,
                location,
                location.extract_from(content),
            ))
        }
        SnippetKind::Body => {
            // This node starts at the start of the function,
            // so to get the body it just needs to select nodes after
            // the end of the parameter list and any immediately following comments.
            let (delim, context) = traverse(node.walk(), Order::Pre)
                .inspect(|node| inspect_node(node, content))
                .filter(|node| node.is_named())
                .skip_while(|node| node.kind() != NODE_KIND_PARAM_LIST)
                .fold(
                    (None, Vec::new()),
                    |(delim, mut context), node| match node.kind() {
                        // The param list is the default delimiter. Start a new context here.
                        NODE_KIND_PARAM_LIST => (Some(node), Vec::new()),
                        // However, if there are comments immediately after the param list
                        // (which we know because no context has been recorded),
                        // keep resetting the context until a not-comment is hit.
                        NODE_KIND_COMMENT if context.is_empty() => (Some(node), Vec::new()),
                        // Anything else after this, including comments, is recorded in the context.
                        _ => {
                            context.push(node);
                            (delim, context)
                        }
                    },
                );

            // Unless a delimiter was found, not much for this extractor to do.
            let Some(delim) = delim else {
                warn!("function body not found: no delimiter node found");
                return None;
            };

            // This node ends at the end of the function.
            // Since the end of the delimiter signifies the start, anything between is the body.
            let mut offset = delim.end_byte();
            let end = node.end_byte();

            // Spaces between the delimiter end and start of the actual
            // body content should not be significant.
            while offset < end && content[offset].is_ascii_whitespace() {
                offset += 1;
            }
            if offset == end {
                warn!("function body appears to be made up entirely of spaces");
                offset = delim.end_byte();
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
            SnippetContext::new(context, report_as, report_as.extract_from(content)).pipe(Some)
        }
        SnippetKind::Signature => {
            // This node starts at the start of the function,
            // so to get the signature it just needs to select nodes up to
            // and including the parameter list and any immediately following comments.
            // Whichever is the last node gets marked as the delimiter.
            let (delim, context) = traverse(node.walk(), Order::Pre)
                .inspect(|node| inspect_node(node, content))
                .filter(|node| node.is_named())
                .fold_while((None, Vec::new()), |(delim, mut context), node| {
                    match node.kind() {
                        // The param list is the default delimiter,
                        // but keep going after it's found to handle comments.
                        node_kind if node_kind == NODE_KIND_PARAM_LIST => {
                            context.push(node);
                            (Some(node), context).pipe(FoldWhile::Continue)
                        }
                        // If there are comments immediately after the param list,
                        // set them as the delimiter instead.
                        // Keep going to handle multiple comments after the param list.
                        node_kind if node_kind == NODE_KIND_COMMENT && delim.is_some() => {
                            context.push(node);
                            (Some(node), context).pipe(FoldWhile::Continue)
                        }
                        // Once the reader hits something that isn't a delimiter candidate,
                        // and a delimiter has already been found, it's done!
                        _ if delim.is_some() => (delim, context).pipe(FoldWhile::Done),
                        // This is the catch-all, which mostly runs before
                        // hitting a delimiter candidate. It just builds the context.
                        _ => {
                            context.push(node);
                            (None, context).pipe(FoldWhile::Continue)
                        }
                    }
                })
                .into_inner();

            // Unless a delimiter was found, not much for this extractor to do.
            let Some(delim) = delim else {
                warn!("function body not found: no delimiter node found");
                return None;
            };

            // The new location is meant to enable a more precise report of where
            // exactly the snippet was found in the file; otherwise the snippet
            // would be reported to have come from the whole function
            // instead of just this small part.
            let report_as = SnippetLocation::builder()
                .byte_offset(meta.location().start_byte())
                .byte_len(delim.end_byte() - meta.location().start_byte())
                .build();

            // The context reports the nodes that made up this extracted snippet,
            // for future pipeline operations to use.
            SnippetContext::new(context, report_as, report_as.extract_from(content)).pipe(Some)
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
        // We need to fill these out before MVP launch, but for now we'll move on.
        SnippetTransform::Code => unimplemented!(),
        SnippetTransform::Comment => normalize_comments(context),
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
    parser.set_language(tree_sitter_c::language())?;
    Ok(parser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_language_compatible() {
        let _ = init_parser().expect("parser language must be compatible");
    }
}
