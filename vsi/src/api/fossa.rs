//! The API Client implementation for communicating with a FOSSA endpoint.

use std::collections::HashSet;

use async_trait::async_trait;
use reqwest::Url;
use stable_eyre::Result;

use crate::{api::Locator, scan};

/// Communicates with the VSI Forensics Service through the FOSSA service using the reverse proxy endpoint.
#[derive(Clone, Debug)]
pub struct Fossa {
    endpoint: Url,
    api_key: String,
    client: reqwest::Client,
}

impl Fossa {
    /// Create a new instance with the provided FOSSA endpoint information.
    pub fn new(endpoint: &str, api_key: &str) -> Fossa {
        todo!()
    }
}

#[async_trait]
impl super::Client for Fossa {
    /// Create a scan in the VSI Forensics Service.
    async fn create_scan(&self) -> Result<scan::Id> {
        todo!()
    }

    /// Add scan artifacts to a scan.
    async fn append_scan(&self, id: &scan::Id, artifacts: Vec<scan::Artifact>) -> Result<()> {
        todo!()
    }

    /// Complete a scan. This signals to the VSI Forensics Service that no new artifacts will be uploaded after this point.
    async fn complete_scan(&self, id: &scan::Id) -> Result<()> {
        todo!()
    }

    /// Waits for the forensics process to complete or error.
    async fn wait_forensics(&self, id: &scan::Id) -> Result<()> {
        todo!()
    }

    /// Downloads the forensics results.
    ///
    /// The results are downloaded as a list of locators, treated as opaque strings.
    /// Each locator represents a direct dependency.
    async fn download_forensics(&self, id: &scan::Id) -> Result<HashSet<Locator>> {
        todo!()
    }
}
