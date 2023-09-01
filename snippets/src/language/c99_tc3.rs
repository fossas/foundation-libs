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

use itertools::Itertools;
use tap::{Pipe, Tap};
use tracing::{debug, warn};
use tree_sitter::Node;
use tree_sitter_traversal::{traverse, traverse_tree, Order};

use crate::debugging::ToDisplayEscaped;
use crate::{impl_language, impl_prelude::*};

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
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_c::language())?;

        let content = content.as_ref();
        let Some(tree) = parser.parse(content, None) else {
            warn!("provided content did not parse to a tree");
            return Vec::new().pipe(Ok);
        };

        traverse_tree(&tree, Order::Pre)
            // Metadata is used further in the pipeline.
            .map(|node| (node, SnippetLocation::from(node.byte_range())))
            // Report syntax errors as warnings.
            // Always write a debugging line for each node, regardless of the kind of node.
            .inspect(|(node, location)| inspect_node(node, location, content))
            // Nodes that are not "named" are syntax,
            // which this function currently ignores.
            //
            // Reference:
            // https://tree-sitter.github.io/tree-sitter/using-parsers#named-vs-anonymous-nodes
            .filter(|(node, _)| node.is_named())
            // Hand each node off to be processed into possibly many snippets,
            // based on the provided options.
            .flat_map(|(node, loc)| {
                opts.cartesian_product()
                    .filter(move |(target, _, _)| matches_target(*target, node))
                    .map(move |(t, kind, method)| (t, SnippetMetadata::new(kind, method, loc)))
                    .filter_map(move |(target, meta)| extract(target, meta, node, content))
            })
            // Then just collect all the produced snippets and done!
            .collect_vec()
            .pipe(Ok)
    }
}

#[tracing::instrument(skip_all, fields(%target, kind = %meta.kind(), method = %meta.method(), location = %meta.location()))]
fn extract<L>(
    target: SnippetTarget,
    meta: SnippetMetadata,
    node: Node<'_>,
    content: &[u8],
) -> Option<Snippet<L>> {
    match target {
        SnippetTarget::Function => extract_function(meta, node, content),
    }
}

#[tracing::instrument(skip_all)]
fn extract_function<L>(
    meta: SnippetMetadata,
    node: Node<'_>,
    content: &[u8],
) -> Option<Snippet<L>> {
    // The raw content here is just extracted for debugging.
    let raw = meta.location().extract_from(content);
    debug!(raw = %raw.display_escaped());

    // The actual context, the part the snippet scanner cares about, is extracted here.
    // It also returns a new location so FOSSA can report a more accurate range for the snippet.
    let (context, loc) = extract_context(meta, node, content)?;
    debug!(context = %context.display_escaped());

    // Transformations are applied on text extraction from the context.
    // A new location is _not_ generated by this function because the transformed text
    // won't have any more real of a relation to the original "range of text"
    // than the context's range.
    let text = extract_text(meta.method(), context);
    debug!(text = %text.display_escaped());

    // The more exact location generated above overwrites the overall node location,
    // otherwise users would just always see the whole node.
    let meta = SnippetMetadata::new(meta.kind(), meta.method(), loc);
    Snippet::from(meta, text)
        .tap(|snippet| debug!(fingerprint = %snippet.fingerprint()))
        .pipe(Some)
}

#[tracing::instrument(skip_all)]
fn extract_context<'a>(
    meta: SnippetMetadata,
    node: Node<'_>,
    content: &'a [u8],
) -> Option<(&'a [u8], SnippetLocation)> {
    match meta.kind() {
        SnippetKind::Full => (content, meta.location()).pipe(Some),
        SnippetKind::Body => {
            // This node starts at the start of the function.
            // The end of the parameter list signifies the end of the function signature.
            let Some((plist, _)) = traverse(node.walk(), Order::Pre)
                .map(|node| (node, node.byte_range().pipe(SnippetLocation::from)))
                .inspect(|(node, location)| inspect_node(node, location, content))
                .find(|(node, _)| node.kind() == "parameter_list")
            else {
                warn!("function body not found: no 'parameter_list' node");
                return None;
            };

            // This node ends at the end of the function.
            // Since the end of the parameter list signifies the start, anything between is the body.
            let mut offset = plist.end_byte();
            let end = node.end_byte();

            // Skip any spaces between the two though. Nobody cares about these.
            while offset < end && content[offset].is_ascii_whitespace() {
                offset += 1;
            }
            if offset == end {
                warn!("function body appears to be made up entirely of spaces");
                offset = plist.end_byte();
            }

            // Since the resulting snippet is a subset of the original snippet
            // indicated by the entire node, return a new location for it.
            let report_as = SnippetLocation::builder()
                .byte_offset(offset)
                .byte_len(end - offset)
                .build();

            (report_as.extract_from(content), report_as).pipe(Some)
        }
        SnippetKind::Signature => {
            // This node starts at the start of the function.
            //
            // Find the parameter list, signifying the end of the function signature,
            // and just slice all the text up to the end of that.
            let Some((plist, _)) = traverse(node.walk(), Order::Pre)
                .map(|node| (node, node.byte_range().pipe(SnippetLocation::from)))
                .inspect(|(node, location)| inspect_node(node, location, content))
                .find(|(node, _)| node.kind() == "parameter_list")
            else {
                warn!("function signature not found: no 'parameter_list' node");
                return None;
            };

            // Since the resulting snippet is a subset of the original snippet
            // indicated by the entire node, return a new location for it.
            let report_as = SnippetLocation::builder()
                .byte_offset(meta.location().start_byte())
                .byte_len(plist.end_byte() - meta.location().start_byte())
                .build();

            (report_as.extract_from(content), report_as).pipe(Some)
        }
    }
}

#[tracing::instrument(skip_all)]
fn extract_text(method: SnippetMethod, content: &[u8]) -> Cow<'_, [u8]> {
    match method {
        SnippetMethod::Raw => Cow::from(content),
        SnippetMethod::Normalized(tf) => transform(tf, content).pipe(Cow::from),
    }
}

#[tracing::instrument(skip_all)]
fn transform(transform: SnippetTransform, content: &[u8]) -> Vec<u8> {
    match transform {
        SnippetTransform::Code => todo!(),
        SnippetTransform::Comment => todo!(),
        SnippetTransform::Space => todo!(),
    }
}

/// Report whether the given treesitter node kind is a valid entrypoint for the target.
///
/// Defined here instead of on [`SnippetTarget`] because that type should be generic across
/// language parse strategies instead of being tied to treesitter-specific implementations.
#[tracing::instrument(level = "DEBUG", skip_all, fields(%target, node_kind = %node.kind()), ret)]
fn matches_target(target: SnippetTarget, node: Node<'_>) -> bool {
    match target {
        SnippetTarget::Function => node.kind() == "function_definition",
    }
}

#[track_caller]
fn inspect_node(node: &Node<'_>, location: &SnippetLocation, content: &[u8]) {
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
