use std::process::Command;

use log::{info, Level};
use stable_eyre::{
    eyre::{ensure, Context},
    Result,
};
use stderrlog::ColorChoice;
use tempfile::TempDir;
use tokio::task::spawn_blocking;
use vsi::{
    api::{Devnull, Fossa},
    config,
};

const REFERENCE_VSI_DEMO_URL: &str = "https://github.com/fossas/cpp-vsi-demo";

/// Clone the reference VSI repo.
///
/// TODO: Clone this into a central location for use in each test instead of cloning it in each test.
pub async fn clone_vsi_example() -> Result<TempDir> {
    let dir = spawn_blocking(TempDir::new)
        .await
        .context("join worker")?
        .context("create temp dir")?;

    let path = dir.path().to_owned();
    let output = spawn_blocking(move || {
        Command::new("git")
            .args(["clone", REFERENCE_VSI_DEMO_URL])
            .current_dir(path)
            .output()
    })
    .await
    .context("join worker")?
    .context("run git clone")?;

    ensure!(output.status.success(), "clone failed: {output:?}");
    Ok(dir)
}

/// Run a scan with the provided configs, outputting its result according to the configuration.
pub async fn run(api: config::Api, scan: config::Scan, display: config::Display) -> Result<String> {
    init_logging(&scan)?;
    info!("running in full mode");

    let client = Fossa::new(&api, &scan).context("create client")?;
    vsi::run(client, scan, display).await.context("run scan")
}

/// Run a dry scan with the provided configs, outputting its result according to the configuration.
pub async fn run_dry(scan: config::Scan, display: config::Display) -> Result<String> {
    init_logging(&scan)?;
    info!("running in dry run mode");
    let client = Devnull::new();
    vsi::run(client, scan, display).await.context("run scan")
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
