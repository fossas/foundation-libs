//! The library portion of the VSI client.
//!
//! The overall process for VSI is as follows:
//!
//! 1. The user initiates a project analysis with VSI enabled.
//! 2. The client is run on the project source directory.
//! 3. The client then waits for results (a list of locators) and reports them as the VSI analysis.
//!
//! The act of running a VSI analysis (step 2) is also composed of multiple steps:
//!
//! 1. A "scan ID" is created in the VSI Forensics Service.
//!    This is an opaque string.
//! 2. Files in the project are enumerated, and their fingerprints are generated.
//!    This results in a set of `(Path, Fingerprint)` pairs for the project.
//! 3. Those pairs are uploaded, attached to the scan ID.
//!    `(Path, Fingerprint)` pairs are referred to as the "scan artifacts".
//! 4. The scan ID is marked complete.
//!    This informs the VSI Forensics Service that no new files will be added to the scan.
//! 5. The VSI Forensics Service begins analyzing the scan artifacts.
//!    This is an asynchronous process that scales subliniarly with the number of scan artifacts.
//! 6. Eventually, the VSI Forensics Service finishes making its determinations and reports its results.
//!    The results are downloaded as either a map of `(Path, Component ID)` pairs, or a list of locators.
//!    Component ID is internal to the VSI Forensics Service, so generally the list of locators is used.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use api::Client;
use log::info;
use stable_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::time::sleep;

pub mod api;
pub mod config;
pub mod forensics;
pub mod scan;

/// Run a scan with the provided configuration, returning its result according to the config.
pub async fn run(
    client: impl Client + Sync,
    scan: config::Scan,
    display: config::Display,
) -> Result<String> {
    let start = Instant::now();

    let id = client.create_scan().await.context("create scan")?;
    info!("created scan: {id}");

    info!("scanning artifacts");
    let opts = scan::Options::builder().root(scan.dir()).build();
    let artifact_count = scan::artifacts(&client, &id, opts)
        .await
        .context("scan artifacts")?;
    client
        .complete_scan(&id)
        .await
        .context("mark scan complete")?;

    info!(
        "completed scan of {artifact_count} artifacts in {:?}",
        start.elapsed()
    );

    info!("waiting for forensics");
    wait_forensics(&client, &id)
        .await
        .context("wait for forensics")?;

    let export = match display.export() {
        config::Export::ScanID => Ok(format!("{{ scan_id: {id} }}")),
        config::Export::Locators => {
            info!("downloading results");
            let results = client
                .download_forensics(&id)
                .await
                .context("download forensics")?;

            serde_json::to_string(&results).context("render results")
        }
    }?;

    Ok(export)
}

/// Waits for forensics to complete or error.
async fn wait_forensics(client: &impl Client, id: &scan::Id) -> Result<()> {
    let start = Instant::now();
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
                info!("forensics complete in {:?}", start.elapsed());
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
