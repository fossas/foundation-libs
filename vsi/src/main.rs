//! The canonical client binary for running VSI scans.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use std::time::Duration;

use clap::{Parser, Subcommand};
use log::{debug, info, Level};
use stable_eyre::{
    eyre::{bail, ensure, Context},
    Result,
};
use stderrlog::ColorChoice;
use tokio::time::sleep;
use vsi::{
    api::{Client, Devnull, Fossa},
    config, forensics, scan,
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

    debug!("scanning partial artifacts into scan {scan_id} with options: {opts:?}");
    scan::artifacts(&client, &scan::Id::from(scan_id), opts)
        .await
        .context("scan artifacts")?;

    Ok(())
}

async fn main_full(CmdFull { scan, api, display }: CmdFull) -> Result<()> {
    init_logging(&scan)?;
    debug!("running in full mode");

    let client = Fossa::new(&api, &scan).context("create client")?;
    run_full_scan(client, scan, display)
        .await
        .context("run scan")
}

async fn main_dryrun(CmdDryRun { scan, display }: CmdDryRun) -> Result<()> {
    init_logging(&scan)?;
    info!("running in dry run mode");

    let client = Devnull::new();
    run_full_scan(client, scan, display)
        .await
        .context("run scan")
}

async fn run_full_scan(
    client: impl Client + Sync,
    scan: config::Scan,
    display: config::Display,
) -> Result<()> {
    let id = client.create_scan().await.context("create scan")?;
    info!("created scan: {id}");

    let opts = scan::Options::builder().root(scan.dir()).build();
    info!("scanning artifacts with options: {opts:?}");
    scan::artifacts(&client, &id, opts)
        .await
        .context("scan artifacts")?;

    info!("complete scan");
    client
        .complete_scan(&id)
        .await
        .context("mark scan complete")?;

    info!("waiting for forensics");
    wait_forensics(&client, &id)
        .await
        .context("wait for forensics")?;

    info!("forensics complete");
    match display.export() {
        config::Export::ScanID => println!("{{ scan_id: {id} }}"),
        config::Export::Locators => {
            info!("downloading results");
            let results = client
                .download_forensics(&id)
                .await
                .context("download forensics")?;

            let encoded = serde_json::to_string(&results).context("render results")?;
            println!("{encoded}");
        }
    }

    Ok(())
}

/// Waits for forensics to complete or error.
async fn wait_forensics(client: &impl Client, id: &scan::Id) -> Result<()> {
    let delay = Duration::from_secs(1);
    let mut last_status: Option<forensics::Status> = None;
    loop {
        let status = client
            .forensics_status(id)
            .await
            .context("get forensics status")?;

        if let Some(last_status) = &last_status {
            if last_status == &status {
                sleep(delay).await;
                continue;
            }
        }

        match status {
            forensics::Status::Pending => {
                info!("forensic analysis is enqueued, waiting to start...")
            }
            forensics::Status::Finished => {
                info!("forensic analysis complete");
                return Ok(());
            }
            forensics::Status::Failed => {
                bail!("forensic analysis failed");
            }
            forensics::Status::Informational(ref step) => {
                info!("forensic analysis step: {step}");
            }
        }

        last_status = Some(status);
    }
}

/// Configures the global logger for the application based on self.
fn init_logging(scan: &config::Scan) -> Result<()> {
    Ok(stderrlog::new()
        .module(module_path!())
        .color(ColorChoice::Auto)
        .verbosity(if *scan.debug() {
            Level::Debug
        } else {
            Level::Info
        })
        .init()?)
}
