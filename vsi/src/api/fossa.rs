//! The API Client implementation for communicating with a FOSSA endpoint.

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use stable_eyre::{
    eyre::{bail, Context},
    Result,
};

use crate::{api::Locator, config, forensics, scan};

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Parse JSON text into a type wrapped with a context displaying the input on error.
macro_rules! parse {
    ($input:expr) => {
        from_str(&$input).wrap_err_with(|| format!("parse: {}", $input))
    };
}

/// Run a request, returning its body as a string on success.
macro_rules! run_req {
    (download, $req:expr, $url:expr, $req_body:expr) => {{
        let res = $req.send().await.context("send request")?;
        let status = res.status();
        let res_body = res.text().await.context("download body")?;
        if !status.is_success() {
            bail!(
                "status({}); url({}); req({}); res({res_body})",
                status.as_u16(),
                $url,
                to_string(&$req_body)?,
            )
        }
        res_body
    }};
    (download, $req:expr, $url:expr) => {{
        run_req!(download, $req, $url, "None")
    }};
    (ignore, $req:expr, $url:expr, $req_body:expr) => {{
        let res = $req.send().await.context("send request")?;
        let status = res.status();
        if !status.is_success() {
            bail!(
                "status({}); url({}); req({}); res({})",
                status.as_u16(),
                $url,
                to_string(&$req_body)?,
                res.text().await.context("download body")?,
            )
        }
    }};
    (ignore, $req:expr, $url:expr) => {{
        run_req!(ignore, $req, $url, "None")
    }};
}

/// Communicates with the VSI Forensics Service through the FOSSA service using the reverse proxy endpoint.
#[derive(Clone, Debug)]
pub struct Fossa {
    client: reqwest::Client,
    endpoint: Url,
    api_key: String,

    // The following information is just for statistics/troubleshooting, so not high priority to get.
    org_id: usize,
    project_id: String,
    revision_id: String,
}

impl Fossa {
    /// Create a new instance with the provided FOSSA endpoint information.
    pub fn new(api: &config::Api, scan: &config::Scan) -> Result<Self> {
        let project_name = scan
            .dir()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "anonymous_project".into());

        Ok(Self {
            org_id: api.organization_id(),
            project_id: format!("custom/{project_name}"),
            api_key: api.key().to_owned(),
            endpoint: Url::parse(api.endpoint())
                .context("parse endpoint")?
                .join("/api/proxy/sherlock/")
                .context("append base url")?,
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(300))
                .user_agent(USER_AGENT)
                .build()
                .context("build client")?,
            revision_id: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("get unix timestamp")?
                .as_secs()
                .to_string(),
        })
    }
}

#[async_trait]
impl super::Client for Fossa {
    /// Create a scan in the VSI Forensics Service.
    async fn create_scan(&self) -> Result<scan::Id> {
        #[derive(Debug, Serialize)]
        struct ReqBody<'a> {
            #[serde(rename = "OrganizationID")]
            org_id: usize,
            #[serde(rename = "ProjectID")]
            project_id: &'a str,
            #[serde(rename = "RevisionID")]
            revision_id: &'a str,
        }

        #[derive(Debug, Deserialize)]
        struct ResBody {
            #[serde(rename = "ScanID")]
            scan_id: scan::Id,
        }

        let req_body = ReqBody {
            org_id: self.org_id,
            project_id: &self.project_id,
            revision_id: &self.revision_id,
        };

        let url = self.endpoint.join("scans")?;
        let req = self
            .client
            .post(url.clone())
            .bearer_auth(&self.api_key)
            .json(&req_body);

        let res_body = run_req!(download, req, url, req_body);
        let ResBody { scan_id } = parse!(&res_body)?;
        Ok(scan_id)
    }

    /// Add scan artifacts to a scan.
    async fn append_artifacts(&self, id: &scan::Id, artifacts: Vec<scan::Artifact>) -> Result<()> {
        #[derive(Debug, Serialize)]
        struct ReqBody {
            #[serde(rename = "ScanData")]
            scan_data: HashMap<PathBuf, fingerprint::Combined>,
        }

        let url = self
            .endpoint
            .join("scans/")?
            .join(&format!("{id}/"))?
            .join("files")?;

        let scan_data = HashMap::from_iter(artifacts.into_iter().map(|a| a.normalize().explode()));
        let req_body = ReqBody { scan_data };
        let req = self
            .client
            .post(url.clone())
            .bearer_auth(&self.api_key)
            .json(&req_body);

        run_req!(ignore, req, url, req_body);
        Ok(())
    }

    /// Complete a scan. This signals to the VSI Forensics Service that no new artifacts will be uploaded after this point.
    async fn complete_scan(&self, id: &scan::Id) -> Result<()> {
        #[derive(Debug, Serialize)]
        struct ReqBody {
            #[serde(rename = "FilePath")]
            file_path: PathBuf,
        }

        let url = self
            .endpoint
            .join("scans/")?
            .join(&format!("{id}/"))?
            .join("complete")?;

        // Indicate that the entire scan is complete.
        let req_body = ReqBody {
            file_path: PathBuf::from("/"),
        };
        let req = self
            .client
            .put(url.clone())
            .bearer_auth(&self.api_key)
            .json(&req_body);

        run_req!(ignore, req, url, req_body);
        Ok(())
    }

    /// Get the current forensics status for a scan.
    async fn forensics_status(&self, id: &scan::Id) -> Result<forensics::Status> {
        #[derive(Debug, Deserialize)]
        struct ResBody {
            #[serde(rename = "Status")]
            status: String,
        }

        let url = self
            .endpoint
            .join("scans/")?
            .join(&format!("{id}/"))?
            .join("status/analysis")?;

        let req = self.client.get(url.clone()).bearer_auth(&self.api_key);
        let res_body = run_req!(download, req, url);
        let ResBody { status } = parse!(res_body)?;
        Ok(forensics::Status::parse(status))
    }

    /// Downloads the forensics results.
    ///
    /// The results are downloaded as a list of locators, treated as opaque strings.
    /// Each locator represents a direct dependency.
    async fn download_forensics(&self, id: &scan::Id) -> Result<HashSet<Locator>> {
        #[derive(Debug, Deserialize)]
        struct ResBody {
            #[serde(default = "Vec::new")]
            locators: Vec<Locator>,
        }

        let url = self
            .endpoint
            .join("scans/")?
            .join(&format!("{id}/"))?
            .join("inferences/locator")?;

        let req = self.client.get(url.clone()).bearer_auth(&self.api_key);
        let res_body = run_req!(download, req, url);
        let ResBody { locators } = parse!(res_body)?;
        Ok(HashSet::from_iter(locators))
    }
}
