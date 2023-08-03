//! Provides a standardized interface for configuring for `tracing` in FOSSA programs.
//!
//! # Quick start
//!
//! Use this library inside `main` like this:
//!
//! ```
//! # use clap::Parser;
//! # use tracing::info;
//!
//! /// Example for how to utilize `traceconf` in another program.
//! #[derive(Debug, Parser)]
//! #[clap(version)]
//! struct Opts {
//!     /// Flatten the trace config into these options.
//!     /// This way the user sees these as top-level options to configure like any other argument.
//!     #[clap(flatten)]
//!     tracing: traceconf::TracingConfig,
//! }
//!
//! fn example_main() {
//!     // Parse the options.
//!     let opts = Opts::parse();
//!
//!     // Configure tracing.
//!     // Note: If your program must be very performant, this implementation is not ideal;
//!     // see the interior function comments for more details.
//!     let subscriber = opts.tracing.subscriber();
//!     tracing::subscriber::set_global_default(subscriber).expect("set global trace subscriber");
//!
//!     // Now do the rest of the work you wanted to do in `main`.
//!     info!("Now I am tracing my program!");
//! }
//! ```
//!
//! Users then see help text like this:
//! ```not_rust
//! Example for how to utilize `traceconf` in another program
//!
//! Usage: traceconf [OPTIONS] <COMMAND>
//!
//! Commands:
//!   help    Print this message or the help of the given subcommand(s)
//!
//! Options:
//!       --trace-level <TRACE_LEVEL>
//!           Set the minimum level for logs. Logs below this level are dropped
//!           
//!           [default: info]
//!
//!           Possible values:
//!           - off:   Do not emit events
//!           - error: Emit events that are 'Error' level or higher
//!           - warn:  Emit events that are 'Warn' level or higher
//!           - info:  Emit events that are 'Info' level or higher
//!           - debug: Emit events that are 'Debug' level or higher
//!           - trace: Emit events that are 'Trace' level or higher
//!
//!       --trace-spans <TRACE_SPANS>
//!           Enable span traces in logs. Span traces report on units of work performed by the program
//!           
//!           [default: off]
//!
//!           Possible values:
//!           - off:    Do not log span traces
//!           - new:    One event when the span is created
//!           - enter:  One event when the span begins executing
//!           - exit:   One event when the span suspends or finishes executing
//!           - close:  One event when the span context is dropped after fully executing
//!           - active: Combination of 'Enter' and 'Exit' events
//!           - full:   Combination of all events
//!
//!       --trace-format <TRACE_FORMAT>
//!           The formatter to use for log and span traces
//!           
//!           [default: text]
//!
//!           Possible values:
//!           - text: Output text formatted logs and traces for humans
//!           - json: Output JSON formatted logs and traces for machines
//!
//!   -h, --help
//!           Print help (see a summary with '-h')
//!
//!   -V, --version
//!           Print version
//! ```
//!
//! For more details, see crate documentation.
//! `main.rs` also contains a slightly more complicated example.
//!

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use std::io::{self, Write};

use clap::{Parser, ValueEnum};
use getset::CopyGetters;
use strum::{Display, EnumIter};
use tracing::{metadata::LevelFilter, Subscriber};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, Layer, Registry};

mod debug_output_format;

pub use debug_output_format::run as debug_output_format;

/// Convenience user-facing configuration for [`tracing`] and [`tracing_subscriber`].
///
/// It's possible to embed this struct directly with `#[clap(flatten)]`.
/// If you do so, ensure that you have documentation set on your main argument structure,
/// because otherwise these docs (attached to [`TracingConfig`]) will be
/// used for your main program's documentation by Clap.
///
/// This struct does make some assumptions about these arguments,
/// especially their name, defaults, and that they are assumed to be global.
///
/// If this is not desired, embed the types directly into your arguments structure,
/// using this struct as an example only. If you create your own argument structure,
/// you can use the `subscriber` method as a reference for how to use the arguments.
///
/// You can also use each type's `Default` implementation for the defaults in this crate.
#[derive(Debug, Parser, CopyGetters)]
#[clap(version)]
#[getset(get_copy = "pub")]
pub struct TracingConfig {
    /// Set the minimum level for logs. Logs below this level are dropped.
    #[clap(long, global = true, default_value_t = Level::default())]
    trace_level: Level,

    /// Enable span traces in logs. Span traces report on units of work performed by the program.
    #[clap(long, global = true, default_value_t = Span::default())]
    trace_spans: Span,

    /// The formatter to use for log and span traces.
    #[clap(long, global = true, default_value_t = Format::default())]
    trace_format: Format,

    /// The coloring mode to use for log and span traces.
    #[clap(long, global = true, default_value_t = Colors::default())]
    trace_colors: Colors,
}

impl TracingConfig {
    /// Translate the level selected by the user into the format used by [`tracing`].
    pub fn level_filter(&self) -> LevelFilter {
        self.trace_level.into()
    }

    /// Translate the span strategy selected by the user into the format used by [`tracing_subscriber`].
    pub fn fmt_span(&self) -> FmtSpan {
        self.trace_spans.into()
    }

    /// Configure a [`Subscriber`] implementation for these options.
    ///
    /// Note: If your program must be very performant,
    /// this implementation is not ideal;
    /// see the interior function comments for more details.
    pub fn subscriber(&self) -> impl Subscriber {
        // Since this is in a library, this function is a little more complicated
        // (and a little less performant) than it would be if it was implemented in a program.
        //
        // Specifically, a program can just set up a single format layer, since it doesn't
        // have to make sure that it always returns one concrete type. This function however
        // has to set up both format layers, and just write one to the sink writer (throwing its output away).
        // This is so that the returned types always line up regardless of configuration.
        //
        // This is fine for _most_ programs, and especially for getting up and running quickly,
        // but is not ideal if the program must be very performant because it formats statements
        // just to throw them away, and additionally uses dynamic dispatch to invoke the writer.
        //
        // If your program must be very performant, prefer to construct your `Registry` with only one layer
        // (or even skip using `Registry` entirely, but that's beyond the scope of this comment).
        //
        // For example, you can place something like the below into your `main` function:
        // ```
        // match opts.format() {
        //     Format::Text => tracing::subscriber::set_global_default(
        //         Registry::default().with(
        //             tracing_subscriber::fmt::layer()
        //                 .with_file(false)
        //                 .with_line_number(false)
        //                 .with_span_events(opts.fmt_span())
        //                 .with_filter(opts.level_filter()),
        //         ),
        //     ),
        //     Format::Json => tracing::subscriber::set_global_default(
        //         Registry::default().with(
        //             tracing_subscriber::fmt::layer()
        //                 .json()
        //                 .with_span_events(opts.fmt_span())
        //                 .with_filter(opts.level_filter()),
        //         ),
        //     ),
        // }
        // .expect("must set global tracing subscriber");
        // ```
        let configured_format = self.trace_format;
        let writer_for = move |format: Format| -> Box<dyn Write> {
            if configured_format == format {
                Box::new(io::stderr())
            } else {
                Box::new(io::sink())
            }
        };

        Registry::default()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(self.trace_colors == Colors::Enable)
                    .with_writer(move || writer_for(Format::Text))
                    .with_file(false)
                    .with_line_number(false)
                    .with_span_events(self.fmt_span())
                    .with_filter(self.level_filter()),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(self.trace_colors == Colors::Enable)
                    .with_writer(move || writer_for(Format::Json))
                    .with_span_events(self.fmt_span())
                    .with_filter(self.level_filter()),
            )
    }
}

/// The log formatting to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Parser, ValueEnum, Display, EnumIter)]
pub enum Format {
    /// Output text formatted logs and traces for humans.
    #[strum(serialize = "text")]
    Text,

    /// Output JSON formatted logs and traces for machines.
    #[strum(serialize = "json")]
    Json,
}

impl Default for Format {
    fn default() -> Self {
        Self::Text
    }
}

/// The terminal color modes to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Parser, ValueEnum, Display, EnumIter)]
pub enum Colors {
    /// Output text with coloring.
    #[strum(serialize = "enable")]
    Enable,

    /// Output text without coloring.
    #[strum(serialize = "disable")]
    Disable,
}

impl Default for Colors {
    fn default() -> Self {
        Self::Enable
    }
}

/// The minimum level to output.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Parser, ValueEnum, Display, EnumIter,
)]
pub enum Level {
    /// Do not emit events.
    #[strum(serialize = "off")]
    Off,

    /// Emit events that are 'Error' level or higher.
    #[strum(serialize = "error")]
    Error,

    /// Emit events that are 'Warn' level or higher.
    #[strum(serialize = "warn")]
    Warn,

    /// Emit events that are 'Info' level or higher.
    #[strum(serialize = "info")]
    Info,

    /// Emit events that are 'Debug' level or higher.
    #[strum(serialize = "debug")]
    Debug,

    /// Emit events that are 'Trace' level or higher.
    #[strum(serialize = "trace")]
    Trace,
}

impl Default for Level {
    fn default() -> Self {
        Self::Info
    }
}

impl From<Level> for LevelFilter {
    fn from(value: Level) -> Self {
        match value {
            Level::Off => LevelFilter::OFF,
            Level::Error => LevelFilter::ERROR,
            Level::Info => LevelFilter::INFO,
            Level::Warn => LevelFilter::WARN,
            Level::Debug => LevelFilter::DEBUG,
            Level::Trace => LevelFilter::TRACE,
        }
    }
}

/// Which parts of span traces to output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Parser, ValueEnum, Display, EnumIter)]
pub enum Span {
    /// Do not log span traces.
    #[strum(serialize = "off")]
    Off,

    /// One event when the span is created.
    #[strum(serialize = "new")]
    New,

    /// One event when the span begins executing.
    #[strum(serialize = "enter")]
    Enter,

    /// One event when the span suspends or finishes executing.
    #[strum(serialize = "exit")]
    Exit,

    /// One event when the span context is dropped after fully executing.
    #[strum(serialize = "close")]
    Close,

    /// Combination of 'Enter' and 'Exit' events.
    #[strum(serialize = "active")]
    Active,

    /// Combination of all events.
    #[strum(serialize = "full")]
    Full,
}

impl Default for Span {
    fn default() -> Self {
        Self::Off
    }
}

impl From<Span> for FmtSpan {
    fn from(val: Span) -> Self {
        match val {
            Span::New => FmtSpan::NEW,
            Span::Enter => FmtSpan::ENTER,
            Span::Exit => FmtSpan::EXIT,
            Span::Close => FmtSpan::CLOSE,
            Span::Off => FmtSpan::NONE,
            Span::Active => FmtSpan::ACTIVE,
            Span::Full => FmtSpan::FULL,
        }
    }
}
