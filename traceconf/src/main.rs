//! Example binary showing how to utilize `traceconf` in another program.
//!
//! Examples:
//! ```not_rust
//! ❯ cargo run --bin traceconf -- hello --name jess --trace-spans full
//!    Compiling traceconf v0.1.0 (/Users/jessica/projects/foundation-libs/traceconf)
//!    Finished dev [unoptimized + debuginfo] target(s) in 0.37s
//!     Running `target/debug/traceconf hello --name jess --trace-spans full`
//! 2023-08-03T19:49:53.387462Z  INFO main_hello{name="jess"}: traceconf: new
//! 2023-08-03T19:49:53.387543Z  INFO main_hello{name="jess"}: traceconf: enter
//! 2023-08-03T19:49:53.387572Z  INFO main_hello{name="jess"}: traceconf: Hello, jess!
//! 2023-08-03T19:49:53.387600Z  INFO main_hello{name="jess"}: traceconf: exit
//! 2023-08-03T19:49:53.387621Z  INFO main_hello{name="jess"}: traceconf: close time.busy=56.0µs time.idle=104µs
//! ```
//!
//! ```not_rust
//! ❯ cargo run --bin traceconf -- debug-output-format                 
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.08s
//!      Running `target/debug/traceconf debug-output-format`
//! 2023-08-03T19:51:11.875071Z  INFO traceconf::debug_output_format: demonstrating how messages are logged.
//! 2023-08-03T19:51:11.875144Z  INFO traceconf::debug_output_format:
//! 2023-08-03T19:51:11.875158Z ERROR traceconf::debug_output_format: message at level 'Error'
//! 2023-08-03T19:51:11.875170Z  INFO traceconf::debug_output_format: message at level 'Info'
//! 2023-08-03T19:51:11.875182Z  WARN traceconf::debug_output_format: message at level 'Warn'
//! 2023-08-03T19:51:11.875193Z  INFO traceconf::debug_output_format:
//! 2023-08-03T19:51:11.875203Z  INFO traceconf::debug_output_format: -------
//! 2023-08-03T19:51:11.875214Z  INFO traceconf::debug_output_format: demonstrating how spans are logged.
//! 2023-08-03T19:51:11.875224Z  INFO traceconf::debug_output_format: calling 'uppercase' with the argument 'hello', then logging the result:
//! 2023-08-03T19:51:11.875236Z  INFO traceconf::debug_output_format:
//! 2023-08-03T19:51:11.875257Z  INFO traceconf::debug_output_format: the word 'hello' uppercased is...
//! 2023-08-03T19:51:11.875399Z  INFO traceconf::debug_output_format: > 'HELLO'
//! 2023-08-03T19:51:11.875413Z  INFO traceconf::debug_output_format:
//! 2023-08-03T19:51:11.875423Z  INFO traceconf::debug_output_format: -------
//! 2023-08-03T19:51:11.875433Z  INFO traceconf::debug_output_format: demonstrating how nested spans are logged.
//! 2023-08-03T19:51:11.875444Z  INFO traceconf::debug_output_format: calling the function 'inner' with an argument, which will in turn call 'nested_inner' with a different argument.
//! 2023-08-03T19:51:11.875456Z  INFO traceconf::debug_output_format:
//! 2023-08-03T19:51:11.875479Z ERROR inner{value="some value passed to 'inner'"}: traceconf::debug_output_format: message at level 'Error' inside function 'inner'
//! 2023-08-03T19:51:11.875508Z  INFO inner{value="some value passed to 'inner'"}: traceconf::debug_output_format: message at level 'Info' inside function 'inner'
//! 2023-08-03T19:51:11.875610Z  WARN inner{value="some value passed to 'inner'"}: traceconf::debug_output_format: message at level 'Warn' inside function 'inner'
//! 2023-08-03T19:51:11.875648Z ERROR inner{value="some value passed to 'inner'"}:nested_inner{value="some value passed to 'nested_inner'"}: traceconf::debug_output_format: message at level 'Error' inside function 'nested_inner_function', called from inside 'inner'
//! 2023-08-03T19:51:11.875680Z  INFO inner{value="some value passed to 'inner'"}:nested_inner{value="some value passed to 'nested_inner'"}: traceconf::debug_output_format: message at level 'Info' inside function 'nested_inner_function', called from inside 'inner'
//! 2023-08-03T19:51:11.875848Z  WARN inner{value="some value passed to 'inner'"}:nested_inner{value="some value passed to 'nested_inner'"}: traceconf::debug_output_format: message at level 'Warn' inside function 'nested_inner_function', called from inside 'inner'
//! ```

use clap::{Parser, Subcommand};
use tracing::info;

/// Example for how to utilize `traceconf` in another program.
#[derive(Debug, Parser)]
#[clap(version)]
struct Opts {
    /// Flatten the trace config into these options.
    /// This way the user sees these as top-level options to configure like any other argument.
    #[clap(flatten)]
    tracing: traceconf::TracingConfig,

    /// Run a number of subcommands.
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Clone, Debug, Subcommand)]
enum Commands {
    /// Say hello.
    ///
    /// This is just an example to show how a user can have other subcommands too.
    Hello(HelloArgs),

    /// Showcase how output is formatted so that users can see what effects formatting arguments have.
    DebugOutputFormat,
}

/// Arguments used by the "walk" command.
#[derive(Debug, Clone, Parser)]
pub struct HelloArgs {
    /// To whom should we say hello?
    #[clap(long)]
    name: String,
}

fn main() {
    // Parse the options.
    let opts = Opts::parse();

    // Configure tracing.
    // Note: If your program must be very performant, this implementation is not ideal;
    // see the interior function comments for more details.
    let subscriber = opts.tracing.subscriber();
    tracing::subscriber::set_global_default(subscriber).expect("set global trace subscriber");

    match opts.command {
        Commands::Hello(args) => main_hello(args),
        Commands::DebugOutputFormat => traceconf::debug_output_format(),
    }
}

#[tracing::instrument]
fn main_hello(HelloArgs { name }: HelloArgs) {
    info!("Hello, {name}!");
}
