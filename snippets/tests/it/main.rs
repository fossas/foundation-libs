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

/// Include the contents of the file at the provided path, normalizing `\r\n` to `\n`.
#[macro_export]
macro_rules! include_str_lf {
    ($path:expr) => {
        include_str!($path).replace("\r\n", "\n")
    };
}
