// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::{error::Error, path::Path};

use nctforge_core::{GridGeometry, PhysicalDoseBundle};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportCase {
    pub schema_version: String,
    pub case_id: String,
    pub geometry: GridGeometry,
    pub material_model_id: String,
    pub source_model_id: String,
    pub requested_source_particles: u64,
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
