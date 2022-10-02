//! Once a scan (which is local) is completed and all the scan artifacts are uploaded to the VSI Forensics Service,
//! the results of the forensics investigation must be waited upon and then its results downloaded.
//!
//! Only then can the client know which dependencies were discovered for the scan artifacts by the forensics service.

use std::fmt::Display;

/// The VSI Forensics Service returns statuses for tracking on which stage forensic analysis is.
///
/// This client only cares about a subset; the rest are informational and can be safely shown to a user to indicate activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Forensic analysis is pending.
    Pending,
    /// Forensic analysis has completed.
    Finished,
    /// Forensic analysis has failed.
    Failed,
    /// Forensic analysis is in some other status, which can be displayed to the user.
    /// This status indicates that it is in process.
    Informational(String),
}

impl Status {
    /// Safely parse the string.
    pub fn parse(input: String) -> Self {
        match input.as_str() {
            "NOT_STARTED" => Self::Pending,
            "DONE" => Self::Finished,
            "FAILED" => Self::Failed,
            _ => Self::Informational(input),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Pending => write!(f, "Pending"),
            Status::Finished => write!(f, "Finished"),
            Status::Failed => write!(f, "Failed"),
            Status::Informational(s) => write!(f, "In Process: {s}"),
        }
    }
}
