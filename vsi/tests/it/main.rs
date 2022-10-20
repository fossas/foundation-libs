//! Integration tests.

use std::collections::HashMap;
use std::sync::Arc;
use std::{collections::HashSet, env};

use async_trait::async_trait;
use stable_eyre::eyre::ensure;
use stable_eyre::{eyre::Context, Result};
use tokio::sync::Mutex;

use vsi::config;
use vsi::scan::{Artifact, Id, Options, Sink};

mod runner;

#[tokio::test]
async fn dry_run_succeeds() -> Result<()> {
    let dir = runner::clone_vsi_example().await?;
    let scan = config::Scan::builder().dir(dir.path()).build();
    let display = config::Display::default();

    let result = runner::run_dry(scan, display).await?;
    let parsed = serde_json::from_str::<HashSet<&str>>(&result)?;
    let expected = HashSet::from(["git+foo$bar", "cargo+baz$bam"]);
    assert_eq!(parsed, expected);

    Ok(())
}

#[tokio::test]
async fn run_succeeds() -> Result<()> {
    if env::var("ENABLE_NETWORK_TESTS").is_err() {
        return Ok(());
    }

    let dir = runner::clone_vsi_example().await?;
    let scan = config::Scan::builder().dir(dir.path()).build();
    let display = config::Display::default();

    let key = env::var("FOSSA_API_KEY").context("provide FOSSA_API_KEY to run this test")?;
    let org = env::var("FOSSA_ORG_ID").context("provide FOSSA_ORG_ID to run this test")?;
    let org = org.parse().context("provide a `usize` for FOSSA_ORG_ID")?;
    let api = config::Api::builder().key(key).organization_id(org).build();

    let result = runner::run(api, scan, display).await?;

    // Currently the criteria for success for this test is very very basic.
    assert!(result.contains("folly") && result.contains("tesseract"));

    Ok(())
}

struct MemorySink {
    // We don't care about performance here, mutex is not a concern.
    buffer: Arc<Mutex<Vec<Artifact>>>,
    id: Id,
}

#[async_trait]
impl Sink for MemorySink {
    async fn append_scan(&self, id: &Id, artifacts: Vec<Artifact>) -> Result<()> {
        ensure!(id == &self.id, "id's did not match: {} <=> {}", id, self.id);
        self.buffer.lock().await.extend(artifacts);
        Ok(())
    }
}

#[tokio::test]
async fn archive_scan_produces_correct_prints() -> Result<()> {
    let id = Id::new("Fake ID, I promise I'm 21.");
    let sink = MemorySink {
        buffer: Arc::new(Mutex::new(Vec::new())),
        id: id.clone(),
    };
    let opts = Options::builder()
        .root("tests/it/testdata/archive-scan")
        .build();
    let count = vsi::scan::artifacts(&sink, &id, opts)
        .await
        .context("vsi scan")?;
    // two files in zip, plus the zip itself
    assert_eq!(count, 3, "count of produced artifacts");

    let results = Arc::try_unwrap(sink.buffer)
        .expect("unwrap arc")
        .into_inner();
    assert_eq!(
        count,
        results.len(),
        "reported count does not match sink ingestion count"
    );

    let result_map: HashMap<_, _> = results
        .into_iter()
        .map(|art| {
            let (key, comb) = art.explode_string();
            (key, comb.to_string())
        })
        .collect();

    let simple_actual = result_map.get("simple.zip").map(|s| s.as_str());
    let simple_zip_expected =
        "sha_256(65edda9e1933aa8cff1d5aeec70a8ddbd43f971454b982f101aa9beff0b72901)";
    assert_eq!(simple_actual, Some(simple_zip_expected));

    let a_actual = result_map
        .get("simple.zip!_fossa.virtual_!/simple/a.txt")
        .map(|s| s.as_str());
    let a_expected = "sha_256(a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce); comment_stripped:sha_256(a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce)";
    assert_eq!(a_actual, Some(a_expected));

    let b_actual = result_map
        .get("simple.zip!_fossa.virtual_!/simple/b.txt")
        .map(|s| s.as_str());
    let b_expected = "sha_256(367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8); comment_stripped:sha_256(367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8)";
    assert_eq!(b_actual, Some(b_expected));

    Ok(())
}
