// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub role: String,
    pub path: String,
    pub sha256: String,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    pub schema_version: String,
    pub run_id: String,
    pub case_id: String,
    pub backend_id: String,
    pub backend_version: String,
    pub nuclear_data_id: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub qualification: QualificationBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationBoundary {
    SyntheticResearchOnly,
    CrossCodeResearchOnly,
    ExperimentallyValidatedResearchOnly,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_stable() {
        assert_eq!(
            sha256_hex(b"NCTForge"),
            "c2408160f40e5432661a0c32e6b6c133c18109af762769e5ca989341fda04961"
        );
    }
}
