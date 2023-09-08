use std::sync::Once;

/// Configure a [`tracing_subscriber`] with provided env settings.
/// For details, see [filtering events with environment variables].
///
/// # Example Usage
///
/// ```ignore
/// RUST_LOG=debug cargo test
/// ```
///
/// [filtering events with environment variables]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables
#[track_caller]
pub fn setup() {
    static SETUP: Once = Once::new();
    SETUP.call_once(tracing_subscriber::fmt::init);
}
