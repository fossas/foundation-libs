use insta::assert_snapshot;
use strum::IntoEnumIterator;
use xshell::{cmd, Shell};

#[test]
fn generated_help_snapshot() {
    let sh = Shell::new().expect("create shell");
    let output = cmd!(sh, "cargo run -q --bin traceconf -- --help")
        .read()
        .expect("must have run");

    assert_snapshot!(output);
}

#[test]
fn generated_help_short_snapshot() {
    let sh = Shell::new().expect("create shell");
    let output = cmd!(sh, "cargo run -q --bin traceconf -- -h")
        .read()
        .expect("must have run");

    assert_snapshot!(output);
}

#[test]
#[ignore = "prompts SIGKILL in macOS CI, I think due to memory pressure"]
fn debug_output_format_text() {
    let sh = Shell::new().expect("create shell");
    let filters = vec![
        (r"[0-9\-]+T[0-9:]+\.\d{6}Z", "{timestamp}"),
        (r"=\d+(\.\d+)?(µs|ms)", "={timespan}"),
    ];

    // Test that every combination of (format, level, span)
    // renders as expected when tested with `debug-output-format`.
    // `format` is split into two functions primarily to improve parallelism under nextest.
    let format = traceconf::Format::Text.to_string();
    for level in traceconf::Level::iter() {
        for spans in traceconf::Span::iter().map(to_string) {
            // If level is not info, no span information is logged
            // (since spans are logged at info level in `debug-output-format`).
            // If level is off, nothing is logged.
            // Give things that should be the same output the same assertion name.

            let snapshot_name = match level {
                traceconf::Level::Off => String::from("debug_output_format_disabled"),
                _ if level >= traceconf::Level::Info => {
                    format!("debug_output_format_{format}_{level}_{spans}")
                }
                _ => format!("debug_output_format_{format}_{level}"),
            };

            let level = level.to_string();
            let output = cmd!(sh, "cargo run -q --bin traceconf -- debug-output-format --trace-colors disable --trace-format {format} --trace-level {level} --trace-spans {spans}")
                    .read_stderr()
                    .expect("must have run");

            insta::with_settings!(
                { filters => filters.clone() },
                { assert_snapshot!(snapshot_name, output); }
            );
        }
    }
}

#[test]
#[ignore = "prompts SIGKILL in macOS CI, I think due to memory pressure"]
fn debug_output_format_json() {
    let sh = Shell::new().expect("create shell");
    let filters = vec![
        (r"[0-9\-]+T[0-9:]+\.\d{6}Z", "<timestamp>"),
        (r"\d+(\.\d+)?(µs|ms)", "<timespan>"),
    ];

    // Test that every combination of (format, level, span)
    // renders as expected when tested with `debug-output-format`.
    // `format` is split into two functions primarily to improve parallelism under nextest.
    let format = traceconf::Format::Json.to_string();
    for level in traceconf::Level::iter() {
        for spans in traceconf::Span::iter().map(to_string) {
            // If level is not info, no span information is logged
            // (since spans are logged at info level in `debug-output-format`).
            // If level is off, nothing is logged.
            // Give things that should be the same output the same assertion name.

            let snapshot_name = match level {
                traceconf::Level::Off => String::from("debug_output_format_disabled"),
                _ if level >= traceconf::Level::Info => {
                    format!("debug_output_format_{format}_{level}_{spans}")
                }
                _ => format!("debug_output_format_{format}_{level}"),
            };

            let level = level.to_string();
            let output = cmd!(sh, "cargo run -q --bin traceconf -- debug-output-format --trace-colors disable --trace-format {format} --trace-level {level} --trace-spans {spans}")
                    .read_stderr()
                    .expect("must have run");

            insta::with_settings!(
                { filters => filters.clone() },
                { assert_snapshot!(snapshot_name, output); }
            );
        }
    }
}

#[test]
#[ignore = "prompts SIGKILL in macOS CI, I think due to memory pressure"]
fn debug_output_format_text_colors() {
    let sh = Shell::new().expect("create shell");
    let filters = vec![
        (r"[0-9\-]+T[0-9:]+\.\d{6}Z", "{timestamp}"),
        (r"\d+(\.\d+)?(µs|ms)", "{timespan}"),
    ];

    let format = traceconf::Format::Text.to_string();
    let level = traceconf::Level::Trace.to_string();
    let spans = traceconf::Span::Full.to_string();
    let colors = traceconf::Colors::Enable.to_string();
    let output = cmd!(sh, "cargo run -q --bin traceconf -- debug-output-format --trace-colors {colors} --trace-format {format} --trace-level {level} --trace-spans {spans}")
                    .read_stderr()
                    .expect("must have run");

    insta::with_settings!(
        { filters => filters.clone() },
        { assert_snapshot!(output); });
}

#[test]
#[ignore = "prompts SIGKILL in macOS CI, I think due to memory pressure"]
fn debug_output_format_json_colors() {
    let sh = Shell::new().expect("create shell");
    let filters = vec![
        (r"[0-9\-]+T[0-9:]+\.\d{6}Z", "<timestamp>"),
        (r"\d+(\.\d+)?(µs|ms)", "<timespan>"),
    ];

    let format = traceconf::Format::Json.to_string();
    let level = traceconf::Level::Trace.to_string();
    let spans = traceconf::Span::Full.to_string();

    // JSON format should not output colors ever,
    // so all the color option output should match.
    for colors in traceconf::Colors::iter().map(to_string) {
        let output = cmd!(sh, "cargo run -q --bin traceconf -- debug-output-format --trace-colors {colors} --trace-format {format} --trace-level {level} --trace-spans {spans}")
                        .read_stderr()
                        .expect("must have run");

        insta::with_settings!(
            { filters => filters.clone() },
            { assert_snapshot!("debug_output_format_json_colors", output); });
    }
}

fn to_string<T: ToString>(item: T) -> String {
    item.to_string()
}
