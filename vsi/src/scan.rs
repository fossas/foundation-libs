//! Manages the scan process.
//!
//! Scan is an overloaded term, especially at FOSSA; within VSI "scan" refers to the
//! process of enumerating and uploading "scan artifacts" (i.e. fingerprints representing user source code)
//! to the VSI Forensics Service.
//!
//! The act of examining those artifacts is then referred to as the "forensics" process.
//!
//! Once a scan (which is local) is completed and all the scan artifacts are uploaded to the VSI Forensics Service,
//! the results of the forensics investigation must be waited upon and then its results downloaded.
//!
//! Only then can the client know which dependencies were discovered for the scan artifacts by the forensics service.

use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use defer_lite::defer;
use derive_more::{Display, From};
use log::debug;
use serde::{Deserialize, Serialize};
use stable_eyre::{
    eyre::{ensure, Context},
    Result,
};
use tokio::{
    sync::mpsc::{channel, Receiver},
    try_join,
};
use typed_builder::TypedBuilder;

use crate::api::Client;

mod walk;

const ARTIFACT_BUFFER_LIMIT: usize = 1000;

/// Options for the scan process.
#[derive(Clone, Eq, PartialEq, Debug, TypedBuilder)]
pub struct Options {
    /// The directory to walk.
    #[builder(setter(into))]
    root: PathBuf,
}

/// An identifier indicating a specific scan. This is an opaque string.
#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, From, Deserialize, Serialize,
)]
pub struct Id(String);

/// An artifact in a scan.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Artifact(PathBuf, fingerprint::Combined);

impl Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Artifact({:?}, {})", self.0, self.1)
    }
}

impl Artifact {
    /// Explode the artifact into its constituent tuple.
    pub fn explode(self) -> (PathBuf, fingerprint::Combined) {
        (self.0, self.1)
    }

    /// Explode, but with a string instead of a path.
    pub fn explode_string(self) -> (String, fingerprint::Combined) {
        (self.0.to_string_lossy().to_string(), self.1)
    }
}

/// A representation of the VSI Forensics Service to which artifacts are uploaded.
#[async_trait]
pub trait Sink {
    /// Add scan artifacts to a scan.
    async fn append_scan(&self, id: &Id, artifacts: Vec<Artifact>) -> Result<()>;
}

#[async_trait]
impl<T: Client + Sync> Sink for T {
    async fn append_scan(&self, id: &Id, artifacts: Vec<Artifact>) -> Result<()> {
        self.append_scan(id, artifacts).await
    }
}

/// Walk the file system, generating and uploading scan artifacts in parallel.
/// Returns the number of artifacts uploaded.
///
/// # Resource leaking
///
/// Dropping this future early can result in leaked threads.
pub async fn artifacts(client: &impl Sink, id: &Id, opts: Options) -> Result<usize> {
    debug!("scanning artifacts for scan {id} with options: {opts:?}");
    defer! { debug!("exited scanning artifacts"); }

    // Allow the channel to buffer up to the limit while an upload runs.
    let (tx, rx) = channel(ARTIFACT_BUFFER_LIMIT);
    let uploader = upload(client, id, rx);

    // Walking and fingerprinting is a synchronous- but streaming- operation.
    // Dropping the future returned by `task::spawn_blocking` doesn't kill the thread (it can't possibly do so).
    // This token allows for cooperative cancellation of the thread.
    let ctx = walk::Context::new();
    let walker = ctx.walk_local_fs(tx, opts);

    // Wait for both uploader and walker to complete, or one to error.
    // Either way, cancel the token and return the result. This ensures that (assuming it behaves correctly)
    // the walker doesn't keep running for an unbounded amount of time after this function returns.
    // Due to parallel invocation it may keep running for a non-zero amount of time, but that _should_ be minimal.
    try_join!(uploader, walker).and_then(|(uploaded, produced)| {
        ensure!(
            uploaded == produced,
            "mismatch between uploaded ({uploaded}) and produced ({produced})"
        );
        Ok(uploaded)
    })
}

/// Buffers incoming `Artifact`s in the input channel. Once enough have been buffered,
/// uploads them to the VSI Forensics Service through the provided sink implementation.
/// Returns the number of artifacts uploaded.
///
/// Returns with an error if an error is encountered during the upload.
async fn upload(client: &impl Sink, id: &Id, mut input: Receiver<Artifact>) -> Result<usize> {
    debug!("running uploader");
    defer! { debug!("exited uploader"); }
    let mut uploaded = 0;

    // Buffer artifacts and upload them.
    // The channel also contains its own buffering, so needless backpressure should be minimal;
    // backpressure should only occur when uploading is actually slower than fingerprinting (which is correct).
    let mut buf = Vec::with_capacity(ARTIFACT_BUFFER_LIMIT);
    while let Some(artifact) = input.recv().await {
        debug!("buffering artifact: {artifact}");
        buf.push(artifact);
        uploaded += 1;

        debug!("buffered {} / {ARTIFACT_BUFFER_LIMIT} artifacts", buf.len());
        if buf.len() == ARTIFACT_BUFFER_LIMIT {
            debug!("buffer limit reached, uploading chunk");
            client.append_scan(id, buf).await.context("upload buffer")?;
            buf = Vec::with_capacity(ARTIFACT_BUFFER_LIMIT);
        }
    }

    // Channel is closed; upload any remaining artifacts in the buffer.
    if !buf.is_empty() {
        debug!("uploading {} remaining item(s) in final chunk", buf.len());
        client
            .append_scan(id, buf)
            .await
            .context("upload final buffer")?;
    }

    Ok(uploaded)
}
