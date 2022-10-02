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

pub mod api;
pub mod config;
pub mod forensics;
pub mod scan;
