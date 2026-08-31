// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use nctforge_core::PhysicalDoseBundle;
use nctforge_transport::{
    BackendDescriptor, CompletedRun, PreparedRun, TransportBackend, TransportCase,
};
use thiserror::Error;

mod data;
mod input;

pub use data::{
    DataArtifact, DataDistributionIdentity, DataInspectionIdentity, NeutronTableCapability,
    NuclearDataError, NuclearDataManifest, PhotonTableCapability, TARGET_DATA_HDF5_VERSION,
    TARGET_EVALUATED_DATA_RELEASE, TARGET_INSPECTION_METHOD, TARGET_OPENMC_SOURCE_COMMIT,
    TARGET_OPENMC_VERSION, TEMPERATURE_TOLERANCE_K,
};
pub use input::{
    CANDIDATE_REFERENCE_SEEDS, GeneratedOpenMcFile, OPENMC_DEFAULT_STRIDE,
    OpenMcCollectionNormalization, OpenMcElectronTreatment, OpenMcEnergyMode,
    OpenMcExecutionProfile, OpenMcExecutionPurpose, OpenMcInputArtifacts, OpenMcInputBindings,
    OpenMcInputDeck, OpenMcInputError, OpenMcInputManifest, OpenMcInputManifestArtifact,
    OpenMcProfileError, OpenMcRawTallyUnit, OpenMcRunControls, OpenMcRunMode, OpenMcScoringMesh,
    OpenMcTallyContract, OpenMcTallyQuantity, OpenMcTemperatureMethod,
};

#[derive(Debug, Clone)]
pub struct OpenMcBackend {
    executable: PathBuf,
}

impl OpenMcBackend {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

impl Default for OpenMcBackend {
    fn default() -> Self {
        Self::new("openmc")
    }
}

impl TransportBackend for OpenMcBackend {
    type BackendError = OpenMcError;

    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor {
            id: "openmc".into(),
            display_name: "OpenMC".into(),
            version: None,
            can_prepare: false,
            can_execute: false,
            can_import: false,
        }
    }

    fn prepare(
        &self,
        _case: &TransportCase,
        _working_directory: &Path,
    ) -> Result<PreparedRun, Self::BackendError> {
        Err(OpenMcError::NotImplemented("input preparation"))
    }

    fn execute(&self, _prepared: &PreparedRun) -> Result<CompletedRun, Self::BackendError> {
        Err(OpenMcError::NotImplemented("controlled execution"))
    }

    fn collect(&self, _completed: &CompletedRun) -> Result<PhysicalDoseBundle, Self::BackendError> {
        Err(OpenMcError::NotImplemented("statepoint collection"))
    }
}

#[derive(Debug, Error)]
pub enum OpenMcError {
    #[error("OpenMC adapter milestone not implemented: {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_no_unimplemented_capability() {
        let descriptor = OpenMcBackend::default().descriptor();
        assert_eq!(descriptor.id, "openmc");
        assert!(!descriptor.can_execute);
    }
}
