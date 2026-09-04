// SPDX-License-Identifier: Apache-2.0

//! A compact, deterministic result for external evidence-loop orchestrators.
//!
//! The result is emitted only after the full evidence-aware report has been
//! regenerated and matched. A rejected scientific candidate is represented
//! as data; it is not confused with failure to perform the verification.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use nctforge_core::ContentReference;
use serde::{Deserialize, Serialize};

use crate::{
    EndfMf6CapturePhotonBalanceReportDocument, EndfMf6Law7ImplicitResidualReportDocument,
    NjoyCapturePhotonMomentComparisonDocument, NjoyDomainAwareSuitabilityReportDocument,
    NjoyEvidenceAwareSuitabilityError, NjoyEvidenceAwareSuitabilityReportDocument,
    NjoyLaw7ImplicitResidualComparisonDocument, NjoySuitabilityQualification,
    NjoyTransportRequirement,
};

pub const NJOY_EVIDENCE_AWARE_CHECK_SCHEMA: &str = "nctforge.njoy-evidence-aware-check/0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NjoyEvidenceAwareCheckResult {
    pub schema_version: String,
    pub source_report: ContentReference,
    pub requirement: NjoyTransportRequirement,
    pub qualification: NjoySuitabilityQualification,
    pub rejected_run_count: u64,
    pub remaining_in_domain_kinematic_violation_count: u64,
    pub verification: NjoyEvidenceAwareCheckVerification,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NjoyEvidenceAwareCheckVerification {
    RegeneratedAndMatched,
}

impl NjoyEvidenceAwareCheckResult {
    pub fn verify_and_build(
        report: &NjoyEvidenceAwareSuitabilityReportDocument,
        domain: &NjoyDomainAwareSuitabilityReportDocument,
        law7_residual: &EndfMf6Law7ImplicitResidualReportDocument,
        law7_attribution: &NjoyLaw7ImplicitResidualComparisonDocument,
        capture_balance: &EndfMf6CapturePhotonBalanceReportDocument,
        capture_comparison: &NjoyCapturePhotonMomentComparisonDocument,
    ) -> Result<Self, NjoyEvidenceAwareSuitabilityError> {
        report.verify_against_evidence(
            domain,
            law7_residual,
            law7_attribution,
            capture_balance,
            capture_comparison,
        )?;
        Ok(Self::from_verified(report))
    }

    #[must_use]
    fn from_verified(report: &NjoyEvidenceAwareSuitabilityReportDocument) -> Self {
        Self {
            schema_version: NJOY_EVIDENCE_AWARE_CHECK_SCHEMA.into(),
            source_report: ContentReference {
                id: report.report.id.clone(),
                sha256: report.sha256.clone(),
            },
            requirement: report.report.requirement,
            qualification: report.report.qualification,
            rejected_run_count: report.report.rejected_run_count,
            remaining_in_domain_kinematic_violation_count: report
                .report
                .remaining_in_domain_kinematic_violation_count,
            verification: NjoyEvidenceAwareCheckVerification::RegeneratedAndMatched,
            limitations: vec![
                "A mechanically clear result would remain unreviewed; this check does not approve response tables, qualify transport, or establish clinical validity.".into(),
                "The result summarizes one exact evidence-aware report and does not replace its per-nuclide findings or bound source evidence.".into(),
            ],
        }
    }

    pub fn write_new(&self, path: &Path) -> io::Result<()> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROZEN_ASSESSMENT: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-evidence-aware-suitability.json"
    );

    #[test]
    fn rejected_science_is_data_not_a_verification_error() {
        let report =
            NjoyEvidenceAwareSuitabilityReportDocument::from_bytes(FROZEN_ASSESSMENT).unwrap();
        let result = NjoyEvidenceAwareCheckResult::from_verified(&report);
        assert_eq!(
            result.qualification,
            NjoySuitabilityQualification::TransportedPhotonKermaRejected
        );
        assert_eq!(result.rejected_run_count, 4);
        assert_eq!(result.remaining_in_domain_kinematic_violation_count, 102);
        assert_eq!(
            result.verification,
            NjoyEvidenceAwareCheckVerification::RegeneratedAndMatched
        );
        assert_eq!(
            serde_json::to_value(result).unwrap()["qualification"],
            "transported_photon_kerma_rejected"
        );
    }
}
