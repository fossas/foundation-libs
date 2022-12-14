//! An API Client implementation that just logs output and drops it.

use std::collections::HashSet;

use async_trait::async_trait;
use derive_more::Constructor;
use log::debug;
use stable_eyre::Result;

use crate::{api::Locator, forensics, scan};

/// Logs output and drops it. Always results in the same set of locators being discovered.
/// Meant for basic sanity testing.
#[derive(Clone, Debug, Default, Constructor)]
pub struct Devnull {}

#[async_trait]
impl super::Client for Devnull {
    /// Create a scan in the VSI Forensics Service.
    async fn create_scan(&self) -> Result<scan::Id> {
        let id = String::from("fake_scan_id");
        debug!("[dryrun] created scan id: {}", id);
        Ok(scan::Id::from(id))
    }

    /// Add scan artifacts to a scan.
    async fn append_artifacts(&self, id: &scan::Id, artifacts: Vec<scan::Artifact>) -> Result<()> {
        debug!(
            "[dryrun] append {} artifact(s) to scan {id}:",
            artifacts.len()
        );
        for artifact in artifacts {
            debug!("[dryrun] -> {artifact}");
        }
        Ok(())
    }

    /// Complete a scan. This signals to the VSI Forensics Service that no new artifacts will be uploaded after this point.
    async fn complete_scan(&self, id: &scan::Id) -> Result<()> {
        debug!("[dryrun] complete scan {id}");
        Ok(())
    }

    /// Waits for the forensics process to complete or error.
    async fn forensics_status(&self, id: &scan::Id) -> Result<forensics::Status> {
        debug!("[dryrun] get forensics for scan {id}");
        Ok(forensics::Status::Finished)
    }

    /// Downloads the forensics results.
    ///
    /// The results are downloaded as a list of locators, treated as opaque strings.
    /// Each locator represents a direct dependency.
    async fn download_forensics(&self, id: &scan::Id) -> Result<HashSet<Locator>> {
        debug!("[dryrun] download forensics for scan {id}");
        Ok(HashSet::from([
            Locator::from(String::from("git+foo$bar")),
            Locator::from(String::from("cargo+baz$bam")),
        ]))
    }
}
