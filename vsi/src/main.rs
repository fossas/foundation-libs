//! The canonical client binary for running VSI scans.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use clap::{Parser, Subcommand};
use log::{debug, info, Level};
use stable_eyre::{
    eyre::{ensure, Context},
    Result,
};
use stderrlog::ColorChoice;
use vsi::{
    self,
    api::{Devnull, Fossa},
    config, scan,
};

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Cmd {
    /// The scan mode for the client.
    #[clap(subcommand)]
    mode: Mode,
}

impl Cmd {
    fn validate(self) -> Result<Self> {
        Ok(Cmd {
            mode: match self.mode {
                Mode::Partial(opts) => Mode::Partial(opts.validate()?),
                Mode::Full(opts) => Mode::Full(opts.validate()?),
                Mode::DryRun(opts) => Mode::DryRun(opts.validate()?),
            },
        })
    }
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Run in partial mode.
    ///
    /// Uses an existing scan ID and uploads files in the project to that scan ID.
    /// Does not complete the scan nor wait for forensics to complete.
    Partial(CmdPartial),

    /// Run in full mode.
    ///
    /// Creates the scan, uploads files in the project, completes the scan, and waits for forensics to complete.
    Full(CmdFull),

    /// Run in "dry run" mode.
    ///
    /// Does not communicate with the VSI Forensics Service at all; instead it just logs what it would have communicated.
    /// Equivalent to full mode, just sans communication.
    DryRun(CmdDryRun),
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdPartial {
    #[clap(flatten)]
    scan: config::Scan,

    #[clap(flatten)]
    api: config::Api,

    /// The scan ID to use for appending the new file scans.
    #[clap(long)]
    scan_id: String,
}

impl CmdPartial {
    fn validate(self) -> Result<Self> {
        ensure!(!self.scan_id.is_empty(), "Scan ID must not be empty");
        Ok(Self {
            scan: self.scan.validate()?,
            api: self.api.validate()?,
            scan_id: self.scan_id,
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdFull {
    #[clap(flatten)]
    scan: config::Scan,

    #[clap(flatten)]
    api: config::Api,

    #[clap(flatten)]
    display: config::Display,
}

impl CmdFull {
    fn validate(self) -> Result<Self> {
        Ok(Self {
            scan: self.scan.validate()?,
            api: self.api.validate()?,
            ..self
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdDryRun {
    #[clap(flatten)]
    scan: config::Scan,

    #[clap(flatten)]
    display: config::Display,
}

impl CmdDryRun {
    fn validate(self) -> Result<Self> {
        Ok(Self {
            scan: self.scan.validate()?,
            ..self
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    stable_eyre::install()?;

    let cmd = Cmd::parse().validate()?;
    match cmd.mode {
        Mode::Partial(opts) => main_partial(opts).await,
        Mode::Full(opts) => main_full(opts).await,
        Mode::DryRun(opts) => main_dryrun(opts).await,
    }
}

async fn main_partial(CmdPartial { scan, api, scan_id }: CmdPartial) -> Result<()> {
    init_logging(&scan)?;
    debug!("running in partial mode");

    let client = Fossa::new(&api, &scan).context("create client")?;
    let opts = scan::Options::builder().root(scan.dir()).build();
    let id = scan::Id::from(scan_id);

    debug!("scanning partial artifacts into scan {id} with options: {opts:?}");
    scan::artifacts(&client, &id, opts)
        .await
        .context("scan artifacts")?;

    Ok(())
}

async fn main_full(CmdFull { scan, api, display }: CmdFull) -> Result<()> {
    init_logging(&scan)?;
    debug!("running in full mode");

    let client = Fossa::new(&api, &scan).context("create client")?;
    let result = vsi::run(client, scan, display).await.context("run scan")?;
    println!("{result}");
    Ok(())
}

async fn main_dryrun(CmdDryRun { scan, display }: CmdDryRun) -> Result<()> {
    init_logging(&scan)?;
    info!("running in dry run mode");

    let client = Devnull::new();
    let result = vsi::run(client, scan, display).await.context("run scan")?;
    println!("{result}");
    Ok(())
}

/// Configures the global logger for the application based on self.
fn init_logging(scan: &config::Scan) -> Result<()> {
    stderrlog::new()
        .module(module_path!())
        .color(ColorChoice::Never)
        .verbosity(if scan.debug() {
            Level::Debug
        } else {
            Level::Info
        })
        .init()?;
    Ok(())
}
