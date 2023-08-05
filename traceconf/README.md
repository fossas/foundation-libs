# traceconf

Provides a standardized interface for configuring for `tracing` in FOSSA programs.

## Quick start

Use this library inside `main` like this:

```rust

/// Example for how to utilize `traceconf` in another program.
#[derive(Debug, Parser)]
#[clap(version)]
struct Opts {
    /// Flatten the trace config into these options.
    /// This way the user sees these as top-level options to configure like any other argument.
    #[clap(flatten)]
    tracing: traceconf::TracingConfig,
}

fn example_main() {
    // Parse the options.
    let opts = Opts::parse();

    // Configure tracing.
    // Note: If your program must be very performant, this implementation is not ideal;
    // see the interior function comments for more details.
    let subscriber = opts.tracing.subscriber();
    tracing::subscriber::set_global_default(subscriber).expect("set global trace subscriber");

    // Now do the rest of the work you wanted to do in `main`.
    info!("Now I am tracing my program!");
}
```

Users then see help text like this:
```not_rust
Example for how to utilize `traceconf` in another program

Usage: traceconf [OPTIONS] <COMMAND>

Commands:
  help    Print this message or the help of the given subcommand(s)

Options:
      --trace-level <TRACE_LEVEL>
          Set the minimum level for logs. Logs below this level are dropped

          [default: info]

          Possible values:
          - off:   Do not emit events
          - error: Emit events that are 'Error' level or higher
          - warn:  Emit events that are 'Warn' level or higher
          - info:  Emit events that are 'Info' level or higher
          - debug: Emit events that are 'Debug' level or higher
          - trace: Emit events that are 'Trace' level or higher

      --trace-spans <TRACE_SPANS>
          Enable span traces in logs. Span traces report on units of work performed by the program

          [default: off]

          Possible values:
          - off:    Do not log span traces
          - new:    One event when the span is created
          - enter:  One event when the span begins executing
          - exit:   One event when the span suspends or finishes executing
          - close:  One event when the span context is dropped after fully executing
          - active: Combination of 'Enter' and 'Exit' events
          - full:   Combination of all events

      --trace-format <TRACE_FORMAT>
          The formatter to use for log and span traces

          [default: text]

          Possible values:
          - text: Output text formatted logs and traces for humans
          - json: Output JSON formatted logs and traces for machines

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

For more details, see crate documentation.
`main.rs` also contains a slightly more complicated example.

