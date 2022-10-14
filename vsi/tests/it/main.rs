//! Integration tests.

use std::{collections::HashSet, env};

use stable_eyre::{eyre::Context, Result};
use vsi::config;

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
