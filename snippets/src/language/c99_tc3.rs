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

use itertools::Itertools;
use tap::Pipe;
use tracing::{debug, warn};
use tree_sitter::Node;
use tree_sitter_traversal::{traverse_tree, Order};

use crate::debugging::ToDisplayEscaped;
use crate::impl_prelude::*;

/// This module implements support for C99 TC3.
///
/// Review module documentation for more details.
pub struct Language;

impl SnippetLanguage for Language {
    const NAME: &'static str = "c99_tc3";
    const STRATEGY: LanguageStrategy = LanguageStrategy::Static;
}

/// Supports extracting snippets from C99 TC3 source code.
pub struct Extractor;

impl SnippetExtractor for Extractor {
    type Support = Support;

    type Language = Language;

    fn support(content: impl AsRef<[u8]>) -> Result<Self::Support, ExtractorError> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_c::language())?;

        if parser.parse(content, None).is_some() {
            Ok(Support::Full)
        } else {
            Ok(Support::None)
        }
    }

    #[tracing::instrument(skip_all)]
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
            // Nodes that are not "named" are syntax,
            // which this function currently ignores.
            //
            // Reference:
            // https://tree-sitter.github.io/tree-sitter/using-parsers#named-vs-anonymous-nodes
            .filter(|node| !node.is_named())
            // Metadata is used further in the pipeline.
            .map(|node| (node, SnippetLocation::from(node.byte_range())))
            // Always write the debug line, regardless of the kind of node.
            .inspect(|(node, loc)| {
                let kind = node.kind();
                let snippet = loc.extract_from(content);
                debug!("{kind}@{loc} -> '{}'", snippet.display_escaped());
            })
            // After this point, this function only cares about function definitions.
            .filter(|(node, _)| node.kind() == "function_definition")
            // Multiple snippets may be built from a single function definition,
            // depending on provided options.
            .flat_map(|(node, loc)| {
                opts.cartesian_product()
                    .map(move |(kind, method)| SnippetMetadata::new(kind, method, loc))
                    .map(move |meta| extract_one(meta, node, content))
            })
            // Then just collect all the produced snippets and done!
            .collect_vec()
            .pipe(Ok)
    }
}

#[tracing::instrument(skip_all, fields(kind = ?meta.kind(), method = ?meta.method(), loc = ?meta.location()))]
fn extract_one<'a, L>(meta: SnippetMetadata, _node: Node<'a>, content: &'a [u8]) -> Snippet<L> {
    let raw = meta.location().extract_from(content);
    debug!("raw: '{}'", raw.display_escaped());

    let context = match meta.kind() {
        SnippetKind::Full => raw,
        kind => unimplemented!("kind: {kind:?}"),
    };
    debug!("context: '{}'", context.display_escaped());

    let snippet = match meta.method() {
        SnippetMethod::Raw => context,
        method => unimplemented!("method: {method:?}"),
    };

    debug!("snippet: '{}'", snippet.display_escaped());
    Snippet::new(meta, snippet)
}
