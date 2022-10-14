//! Interactions with the VSI Forensics Service.
//!
//! Interacting with the VSI Forensics Service is performed through FOSSA;
//! FOSSA configures a reverse proxy endpoint to the VSI Forensics Service.
//!
//! This client then communicates with the VSI Forensics Service through that reverse proxy endpoint
//! using the FOSSA API Key. Both push-only and full access keys are supported for this communication.

use std::collections::HashSet;

use async_trait::async_trait;
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use stable_eyre::Result;

mod devnull;
mod fossa;

pub use devnull::*;
pub use fossa::*;

use crate::{forensics, scan};

/// Refers to a locator result for a scan.
///
/// This is an opaque string. It represents an address for a dependency.
#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, From, Deserialize, Serialize,
)]
pub struct Locator(String);

/// The client for communicating with a given VSI Forensics Service.
#[async_trait]
pub trait Client {
    /// Create a scan in the VSI Forensics Service.
    async fn create_scan(&self) -> Result<scan::Id>;

    /// Add scan artifacts to a scan.
    async fn append_artifacts(&self, id: &scan::Id, artifacts: Vec<scan::Artifact>) -> Result<()>;

    /// Complete a scan. This signals to the VSI Forensics Service that no new artifacts will be uploaded after this point.
    async fn complete_scan(&self, id: &scan::Id) -> Result<()>;

    /// Get the current forensics status for a scan.
    async fn forensics_status(&self, id: &scan::Id) -> Result<forensics::Status>;

    /// Downloads the forensics results.
    ///
    /// The results are downloaded as a list of locators, treated as opaque strings.
    /// Each locator represents a direct dependency.
    async fn download_forensics(&self, id: &scan::Id) -> Result<HashSet<Locator>>;
}
