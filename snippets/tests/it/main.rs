//! Entry point for integration tests.
//!
//! Note: `cargo` "integration tests"
//! mean "tests as the library consumer uses the library",
//! not e.g. "tests using remote resources".
//!
//! # Debugging fingerprinting
//!
//! Fingerprint tests _should_ include a call to [`tracing::setup`].
//! This then configures the test to output tracing data to the terminal,
//! which can be debugged by running `cargo test` in the terminal with a `RUST_LOG`
//! setting. For details, see [filtering events with environment variables].
//!
/// [filtering events with environment variables]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables
pub mod language;
mod tracing;

/// Compare two snippets for equality in context.
///
/// Exists primarily because, since snippets use byte buffers,
/// comparing them by the default `assert_eq` is really painful to debug.
#[macro_export]
macro_rules! assert_snippet_eq {
    ($content:expr => $a:expr, $b:expr) => {{
        use snippets::debugging::ToDisplayEscaped;

        let content_a = $a.metadata().location().extract_from($content);
        let content_b = $b.metadata().location().extract_from($content);

        let text_a = format!("{}", content_a.display_escaped());
        let text_b = format!("{}", content_b.display_escaped());
        assert_eq!($a.metadata(), $b.metadata(), "'{text_a}' == '{text_b}'");
        assert_eq!(text_a, text_b, "escaped input texts");

        let fp_a = format!("{}", $a.fingerprint());
        let fp_b = format!("{}", $b.fingerprint());
        assert_eq!(fp_a, fp_b, "'{text_a}' == '{text_b}'");

        // This should always pass at this point but included as a catch-all
        // and to backwards-infer types.
        assert_eq!($a, $b, "'{text_a}' == '{text_b}'");
    }};
}

/// Compare two iterators of snippets for equality.
#[macro_export]
macro_rules! assert_snippets_eq {
    ($content:expr => $a:expr, $b:expr) => {{
        for (a, b) in itertools::zip_eq($a, $b) {
            $crate::assert_snippet_eq!($content => a, b);
        }
    }};
}

/// Include the contents of the file at the provided path, normalizing `\r\n` to `\n`.
#[macro_export]
macro_rules! include_str_lf {
    ($path:expr) => {
        include_str!($path).replace("\r\n", "\n")
    };
}
