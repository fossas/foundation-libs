use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cancel::Token;
use defer_lite::defer;
use fingerprint::fingerprint;
use log::{debug, info};
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use stable_eyre::{
    eyre::{bail, eyre},
    Result,
};
use tokio::{sync::mpsc::Sender, task};
use walkdir::{DirEntry, WalkDir};

const REPORT_TIMEOUT: Duration = Duration::from_secs(1);

use super::{Artifact, Options};

/// Represents a walking operation context.
///
/// Walking operations are run in parallel and rely on cooperative cancellation.
/// This type ensures that walkers attached to `Context` are requested to cancel when `Context` is dropped.
pub struct Context {
    token: Arc<Token>,
}

impl Drop for Context {
    fn drop(&mut self) {
        debug!("walk context dropped, requesting walker cancellation");
        self.token.cancel();
    }
}

impl Context {
    /// Create a new walk context.
    pub fn new() -> Self {
        Self {
            token: Arc::new(Token::new()),
        }
    }

    /// Walks the file system producing `Artifact`s. Outputs them to the output channel.
    /// Returns the count of artifcts produced.
    ///
    /// Outputs are generated in parallel and then are interleaved to the channel,
    /// meaning that it is possible to have an error returned followed by data being written to the channel.
    /// In general it should be fine to return early (and drop the receive side of the channel) in the case of an error;
    /// if the parallel producers attempt to send to a closed channel they simply error,
    /// and subsequent errors after the first should be ignored.
    ///
    /// Returns with an error if an error is encountered during the walk or fingerprint process.
    /// This includes cancellation: if the cancel token is cancelled, this function returns a cancellation error.
    /// Closes the output channel on return.
    // This function primarily exists in order to wrap the spawn join back into a result.
    pub async fn walk_local_fs(&self, output: Sender<Artifact>, opts: Options) -> Result<usize> {
        debug!("walking fs with options: {opts:?}");
        defer! { debug!("exiting fs walker"); }

        let cancel = self.token.clone();
        task::spawn_blocking(move || fs_worker(cancel, output, opts))
            .await
            .expect("worker thread must not panic")
    }
}

/// The worker for `fs`, since directory walking and fingerprinting are currently synchronous operations.
fn fs_worker(token: Arc<Token>, out: Sender<Artifact>, opts: Options) -> Result<usize> {
    debug!("enter fs worker");
    defer! { debug!("exiting fs worker"); }

    let mut produced = 0;
    let mut last_report = Instant::now();

    use stable_eyre::eyre::Context;
    WalkDir::new(&opts.root)
        .follow_links(false)
        // Produces an iterator over recursive contents of the directory.
        .into_iter()
        // User-provided filters.
        .filter_entry(|de| matches_filters(de, &opts))
        // Only attempt to process files for fingerprints.
        // Pass along errors too, so that the error can be reported.
        .filter(is_file_or_err)
        // Collect and report in the iterator before it becomes parallel; iteration here is serial.
        // Iterators are lazy so this still benefits from parallel operations.
        .inspect(|_| {
            produced += 1;
            let now = Instant::now();
            if now.duration_since(last_report) >= REPORT_TIMEOUT {
                info!(
                    "discovered {} fingerprints...",
                    produced.to_formatted_string(&Locale::en)
                );
                last_report = now;
            }
        })
        // Rayon magic: turn this iterator into a parallel iterator, then generate each artifact in parallel.
        .par_bridge()
        .try_for_each(|de| -> Result<()> {
            if token.check_cancel().is_err() {
                debug!("received cancellation signal, bailing");
                bail!("cancellation requested");
            }

            // Fingerprint the file.
            let path = de.map(|de| de.into_path()).context("iterate contents")?;
            let combined = fingerprint(&path).wrap_err_with(|| eyre!("fingerprint {path:?}"))?;

            // Generate and send the artifact.
            let artifact = Artifact(path, combined);
            debug!("generated artifact: {artifact}");
            out.blocking_send(artifact).context("send entry")?;

            Ok(())
        })?;

    info!(
        "discovered {} fingerprints total",
        produced.to_formatted_string(&Locale::en)
    );
    Ok(produced)
}

fn matches_filters(_: &DirEntry, _: &Options) -> bool {
    // TODO: implement filtering
    true
}

fn is_file_or_err<E>(de: &Result<DirEntry, E>) -> bool {
    de.as_ref().map(|de| de.path().is_file()).unwrap_or(true)
}
