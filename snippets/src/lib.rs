//! Provides a framework and implementations for extracting snippets of programming languages from files.
//!
//! # Aspirations
//!
//! - Extensible over feature complete
//! - Platform independent over platform optimized
//! - Reliable over performant
//!
//! # Feature flags
//!
//! The main library, which enables consumers to plug their own implementations, is available by default.
//! Features are most commonly used to enable support for languages,
//! but other kinds of flags exist; see the table below for details.
//!
//! Name | Description | Kind
//! ---|---|---
//! `lang-all` | Enables all features that are of the kind "Language" | Language
//! `lang-c99-tc3` | Enables support for C99 TC3 | Language
//! `sha2-asm` | Enables hardware acceleration for SHA2 | Performance

use std::{
    borrow::Cow,
    cmp::Ordering,
    marker::PhantomData,
    ops::{Range, RangeInclusive},
    str::Utf8Error,
};

use derivative::Derivative;
use derive_more::{Constructor, Deref, Index};
pub use fallible_iterator::FallibleIterator;
use flagset::{flags, FlagSet};
use getset::{CopyGetters, Getters};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use strum::{Display, EnumIter};
use tap::Pipe;
use thiserror::Error;
use typed_builder::TypedBuilder;

pub mod debugging;
pub mod language;
pub mod text;

/// Convenience import for all types that
/// an implementation of [`Extractor`] would likely need.
///
/// Some commonly-named types are renamed to reduce the liklihood of collisions
/// when imported via this prelude: e.g. [`Error`] becomes [`ExtractorError`].
///
/// [`Extractor`]: crate::Extractor
/// [`Error`]: crate::Error
pub mod impl_prelude {
    pub use super::{
        Error as ExtractorError, Extractor as SnippetExtractor, Kind as SnippetKind,
        Kinds as SnippetKinds, Language as SnippetLanguage, LanguageError,
        Location as SnippetLocation, Metadata as SnippetMetadata, Method as SnippetMethod,
        Options as SnippetOptions, Snippet, Strategy as LanguageStrategy,
        Transform as SnippetTransform, Transforms as SnippetTransforms,
    };
}

/// Errors reported by [`Extractor`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("configure parser")]
    Configure(#[from] LanguageError),

    #[error("read input as utf8")]
    DecodeUTF8(#[from] Utf8Error),
}

impl From<tree_sitter::LanguageError> for Error {
    fn from(err: tree_sitter::LanguageError) -> Self {
        LanguageError(err).into()
    }
}

/// An error that occurrs when trying to assign an incompatible language to a parser.
// Note: Implementing it this way allows us to keep `tree_sitter` out of the public API.
//       More details: https://docs.rs/thiserror/latest/thiserror/
#[derive(Debug, Error)]
#[error(transparent)]
pub struct LanguageError(#[from] tree_sitter::LanguageError);

/// An implementation of [`Extractor`] enables snippets to be extracted
/// from a given unit of source code (typically a file).
pub trait Extractor {
    /// The source language supported by the implementation.
    type Language: Language;

    /// Reads the provided unit of source code for snippets, according to the provided options.
    ///
    /// # Reader
    ///
    /// The [`Read`] instance provided to `source` may be partially or fully consumed during this process.
    ///
    /// If the reader was previously read (partially or fully, by example via [`Extractor::support`]),
    /// it almost definitely needs to be reset to the initial point before using this method.
    fn extract(
        opts: &Options,
        content: impl AsRef<[u8]>,
    ) -> Result<Vec<Snippet<Self::Language>>, Error>;
}

/// Options for extracting snippets.
/// Options are constructed via the `Options::builder` method.
///
/// By default, all kinds of snippet are extracted, and all normalizations are applied.
/// Providing an empty set to [`Options::kinds`] is equivalent to the default set
/// (namely, all [`Kind`]s).
///
/// All options implicitly assume [`Method::Raw`]; it is not currently possible to disable
/// this snippet extraction method. This means that providing an empty set of transforms
/// (via [`Transforms::none`]) results in a collection of snippets extracted with [`Method::Raw`].
///
/// # Examples
///
/// Defaults:
/// ```
/// # use snippets::*;
/// let options = Options::default();
///
/// assert_eq!(options.kinds(), Kinds::full());
/// assert_eq!(options.transforms(), Transforms::full());
/// ```
///
/// Restricting the kinds of snippet extracted:
/// ```
/// # use snippets::*;
/// let options = Options::new(Kind::Signature, Transforms::full());
/// assert!(options.kinds().contains(Kind::Signature));
/// assert!(!options.kinds().contains(Kind::Body));
/// assert!(!options.kinds().contains(Kind::Full));
/// ```
///
/// Restricting the transforms applied:
/// ```
/// # use snippets::*;
/// let options = Options::new(Kinds::full(), Transform::Comment);
/// assert!(options.transforms().contains(Transform::Comment));
/// assert!(!options.transforms().contains(Transform::Space));
/// ```
///
/// Only use [`Method::Raw`]:
/// ```
/// # use snippets::*;
/// let options = Options::new(Kinds::full(), Transforms::none());
/// assert!(options.transforms().is_empty());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct Options {
    /// The kinds of snippet to extract.
    kinds: Kinds,

    /// The normalizations used to extract this snippet.
    transforms: Transforms,
}

impl Options {
    /// Create a new set of options for a snippet extractor.
    pub fn new(kinds: impl Into<Kinds>, transforms: impl Into<Transforms>) -> Self {
        Self {
            kinds: Self::non_empty_kinds(kinds),
            transforms: transforms.into(),
        }
    }

    /// Report the cartesian product of the configured [`Kind`]s of snippets to extract
    /// with configured [`Method`]s to apply.
    pub fn cartesian_product(&self) -> impl Iterator<Item = (Kind, Method)> {
        self.kinds
            .iter()
            .cartesian_product(Method::iter(self.transforms))
    }

    fn non_empty_kinds(input: impl Into<Kinds>) -> Kinds {
        let input = input.into();
        if input.is_empty() {
            Kinds::full()
        } else {
            input
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            kinds: Kinds::full(),
            transforms: Transforms::full(),
        }
    }
}

/// Standardizes the description of languages supported by [`Extractor`] implementations.
///
/// Note: [`impl_language!`] is available for convenience trait implementations if desired.
///
/// # Example
///
/// ```
/// use snippets::Language;
///
/// pub struct CustomLanguage;
///
/// impl Language for CustomLanguage {
///     const NAME: &'static str = "example";
///     const STRATEGY: snippets::Strategy = snippets::Strategy::Static;
/// }
///
/// assert_eq!(&format!("{}", CustomLanguage::display()), "example/static");
/// ```
///
/// # Customization
///
/// ```
/// use snippets::Language;
///
/// pub struct CustomLanguage;
///
/// impl Language for CustomLanguage {
///     const NAME: &'static str = "example";
///     const STRATEGY: snippets::Strategy = snippets::Strategy::Static;
///
///     fn display() -> &'static str {
///         "custom name"
///     }
/// }
///
/// assert_eq!(&format!("{}", CustomLanguage::display()), "custom name");
/// ```
pub trait Language {
    /// The name of the language.
    const NAME: &'static str;

    /// The strategy used for parsing the language.
    const STRATEGY: Strategy;

    /// Override the display of the language name if desired.
    fn display() -> &'static str {
        static DISPLAY: OnceCell<String> = OnceCell::new();
        DISPLAY.get_or_init(|| format!("{}/{}", Self::NAME, Self::STRATEGY))
    }
}

/// Convenience macro to implement standard traits for a [`Language`].
///
/// This library cannot auto implement debug implementations because
/// they may be foreign types for a foreign trait.
///
/// Trait | Default | Description
/// --- | --- | ---
/// `Debug` | Yes | Implement `std::fmt::Debug` with the same text as `Self::display()`.
/// `Display` | Yes | Implement `std::fmt::Display` with the same text as `Self::display()`.
///
/// Implement all "default" traits in the table above by calling this macro
/// with only the type name as an argument:
/// ```ignore
/// impl_language!(CustomLanguage);
/// ```
///
/// Implement a subset of traits by calling this macro with the type and trait:
/// ```ignore
/// impl_language!(CustomLanguage => Debug);
/// ```
///
/// Only traits in the table above are supported at all.
///
/// # Example
///
/// ```
/// use snippets::{impl_language, Language};
///
/// pub struct CustomLanguage;
///
/// impl Language for CustomLanguage {
///     const NAME: &'static str = "example";
///     const STRATEGY: snippets::Strategy = snippets::Strategy::Static;
/// }
///
/// impl_language!(CustomLanguage);
///
/// assert_eq!(format!("{}", CustomLanguage), format!("{}", CustomLanguage::display()));
/// assert_eq!(format!("{:?}", CustomLanguage), format!("{}", CustomLanguage));
/// ```
///
/// # Customization
///
/// Users may still customize the display of a language:
/// ```
/// use snippets::{impl_language, Language};
///
/// pub struct CustomLanguage;
///
/// impl Language for CustomLanguage {
///     const NAME: &'static str = "example";
///     const STRATEGY: snippets::Strategy = snippets::Strategy::Static;
///
///     fn display() -> &'static str {
///         "custom name"
///     }
/// }
///
/// impl_language!(CustomLanguage);
///
/// assert_eq!(&format!("{}", CustomLanguage), "custom name");
/// assert_eq!(&format!("{:?}", CustomLanguage), "custom name");
/// ```
#[macro_export]
macro_rules! impl_language {
    ($language:ty => Debug) => {
        impl std::fmt::Debug for $language {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", <Self as $crate::Language>::display())
            }
        }
    };
    ($language:ty => Display) => {
        impl std::fmt::Display for $language {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", <Self as $crate::Language>::display())
            }
        }
    };
    ($language:ty) => {
        $crate::impl_language!($language => Display);
        $crate::impl_language!($language => Debug);
    };

}

/// Many programming languages include compile-time metaprogramming,
/// for example C and C++ have [preprocessing macros],
/// Rust has [multiple types of macros],
/// Haskell has [Template Haskell], and more.
///
/// This type allows a [`Language`], specified for an [`Extractor`],
/// to advertise the kind of parsing strategy it employs to parse the language.
///
/// [preprocessing macros]: https://gcc.gnu.org/onlinedocs/cpp/Macros.html
/// [multiple types of macros]: https://doc.rust-lang.org/book/ch19-06-macros.html
/// [Template Haskell]: http://wiki.haskell.org/Template_Haskell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
#[strum(serialize_all = "snake_case")]
#[non_exhaustive]
pub enum Strategy {
    /// The extractor statically analyzes the code.
    /// No compile time metaprogramming is evaluated.
    Static,
}

/// An extracted snippet from the given unit of source code.
#[derive(Debug, Clone, Getters, CopyGetters, Index, Deref, Derivative)]
#[derivative(PartialOrd, Ord, PartialEq, Eq)]
pub struct Snippet<L> {
    /// Metadata for the extracted snippet.
    #[getset(get_copy = "pub")]
    metadata: Metadata,

    /// The bytes of the snippet fingerprint.
    #[index]
    #[deref]
    #[getset(get = "pub")]
    #[derivative(PartialOrd = "ignore", Ord = "ignore")]
    fingerprint: text::Buffer,

    /// Used to disambiguate snippets by source language.
    ///
    /// Technically this is evaluated for ordering and equality,
    /// but `PhantomData<T>` is always equal to itself for both checks.
    language: PhantomData<L>,
}

impl<L> Snippet<L> {
    /// Create a new snippet from the provided data.
    pub fn from(meta: Metadata, content: impl AsRef<[u8]>) -> Self {
        text::fingerprint(&content).pipe(|fp| Self::new(meta, fp))
    }

    /// Create a new instance from the provided information.
    pub fn new(metadata: Metadata, fingerprint: text::Buffer) -> Self {
        Self {
            metadata,
            fingerprint,
            language: PhantomData,
        }
    }
}

impl<L: Language> std::fmt::Display for Snippet<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", L::display(), self.metadata)
    }
}

/// The metadata for an extracted snippet.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, CopyGetters, Constructor)]
#[getset(get_copy = "pub")]
pub struct Metadata {
    /// The kind of item this snippet represents.
    kind: Kind,

    /// The method used to extract this snippet.
    method: Method,

    /// The location at which the snippet was found.
    location: Location,
}

impl std::fmt::Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.kind, self.method, self.location)
    }
}

/// The location in the unit of source code from which the snippet was extracted.
///
/// After opening the file (so a hypothetical reader is at byte offset `0`),
/// the reader then skips a number of bytes equal to `byte_offset`,
/// then reads a number of bytes equal to `byte_len`.
/// The bytes that were read compose the entire snippet.
///
/// For example, given the file:
/// ```not_rust
/// #include <stdio.h>
///
/// int main() {
///   printf("hello world\n");
///   return 0;
/// }
/// ```
///
/// In the representation the computer sees, it looks like this (using `⏎` to represent a newline):
/// ```not_rust
/// #include <stdio.h>⏎⏎int main() {⏎  printf("hello world\n");⏎  return 0;⏎}⏎
/// ^^^^                ^        ^
/// 0123...             20 <-9-> 29
/// ```
///
/// The [`Location`] below represents the `int main()` snippet in that example:
/// ```
/// # // ⏎ is a multi-byte symbol, so use an empty space for demonstration instead.
/// # let example = "#include <stdio.h>  int main() {}";
/// # use snippets::*;
/// let location = Location::builder().byte_offset(20).byte_len(10).build();
///
/// let range = location.as_range();
/// let snippet = &example.as_bytes()[range];
///
/// let got = std::str::from_utf8(snippet)?;
/// assert_eq!(got, "int main()");
/// # Ok::<(), std::str::Utf8Error>(())
/// ```
//
// Note: we use a `TypedBuilder` instead of a `Constructor` here because this way we can accept
// a standard `usize` for each argument while still making it very clear in-code
// which argument is which.
//
// Basically, the intent is to straddle the line between newtype convenience and newtype safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, CopyGetters, TypedBuilder)]
#[getset(get_copy = "pub")]
pub struct Location {
    /// The byte offset at which the snippet began.
    #[builder(setter(transform = |input: usize| ByteOffset(input)))]
    byte_offset: ByteOffset,

    /// The number of bytes to read for the snippet from the file.
    #[builder(setter(transform = |input: usize| ByteLen(input)))]
    byte_len: ByteLen,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..={}", self.start_byte(), self.end_byte())
    }
}

impl Location {
    /// Read a [`Location`] as a range, intended to be used to index a buffer of bytes.
    pub fn as_range(&self) -> std::ops::Range<usize> {
        let start = self.byte_offset.0;
        let len = self.byte_len.0;
        let end = start + len;
        start..end
    }

    /// The index of the first byte indicated for the provided location.
    pub fn start_byte(&self) -> usize {
        self.as_range().start
    }

    /// The index of the last byte indicated for the provided location.
    pub fn end_byte(&self) -> usize {
        let end = self.as_range().end;
        if end == 0 {
            0
        } else {
            end - 1 // as_range is not inclusive, so the last byte _to be read_ is less one.
        }
    }

    /// Extract the bytes indicated by a [`Location`] from a buffer.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let example = "#include <stdio.h>  int main() {}";
    /// let location = Location::builder().byte_offset(20).byte_len(10).build();
    ///
    /// let got = location.extract_from(example.as_bytes());
    /// assert_eq!(got, b"int main()");
    /// ```
    pub fn extract_from<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        &buf[self.as_range()]
    }

    /// Extract the bytes indicated by a [`Location`] from a buffer,
    /// into a lossily converted [`String`].
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let example = "#include <stdio.h>  int main() {}";
    /// let location = Location::builder().byte_offset(20).byte_len(10).build();
    ///
    /// let got = location.extract_from_lossy(example.as_bytes());
    /// assert_eq!(got, "int main()");
    /// ```
    pub fn extract_from_lossy<'a>(&self, buf: &'a [u8]) -> Cow<'a, str> {
        let bytes = self.extract_from(buf);
        String::from_utf8_lossy(bytes)
    }
}

impl From<Range<usize>> for Location {
    fn from(value: Range<usize>) -> Self {
        let start = value.start;
        let end = value.end;
        Self {
            byte_offset: ByteOffset(start),
            byte_len: ByteLen(end - start),
        }
    }
}

impl From<RangeInclusive<usize>> for Location {
    fn from(value: RangeInclusive<usize>) -> Self {
        let start = *value.start();
        let end = *value.end() + 1;
        Self {
            byte_offset: ByteOffset(start),
            byte_len: ByteLen(end - start),
        }
    }
}

/// The byte offset at which the snippet began.
///
/// Zero-based, meaning that if the snippet begins on the first byte of the file,
/// this offset is `0`.
///
/// Think of the offset as
/// "the number of bytes to skip from the start of the file to when this snippet begins".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub struct ByteOffset(usize);

/// The number of bytes to read for the snippet from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub struct ByteLen(usize);

flags! {
    /// The kind of item this snippet represents.
    ///
    /// # Specificity order
    ///
    /// Specificity is in the order specified by the implementation of [`Ord`] for this type,
    /// meaning that a [`Kind::Full`] variant is considered a more exact match
    /// than a [`Kind::Body`] variant, which is a more exact match
    /// than a [`Kind::Signature`] variant.
    ///
    /// Items with higher "specificity order" are sorted _higher_; meaning that a
    /// [`Kind::Full`] variant would be sorted later in a vector
    /// than a [`Kind::Signature`] variant:
    ///
    /// ```
    /// # use snippets::*;
    /// assert!(Kind::Full > Kind::Body);
    /// assert!(Kind::Body > Kind::Signature);
    /// ```
    #[derive(Hash, PartialOrd, Ord, EnumIter, Display)]
    #[strum(serialize_all = "snake_case")]
    #[non_exhaustive]
    pub enum Kind: u8 {
        /// The signature of the function.
        ///
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String            // <- included
        /// {                                                      // <- omitted
        ///   println!("Happy birthday! You're {age} years old!"); // <- omitted
        /// }                                                      // <- omitted
        /// ```
        Signature,

        /// The body of the function.
        ///
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String {          // <- omitted
        ///   println!("Happy birthday! You're {age} years old!"); // <- included
        /// }                                                      // <- omitted
        /// ```
        Body,

        /// Both signature and body.
        ///
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String {          // <- included
        ///   println!("Happy birthday! You're {age} years old!"); // <- included
        /// }                                                      // <- included
        /// ```
        Full,
    }
}

/// The kinds of snippet to extract.
///
/// # Examples
///
/// Single [`Kind`] in the set:
/// ```
/// # use snippets::*;
/// let kinds = Kinds::from(Kind::Signature);
/// assert!(kinds.contains(Kind::Signature));
/// ```
///
/// Multiple [`Kind`]s in the set:
/// ```
/// # use snippets::*;
/// let kinds = Kinds::from(Kind::Signature | Kind::Body);
/// assert!(kinds.contains(Kind::Signature));
/// assert!(kinds.contains(Kind::Body));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Kinds(FlagSet<Kind>);

impl Kinds {
    /// Check whether a given [`Kind`] is in the set.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let kinds = Kinds::from(Kind::Signature | Kind::Body);
    /// assert!(kinds.contains(Kind::Signature));
    /// assert!(kinds.contains(Kind::Body));
    /// ```
    pub fn contains(&self, kind: Kind) -> bool {
        self.0.contains(kind)
    }

    /// Check whether the set is empty.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let kinds = Kinds::from(Kind::Signature | Kind::Body);
    /// assert!(!kinds.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Create a new set of all [`Kind`]s.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let kinds = Kinds::full();
    /// assert!(kinds.contains(Kind::Signature));
    /// assert!(kinds.contains(Kind::Body));
    /// ```
    pub fn full() -> Self {
        Self(FlagSet::full())
    }

    /// Iterate over the [`Kind`]s in the set.
    pub fn iter(&self) -> impl Iterator<Item = Kind> + Clone {
        self.0.into_iter()
    }
}

impl From<FlagSet<Kind>> for Kinds {
    fn from(value: FlagSet<Kind>) -> Self {
        Self(value)
    }
}

impl From<Kind> for Kinds {
    fn from(value: Kind) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for Kinds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kinds = self
            .iter()
            .sorted_unstable()
            .map(|t| t.to_string())
            .collect_vec()
            .join(",");
        write!(f, "{kinds}")
    }
}

/// The method used to extract this snippet.
///
/// # Specificity order
///
/// Specificity is in the order specified by the implementation of [`Ord`] for this type,
/// meaning that a [`Method::Raw`] variant is considered a more exact match
/// than a [`Method::Normalized`] variant.
///
/// Items with higher "specificity order" are sorted _higher_; meaning that a
/// [`Method::Raw`] variant would be sorted later in a vector
/// than a [`Method::Normalized`] variant:
///
/// ```
/// # use snippets::*;
/// # let arbitrary_flagset = Transforms::from(Transform::Space);
/// assert!(Method::Raw > Method::Normalized(arbitrary_flagset));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Method {
    /// Generated from the text with the specified normalizations applied.
    Normalized(Transforms),

    /// Generated from the text as written.
    ///
    /// ```ignore
    /// fn say_happy_birthday(age: usize) -> String {
    ///   // TODO: make 'years' smart plural.
    ///   println!("Happy birthday! You're {age} years old!");
    /// }
    /// ```
    Raw,
}

impl Method {
    /// Create an iterator over possible methods to use for snippet extraction,
    /// given the provided [`Transforms`].
    ///
    /// If the provided set is empty, this is equivalent to [`std::iter::once`] over [`Method::Raw`].
    pub fn iter(transforms: Transforms) -> impl Iterator<Item = Method> + Clone {
        // Implement with `Vec` so that the types for each branch line up.
        // Since each branch uses a macro to construct an appropriately-sized `Vec`,
        // this is no worse performance than e.g. `iter::once().chain(iter::once())`.
        if transforms.is_empty() {
            vec![Method::Raw]
        } else {
            vec![Method::Raw, Method::Normalized(transforms)]
        }
        .into_iter()
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Normalized(transforms) => write!(f, "normalized({transforms})"),
            Method::Raw => write!(f, "raw"),
        }
    }
}

flags! {
    /// The normalization used to extract this snippet.
    ///
    /// # Specificity order
    ///
    /// Specificity is in the order specified by the implementation of [`Ord`] for this type,
    /// meaning that a [`Transform::Space`] variant is considered a more exact match
    /// than a [`Transform::Comment`] variant.
    ///
    /// Items with higher "specificity order" are sorted _higher_; meaning that a
    /// [`Transform::Space`] variant would be sorted later in a vector
    /// than a [`Transform::Comment`] variant:
    ///
    /// ```
    /// # use snippets::*;
    /// assert!(Transform::Space > Transform::Comment);
    /// ```
    #[derive(Hash, PartialOrd, Ord, EnumIter, Display)]
    #[strum(serialize_all = "snake_case")]
    #[non_exhaustive]
    pub enum Transform: u8 {
        /// Generated with any comments removed. Exactly what constitutes a comment is up to the implementation
        /// of the [`Extractor`] for the language being analyzed.
        ///
        /// # Example
        ///
        /// The original input:
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String {
        ///   // TODO: make 'years' smart plural.
        ///   println!("Happy birthday! You're {age} years old!");
        /// }
        /// ```
        ///
        /// Is normalized to this:
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String {
        ///   println!("Happy birthday! You're {age} years old!");
        /// }
        /// ```
        Comment,

        /// Generated with any whitespace characters (including newlines) normalized to a single space.
        /// Contiguous spaces are also collapsed to a single space. The specific test for whether
        /// a character is considered "whitespace" is the Unicode property `White_Space=yes`.
        ///
        /// # Example
        ///
        /// The original input:
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String {
        ///   // TODO: make 'years' smart plural.
        ///   println!("Happy birthday! You're {age} years old!");
        /// }
        /// ```
        ///
        /// Is normalized to this:
        /// ```ignore
        /// fn say_happy_birthday(age: usize) -> String { // TODO: make 'years' smart plural. println!("Happy birthday! You're {age} years old!"); }
        /// ```
        Space,
    }
}

impl Transform {
    /// Scores each variant on its specificity order.
    ///
    /// Implemented manually for now, if we ever get lots of variant churn we can look into a macro.
    /// The specific scores chosen should just ensure that combinations of normalizations compare
    /// as desired to other combinations.
    ///
    /// This is by its nature inexact: there's not always a good way to obviously map
    /// "A+B+C+D is better than B+C+E+F", but we'll do the best we can.
    ///
    /// Scores that are truly equivalent may be given equivalent scores.
    fn score(self) -> usize {
        match self {
            Transform::Comment => 1,
            Transform::Space => 2,
        }
    }
}

/// The normalizations used to extract this snippet.
///
/// # Specificity order
///
/// As discussed on [`Transform`], flags are already ordered by specificity, such that higher
/// specificity flags are ordered later in a collection.
///
/// For [`Transforms`] (this type), it's a little different. Since the goal of "specificity order"
/// is to sort snippets higher that are _less modified_ from the original source code,
/// this type is ordered such that:
/// - If there is a single [`Transform`] in compared sets, they're sorted as usual.
/// - If there are multiple [`Transform`]s in compared sets, sets with fewer members are higher specificity.
/// - For sets with the same number of members, sets are compared with the scores of their composite variants.
///
/// This works because in this case "more flags" means "more normalizations applied to the source",
/// which means that they are _less_ specific. Meanwhile, if the count of flags are equal,
/// the flags used can be extracted and compared by their score.
///
/// To give examples using [`Transform`]
/// (assume a third pretend variant "Other" and pretend it is lower specificity than "Comment"):
/// - `[Space] > [Comment]`: same as standalone.
/// - `[Comment] > [Space,Comment]`: fewer modifications.
/// - `[Space,Comment] > [Comment,Other]`: the score of "Space+Comment" is higher than "Comment+Other".
///
/// Scores are set based on the specificity of the variant.
/// For example, [`Transform::Comment`] is scored `1`, as the lowest specificity;
/// meanwhile [`Transform::Space`] is scored `2` as the next lowest specificity,
/// and so on.
/// Specific score values are not meaningful other than as a non-durable comparison to one another.
///
/// # Application order
///
/// The order that combinations of flags are applied matters: for instance, note the example for
/// [`Transform::Space`] creates a syntax error, and the entire function body
/// would be removed if it was performed before [`Transform::Comment`].
///
/// The application order is therefore up to the implementation of [`Extractor`];
/// users are only able to specify which normalizations are performed.
///
/// It also follows that implementations of [`Extractor`] do not necessarily obey
/// the specificity order of normalizations when applying normalizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Transforms(FlagSet<Transform>);

impl Transforms {
    /// Scores each variant in self on its specificity order,
    /// returning the summed score and the count of applied normalizations.
    fn score_count(self) -> (usize, usize) {
        self.0
            .into_iter()
            .map(Transform::score)
            .fold((0, 0), |(prev, len), score| (prev + score, len + 1))
    }

    /// Check whether a given [`Transform`] is in the set.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let transforms = Transforms::from(Transform::Space | Transform::Comment);
    /// assert!(transforms.contains(Transform::Space));
    /// assert!(transforms.contains(Transform::Comment));
    /// ```
    pub fn contains(&self, transform: Transform) -> bool {
        self.0.contains(transform)
    }

    /// Check whether the set is empty.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let transforms = Transforms::from(Transform::Space | Transform::Comment);
    /// assert!(!transforms.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Create a new set of all [`Transform`]s.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let transforms = Transforms::full();
    /// assert!(transforms.contains(Transform::Space));
    /// assert!(transforms.contains(Transform::Comment));
    /// ```
    pub fn full() -> Self {
        Self(FlagSet::full())
    }

    /// Create a new set with no [`Transform`]s.
    ///
    /// # Example
    ///
    /// ```
    /// # use snippets::*;
    /// let transforms = Transforms::none();
    /// assert!(!transforms.contains(Transform::Space));
    /// assert!(!transforms.contains(Transform::Comment));
    /// ```
    pub fn none() -> Self {
        Self(FlagSet::default())
    }

    /// Iterate over the [`Transform`]s in the set.
    pub fn iter(&self) -> impl Iterator<Item = Transform> + Clone {
        self.0.into_iter()
    }
}

impl std::fmt::Display for Transforms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transforms = self
            .iter()
            .sorted_unstable()
            .map(|t| t.to_string())
            .collect_vec()
            .join(",");
        write!(f, "{transforms}")
    }
}

impl PartialOrd for Transforms {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Transforms {
    fn cmp(&self, other: &Self) -> Ordering {
        let (self_score, self_len) = self.score_count();
        let (other_score, other_len) = other.score_count();
        match self_len.cmp(&other_len) {
            // Fewer normalizations in set; higher specificity.
            Ordering::Less => Ordering::Greater,
            // More normalizations in set; lower specificity.
            Ordering::Greater => Ordering::Less,
            // Equal count of normalizations in set; order by score.
            Ordering::Equal => self_score.cmp(&other_score),
        }
    }
}

impl From<FlagSet<Transform>> for Transforms {
    fn from(value: FlagSet<Transform>) -> Self {
        Self(value)
    }
}

impl From<Transform> for Transforms {
    fn from(value: Transform) -> Self {
        Self(value.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn specificity_order_kind() {
        let mut input = vec![Kind::Body, Kind::Signature, Kind::Full];
        input.sort_unstable();
        assert_eq!(input, vec![Kind::Signature, Kind::Body, Kind::Full]);
    }

    #[test]
    fn specificity_order_method() {
        let arbitrary_flags = Transforms(Transform::Space | Transform::Comment);
        let mut input = vec![Method::Raw, Method::Normalized(arbitrary_flags)];
        input.sort_unstable();

        let expected = vec![Method::Normalized(arbitrary_flags), Method::Raw];
        assert_eq!(input, expected);
    }

    #[test]
    fn specificity_order_normalization() {
        let mut input = vec![Transform::Space, Transform::Comment];
        input.sort_unstable();
        assert_eq!(input, vec![Transform::Comment, Transform::Space]);
    }

    #[test]
    fn specificity_order_normalizations() {
        let mut input = vec![
            Transforms(FlagSet::from(Transform::Comment)),
            Transforms(Transform::Space | Transform::Comment),
            Transforms(FlagSet::from(Transform::Space)),
        ];
        let expected = vec![
            Transforms(Transform::Space | Transform::Comment),
            Transforms(FlagSet::from(Transform::Comment)),
            Transforms(FlagSet::from(Transform::Space)),
        ];

        input.sort_unstable();
        assert_eq!(input, expected);
    }

    #[test]
    fn slice_offset() -> Result<(), std::str::Utf8Error> {
        let example = "#include <stdio.h>  int main() {}";
        let location = Location::builder().byte_offset(20).byte_len(10).build();

        let range = location.as_range();
        let snippet = &example.as_bytes()[range];
        let got = std::str::from_utf8(snippet)?;
        assert_eq!(got, "int main()");

        Ok(())
    }

    #[test]
    fn normalizations_count() {
        let scores = [
            (FlagSet::from(Transform::Comment), 1),
            (FlagSet::from(Transform::Space), 1),
            (Transform::Comment | Transform::Space, 2),
        ];
        for (set, expected) in scores {
            let (_, len) = Transforms(set).score_count();
            assert_eq!(len, expected, "set: {set:?}");
        }
    }

    #[test]
    fn normalizations_score() {
        let scores = [
            (FlagSet::from(Transform::Comment), 1),
            (FlagSet::from(Transform::Space), 2),
            (Transform::Comment | Transform::Space, 3),
        ];
        for (set, expected) in scores {
            let (score, _) = Transforms(set).score_count();
            assert_eq!(score, expected, "set: {set:?}");
        }
    }

    #[test]
    fn location_round_trip() {
        let location = Location {
            byte_offset: ByteOffset(20),
            byte_len: ByteLen(10),
        };
        let inclusive_range = 20..=29;
        let range = 20..30;

        assert_eq!(location.as_range(), range);
        assert_eq!(Location::from(inclusive_range), location);
        assert_eq!(Location::from(range), location);
    }

    #[test]
    fn location_bytes_round_trip() {
        let location = Location {
            byte_offset: ByteOffset(10),
            byte_len: ByteLen(10),
        };

        let input = "0123456789helloworld_abcdefghijk";
        assert_eq!(location.extract_from(input.as_bytes()), b"helloworld");

        let range = location.start_byte()..=location.end_byte();
        let roundtrip = Location::from(range);
        assert_eq!(roundtrip, location);
        assert_eq!(roundtrip.extract_from(input.as_bytes()), b"helloworld");

        let range = location.as_range();
        let roundtrip = Location::from(range);
        assert_eq!(roundtrip, location);
        assert_eq!(roundtrip.extract_from(input.as_bytes()), b"helloworld");
    }

    #[test]
    fn location_extract() {
        let input = "0123456789helloworld0123456789";
        let location = Location {
            byte_offset: ByteOffset(10),
            byte_len: ByteLen(10),
        };

        assert_eq!(location.extract_from(input.as_bytes()), b"helloworld");
        assert_eq!(location.start_byte(), 10);
        assert_eq!(location.end_byte(), 19);
    }

    #[test]
    fn location_extract_end() {
        let input = "0123456789helloworld";
        let location = Location {
            byte_offset: ByteOffset(10),
            byte_len: ByteLen(10),
        };

        assert_eq!(location.extract_from(input.as_bytes()), b"helloworld");
    }
}
