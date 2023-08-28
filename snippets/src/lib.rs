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
//! Features are most commonly used to enable support for languages.
//!
//! Name | Description | Kind
//! ---|---|---
//! `all-languages` | Enables all features that are of the kind "Language" | Language
//! `c` | Enables support for C | Language
//! `cpp` | Enables support for C++ | Language
//!
//! # Examples
//!
//! (TODO)

use std::{cmp::Ordering, io::Read};

use derivative::Derivative;
use derive_more::{Deref, Index};
use fallible_iterator::FallibleIterator;
use flagset::{flags, FlagSet};
use getset::{CopyGetters, Getters};
use strum::{Display, EnumIter};
use thiserror::Error;
use typed_builder::TypedBuilder;

/// Errors reported by [`Extractor`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {}

/// An implementation of [`Extractor`] enables snippets to be extracted
/// from a given unit of source code (typically a file).
pub trait Extractor {
    /// When the extractor implementation reports its support for a given code unit,
    /// this is the type that is reported.
    ///
    /// This is implemented as an associated type so that extractors
    /// are able to provide their own support states.
    ///
    /// However most extractors are encouraged to use the standard [`Support`] type if possible,
    /// and indeed the custom type must be able to translate to the standard type
    /// in order to work with the rest of the library.
    type Support: std::fmt::Debug + Into<Support>;

    /// Reports the support status for extractor in regards to the provided unit of source code.
    ///
    /// # Reader
    ///
    /// The [`Read`] instance provided to `source` may be partially or fully consumed during this process.
    fn support<R: Read>(source: R) -> Result<Self::Support, Error>;

    /// Reads the provided unit of source code for snippets, according to the provided options.
    ///
    /// # Reader
    ///
    /// The [`Read`] instance provided to `source` may be partially or fully consumed during this process.
    /// If the reader was previously read (partially or fully, by example via [`Extractor::support`]),
    /// it almost definitely needs to be reset to the initial point before using this method.
    fn extract<R: Read, I: FallibleIterator<Item = Snippet, Error = Error>>(source: R) -> I;
}

/// An implementation of [`Extractor`] may support source code to varying extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
#[strum(serialize_all = "snake_case")]
#[non_exhaustive]
pub enum Support {
    /// The [`Extractor`] implementation fully supports the provided unit of source code.
    Full,

    /// The [`Extractor`] implementation partially supports the provided unit of source code.
    ///
    /// The specific meaning of "partial support" is up to the implementation. Check its documentation for more details.
    Partial,

    /// The [`Extractor`] implementation does not support the provided unit of source code.
    None,
}

/// An extracted snippet from the given unit of source code.
#[derive(Debug, Clone, PartialEq, Eq, Getters, CopyGetters, Index, Deref, Derivative)]
#[derivative(PartialOrd, Ord)]
pub struct Snippet {
    /// The raw bytes of the snippet content.
    #[getset(get = "pub")]
    #[index]
    #[deref]
    #[derivative(PartialOrd = "ignore", Ord = "ignore")]
    bytes: Vec<u8>,

    /// Metadata for the extracted snippet.
    #[getset(get_copy = "pub")]
    metadata: Metadata,
}

/// The metadata for an extracted snippet.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct Metadata {
    /// The kind of item this snippet represents.
    kind: Kind,

    /// The method used to generate this snippet.
    method: Method,

    /// The location at which the snippet was found.
    location: Location,
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
/// let location = Location::builder().byte_offset(20).byte_len(9).build();
///
/// let range = location.as_range();
/// let snippet = &example.as_bytes()[range];
///
/// let got = std::str::from_utf8(snippet)?;
/// assert_eq!(got, "int main()");
/// # Ok::<(), std::str::Utf8Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, CopyGetters, TypedBuilder)]
#[getset(get_copy = "pub")]
pub struct Location {
    /// The byte offset at which the snippet began.
    #[builder(setter(transform = |input: u64| ByteOffset(input)))]
    byte_offset: ByteOffset,

    /// The number of bytes to read for the snippet from the file.
    #[builder(setter(transform = |input: u64| ByteLen(input)))]
    byte_len: ByteLen,
}

impl Location {
    /// Read a [`Location`] as a range, intended to be used to index a `Vec<u8>` or `&[u8]`.
    pub fn as_range(&self) -> std::ops::RangeInclusive<usize> {
        let len = self.byte_len.0 as usize;
        let start = self.byte_offset.0 as usize;
        let end = start + len;
        start..=end
    }
}

/// The byte offset at which the snippet began.
///
/// Zero-based, meaning that if the snippet begins on the first byte of the file,
/// this offset is `0`.
///
/// Think of the offset as
/// "the number of bytes to skip from the start of the file to when this snippet begins".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteOffset(u64);

/// The number of bytes to read for the snippet from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteLen(u64);

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, EnumIter)]
#[non_exhaustive]
pub enum Kind {
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

/// The method used to generate this snippet.
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

flags! {
    /// The normalization used to generate this snippet.
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
    #[derive(Hash, PartialOrd, Ord, EnumIter)]
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

/// The normalizations used to generate this snippet.
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
        let location = Location::builder().byte_offset(20).byte_len(9).build();

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
}
