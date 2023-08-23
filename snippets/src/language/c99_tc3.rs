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

use std::{io::Read, marker::PhantomData};

use fallible_iterator::FallibleIterator;
use typed_builder::TypedBuilder;

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

    type ExtractIterator = SnippetExtractionIterator<Self::Language>;

    fn support<R: Read>(source: R) -> Result<Self::Support, ExtractorError> {
        todo!()
    }

    fn extract<R: Read>(source: R) -> Self::ExtractIterator {
        SnippetExtractionIterator::builder()
            .with_language(tree_sitter_c::language)
            .build()
    }
}

#[derive(TypedBuilder)]
pub struct SnippetExtractionIterator<L> {
    #[builder(default)]
    language: PhantomData<L>,

    #[builder(default_code = "tree_sitter::Parser::new()")]
    parser: tree_sitter::Parser,

    #[builder(setter(strip_option))]
    with_language: Option<fn() -> tree_sitter::Language>,
}

impl<L> FallibleIterator for SnippetExtractionIterator<L> {
    type Item = Snippet<L>;

    type Error = ExtractorError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(language) = self.with_language.take() {
            if let Err(err) = self.parser.set_language(language()) {
                return Err(LanguageError(err).into());
            }
        }

        todo!()
    }
}
