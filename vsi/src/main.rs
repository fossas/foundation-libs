//! The canonical client binary for running VSI scans.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use log::{debug, info, Level};
use stable_eyre::{
    eyre::{ensure, Context},
    Result,
};
use stderrlog::ColorChoice;
use vsi::{
    api::{Client, Devnull, Fossa},
    scan,
};

#[tokio::main]
async fn main() -> Result<()> {
    stable_eyre::install()?;

    let opts = Cmd::parse().validate()?;
    match opts.mode {
        Mode::Partial(opts) => main_partial(opts).await,
        Mode::Full(opts) => main_full(opts).await,
        Mode::DryRun(opts) => main_dryrun(opts).await,
    }
}

async fn main_partial(CmdPartial { opts, scan_id }: CmdPartial) -> Result<()> {
    opts.init_logging()?;
    debug!("running in partial mode");

    let client = Fossa::new(&opts.endpoint, &opts.fossa_api_key);
    let opts = scan::Options::builder().root(opts.dir).build();

    debug!("scanning partial artifacts into scan {scan_id} with options: {opts:?}");
    scan::artifacts(&client, &scan::Id::from(scan_id), opts)
        .await
        .context("scan artifacts")?;

    Ok(())
}

async fn main_full(CmdFull { opts, export }: CmdFull) -> Result<()> {
    opts.init_logging()?;
    debug!("running in full mode");

    let client = Fossa::new(&opts.endpoint, &opts.fossa_api_key);
    let id = client.create_scan().await.context("create scan")?;
    info!("created scan: {id}");

    let opts = scan::Options::builder().root(opts.dir).build();
    debug!("scanning artifacts with options: {opts:?}");
    scan::artifacts(&client, &id, opts)
        .await
        .context("scan artifacts")?;

    debug!("waiting for forensics");
    client
        .wait_forensics(&id)
        .await
        .context("wait for forensics")?;

    debug!("forensics complete");
    match export {
        Export::ScanID => println!("{{ scan_id: {id} }}"),
        Export::Locators => {
            debug!("downloading results");
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

async fn main_dryrun(CmdDryRun { opts, export }: CmdDryRun) -> Result<()> {
    opts.init_logging()?;
    info!("running in dry run mode");

    let client = Devnull::new();
    let id = client.create_scan().await.context("create scan")?;
    info!("created scan: {id}");

    let opts = scan::Options::builder().root(opts.dir).build();
    info!("scanning artifacts with options: {opts:?}");
    scan::artifacts(&client, &id, opts)
        .await
        .context("scan artifacts")?;

    info!("waiting for forensics");
    client
        .wait_forensics(&id)
        .await
        .context("wait for forensics")?;

    info!("forensics complete");
    match export {
        Export::ScanID => println!("{{ scan_id: {id} }}"),
        Export::Locators => {
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

/// Determines which scan data is exported.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum Export {
    /// Exports the scan ID after all files have been uploaded.
    ///
    /// When running in partial mode, the scan ID is exported immediately after the scan completes without waiting for forensics.
    /// When running in full mode, the scan ID is exported after forensics complete.
    ScanID,

    /// Exports the list of locators determined as dependencies for the project after forensics have run.
    ///
    /// Conflicts with the partial scan mode: it is impossible to export locators for a partial scan.
    Locators,
}

/// Determines the format used for exporting scan data.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum Format {
    /// JSON output format.
    Json,
}

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

#[derive(Parser, Debug)]
struct Opts {
    /// The FOSSA endpoint to which the scans are uploaded.
    #[clap(long, default_value = "https://app.fossa.com")]
    endpoint: String,

    /// The FOSSA API key. Also available via the `FOSSA_API_KEY` environment variable.
    #[clap(long, env = "FOSSA_API_KEY")]
    fossa_api_key: String,

    /// The directory to fingerprint.
    #[clap()]
    dir: PathBuf,

    /// The data format used to export scan data.
    #[clap(long, default_value_t = Format::Json)]
    #[arg(value_enum)]
    format: Format,

    /// Paths provided here are included.
    ///
    /// Exclusion takes precedence: if a path is both excluded and included, it is excluded.
    /// This rule holds recursively; if a parent is excluded, included children are still excluded.
    #[clap(long)]
    only_paths: Vec<PathBuf>,

    /// Paths provided here are not included.
    ///
    /// Exclusion takes precedence: if a path is both excluded and included, it is excluded.
    /// This rule holds recursively; if a parent is excluded, included children are still excluded.
    #[clap(long)]
    exclude_paths: Vec<PathBuf>,

    /// Whether to enable debug logging.
    #[clap(long, short)]
    debug: bool,
}

impl Opts {
    fn validate(self) -> Result<Self> {
        ensure!(
            !self.fossa_api_key.is_empty(),
            "FOSSA API key must not be empty"
        );

        let dir = self.dir.canonicalize().context("canonicalize target dir")?;
        Ok(Self { dir, ..self })
    }

    fn init_logging(&self) -> Result<()> {
        Ok(stderrlog::new()
            .module(module_path!())
            .color(ColorChoice::Auto)
            .verbosity(if self.debug {
                Level::Debug
            } else {
                Level::Info
            })
            .init()?)
    }
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdPartial {
    #[clap(flatten)]
    opts: Opts,

    /// The scan ID to use for appending the new file scans.
    #[clap(long)]
    scan_id: String,
}

impl CmdPartial {
    fn validate(self) -> Result<Self> {
        ensure!(!self.scan_id.is_empty(), "Scan ID must not be empty");
        Ok(Self {
            opts: self.opts.validate()?,
            scan_id: self.scan_id,
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdFull {
    #[clap(flatten)]
    opts: Opts,

    /// The data that should be exported.
    #[clap(long, default_value_t = Export::Locators)]
    #[arg(value_enum)]
    export: Export,
}

impl CmdFull {
    fn validate(self) -> Result<Self> {
        Ok(Self {
            opts: self.opts.validate()?,
            export: self.export,
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version, about)]
struct CmdDryRun {
    #[clap(flatten)]
    opts: Opts,

    /// The data that should be exported.
    #[clap(long, default_value_t = Export::Locators)]
    #[arg(value_enum)]
    export: Export,
}

impl CmdDryRun {
    fn validate(self) -> Result<Self> {
        Ok(Self {
            opts: self.opts.validate()?,
            export: self.export,
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
