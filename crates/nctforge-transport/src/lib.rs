// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::{error::Error, path::Path};

use nctforge_core::PhysicalDoseBundle;
use serde::{Deserialize, Serialize};

mod model;

pub use model::{
    AngularDistribution, EnergyDistribution, FixedSourceDefinition, IntervalConvention,
    MaterialDefinition, NeutronThermalTreatment, NuclideMassFraction, ParticleType,
    SourceSpatialDistribution, TransportCase, TransportModelError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendDescriptor {
    pub id: String,
    pub display_name: String,
    pub version: Option<String>,
    pub can_prepare: bool,
    pub can_execute: bool,
    pub can_import: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRun {
    pub backend_id: String,
    pub case_id: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedRun {
    pub backend_id: String,
    pub case_id: String,
    pub working_directory: String,
    pub exit_code: i32,
}

/// Boundary between NCTForge and a particle-transport implementation.
///
/// GUI, biological models, QA, and evidence code must consume this trait or
/// the normalized dose bundle, never backend-specific output directly.
pub trait TransportBackend {
    type BackendError: Error + Send + Sync + 'static;

    fn descriptor(&self) -> BackendDescriptor;

    fn prepare(
        &self,
        case: &TransportCase,
        working_directory: &Path,
    ) -> Result<PreparedRun, Self::BackendError>;

    fn execute(&self, prepared: &PreparedRun) -> Result<CompletedRun, Self::BackendError>;

    fn collect(&self, completed: &CompletedRun) -> Result<PhysicalDoseBundle, Self::BackendError>;
}
