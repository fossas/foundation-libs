//! Provides types and helpers for parsing JSON-formatted [`tracing_subscriber`] output for Rust programs.

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

/// A single entry in the trace file.
///
/// Variants are split into their own structs
/// instead of embedded in the enum directly
/// so that they can be more ergonomically used.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Entry {
    /// Spans recorded in the file.
    Span(SpanEntry),

    /// Log messages recorded in the file.
    Log(LogEntry),
}

/// The shape of an [`Entry::Log`].
#[derive(Debug, Deserialize, Serialize, Getters, CopyGetters)]
pub struct LogEntry {
    /// Common fields used in all variants of [`Entry`].
    #[getset(get = "pub")]
    #[serde(flatten)]
    common: CommonEntry,

    /// The fields of log-specific entries.
    #[getset(get = "pub")]
    fields: LogFields,
}

/// The shape of an [`Entry::Span`].
#[derive(Debug, Deserialize, Serialize, Getters, CopyGetters)]
pub struct SpanEntry {
    /// Common fields used in all variants of [`Entry`].
    #[getset(get = "pub")]
    #[serde(flatten)]
    common: CommonEntry,

    /// The fields of span-specific entries.
    #[getset(get_copy = "pub")]
    fields: SpanFields,
}

/// Common fields used in all variants of [`Entry`].
#[derive(Debug, Deserialize, Serialize, Getters, CopyGetters)]
pub struct CommonEntry {
    /// The time stamp for when the entry was emitted.
    #[getset(get_copy = "pub")]
    #[serde(with = "time::serde::iso8601")]
    timestamp: OffsetDateTime,

    /// The entry target.
    #[getset(get = "pub")]
    target: String,

    /// Not all [`traceconf::Level`] values are actually represented:
    /// in particular [`traceconf::Level::Off`] will never be read out of a trace file.
    ///
    /// This type is used strictly for convenience for now. As we productionize
    /// this tool we should make a more appropriate standalone type.
    #[getset(get_copy = "pub")]
    level: traceconf::Level,

    /// Raw span values, minimally parsed.
    ///
    /// This type is used strictly for convenience for now. As we productionize
    /// this tool we should make a more appropriate and concrete standalone type.
    #[getset(get = "pub")]
    span: Value,

    /// Raw span ancestry, minimally parsed.
    ///
    /// This type is used strictly for convenience for now. As we productionize
    /// this tool we should make a more appropriate and concrete standalone type.
    #[getset(get = "pub")]
    spans: Value,
}

/// The `fields` entry for a span.
///
/// Basically the same as for [`LogFields`],
/// but the message is predefined.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct SpanFields {
    /// Not all [`traceconf::Span`] values are actually represented:
    /// in particular the values that are combinations of other values
    /// (e.g. [`traceconf::Span::Active`] and [`traceconf::Span::Full`])
    /// will never be read out of a trace file.
    ///
    /// This type is used strictly for convenience for now. As we productionize
    /// this tool we should make a more appropriate standalone type.
    message: traceconf::Span,
}

/// The `fields` entry for a log message.
#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct LogFields {
    message: String,
}
