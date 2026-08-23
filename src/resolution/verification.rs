//! Operational verification report types kept separate from definition validity.

use serde::{Deserialize, Serialize};

/// Result of one optional operational verification check.
///
/// Operational verification is deliberately separate from definition validity:
/// a configuration can be definition-valid without a check having been run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// The installation-specific check succeeded.
    Verified,
    /// The check ran and failed.
    Failed,
    /// The check was intentionally not attempted.
    NotPerformed,
    /// Only part of a multi-subject check could be completed.
    PartiallyVerified,
}

/// One installation-specific operational verification result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationCheck {
    /// Stable check name, such as `database-connectivity`.
    pub name: String,
    /// Configuration paths relevant to the check.
    pub subjects: Vec<String>,
    /// Structured outcome.
    pub result: VerificationResult,
    /// Optional redaction-safe explanation.
    pub message: Option<String>,
}

/// Installation-specific operational verification results.
///
/// This report is deliberately separate from [`crate::ResolutionReport`]. The kernel
/// does not infer whether a failed or partial check should block process startup
/// or a runtime refresh; that decision belongs to the consumer.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Individual checks performed for this installation.
    pub checks: Vec<VerificationCheck>,
}

impl VerificationReport {
    /// Creates an empty operational verification report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one verification check.
    #[must_use]
    pub fn push(mut self, check: VerificationCheck) -> Self {
        self.checks.push(check);
        self
    }
}
