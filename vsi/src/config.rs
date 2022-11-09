//! Provides application configuration structures.

use std::path::PathBuf;

use clap::Parser;
use getset::{CopyGetters, Getters};
use stable_eyre::{
    eyre::{ensure, Context},
    Result,
};
use typed_builder::TypedBuilder;

/// Determines which scan data is exported.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum Export {
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
pub enum Format {
    /// JSON output format.
    Json,
}

/// Configures API related information.
#[derive(Parser, Debug, Getters, CopyGetters, TypedBuilder)]
pub struct Api {
    /// The FOSSA endpoint to which the scans are uploaded.
    #[clap(long, default_value = "https://app.fossa.com")]
    #[getset(get = "pub")]
    #[builder(default = String::from("https://app.fossa.com"), setter(into))]
    endpoint: String,

    /// The FOSSA API key. Also available via the `FOSSA_API_KEY` environment variable.
    #[clap(long = "fossa-api-key", env = "FOSSA_API_KEY")]
    #[getset(get = "pub")]
    #[builder(setter(into))]
    key: String,

    /// The FOSSA organization to which the scan belongs.
    ///
    /// This is not important from a security perspective; it's meant to be used for statistical purposes.
    #[clap(long, default_value_t = 1, env = "FOSSA_ORG_ID")]
    #[getset(get_copy = "pub")]
    organization_id: usize,
}

impl Api {
    /// Validates that self is correctly formed.
    pub fn validate(self) -> Result<Self> {
        ensure!(!self.key.is_empty(), "FOSSA API key must not be empty");
        Ok(self)
    }
}

/// Configures how scan results are displayed.
#[derive(Parser, Debug, Getters, TypedBuilder)]
#[getset(get = "pub")]
pub struct Display {
    /// The data format used to export scan data.
    #[clap(long, default_value_t = Format::Json)]
    #[arg(value_enum)]
    #[builder(default = Format::Json)]
    format: Format,

    /// The data that should be exported.
    #[clap(long, default_value_t = Export::Locators)]
    #[arg(value_enum)]
    #[builder(default = Export::Locators)]
    export: Export,
}

impl Default for Display {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Configures options related to the scan.
#[derive(Parser, Debug, Getters, CopyGetters, TypedBuilder)]
pub struct Scan {
    /// Whether to enable debug logging.
    #[clap(long, short)]
    #[getset(get_copy = "pub")]
    #[builder(default = false)]
    debug: bool,

    /// The directory to fingerprint.
    #[clap()]
    #[getset(get = "pub")]
    #[builder(setter(into))]
    dir: PathBuf,

    /// Paths provided here are included.
    ///
    /// Exclusion takes precedence: if a path is both excluded and included, it is excluded.
    /// This rule holds recursively; if a parent is excluded, included children are still excluded.
    #[clap(long)]
    #[getset(get = "pub")]
    #[builder(default)]
    only_paths: Vec<PathBuf>,

    /// Paths provided here are not included.
    ///
    /// Exclusion takes precedence: if a path is both excluded and included, it is excluded.
    /// This rule holds recursively; if a parent is excluded, included children are still excluded.
    #[clap(long)]
    #[getset(get = "pub")]
    #[builder(default)]
    exclude_paths: Vec<PathBuf>,
}

impl Scan {
    /// Validates that self is correctly formed.
    pub fn validate(self) -> Result<Self> {
        let dir = self.dir.canonicalize().context("canonicalize target dir")?;
        Ok(Self { dir, ..self })
    }
}
