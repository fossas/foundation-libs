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
//! [`tree-sitter-cpp`]: https://github.com/tree-sitter/tree-sitter-c
//! [standards]: https://en.wikipedia.org/wiki/C%2B%2B#History
use super::c99_tc3;
use crate::{impl_language, impl_prelude::*};

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

impl SnippetExtractor for Extractor {
    type Language = Language;

    fn extract(
        opts: &SnippetOptions,
        content: impl AsRef<[u8]>,
    ) -> Result<Vec<Snippet<Self::Language>>, ExtractorError> {
        c99_tc3::Extractor::extract(opts, content)
            .map(|snippets| snippets.into_iter().map(|snippet| snippet.into()).collect())
    }
}
