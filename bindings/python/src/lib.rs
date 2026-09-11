// SPDX-License-Identifier: Apache-2.0

//! Narrow PyO3 boundary over the authoritative NCTForge Rust crates.
//!
//! This module implements no dose, geometry, evidence, or qualification logic
//! of its own (ADR 0015). Every function below delegates to the same Rust
//! contracts used by the CLI and GUI so Python users observe identical
//! acceptance, rejection, serialization, and content-identity behavior.

use std::fmt::Display;
use std::fs;
use std::path::PathBuf;

use nctforge_dicom::{
    BenchmarkReport, VerifiedBenchmarkCase, load_nf_bnct_001, synthetic::generate_nf_bnct_001,
    verify_nf_bnct_001,
};
use nctforge_evidence::{CaseManifest, sha256_file};
use nctforge_openmc::OpenMcBackend;
use nctforge_transport::{
    BackendDescriptor, ComponentDefinitionProfile, FixedSourceDefinition, MaterialDefinition,
    NeutronResponseSet, ResponseGenerationMethod, TransportBackend,
};
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use serde::de::DeserializeOwned;

create_exception!(
    nctforge,
    NctForgeError,
    PyException,
    "An NCTForge contract, verification, or evidence check failed."
);

fn reject(error: impl Display) -> PyErr {
    NctForgeError::new_err(error.to_string())
}

fn load_contract<T>(path: PathBuf) -> PyResult<T>
where
    T: DeserializeOwned + ContractCheck,
{
    let bytes = fs::read(&path).map_err(reject)?;
    let contract: T = serde_json::from_slice(&bytes).map_err(reject)?;
    contract.check().map_err(reject)?;
    Ok(contract)
}

/// Uniform "deserialize then run the contract's own validation" rule so no
/// validation rule is reimplemented at the language boundary.
trait ContractCheck {
    fn check(&self) -> Result<(), String>;
}

macro_rules! contract_check {
    ($type:ty, $method:ident, $error:ty) => {
        impl ContractCheck for $type {
            fn check(&self) -> Result<(), String> {
                self.$method().map_err(|error: $error| error.to_string())
            }
        }
    };
}

contract_check!(
    MaterialDefinition,
    validate,
    nctforge_transport::TransportModelError
);
contract_check!(
    FixedSourceDefinition,
    validate,
    nctforge_transport::TransportModelError
);
contract_check!(
    ComponentDefinitionProfile,
    validate,
    nctforge_transport::ResponseMethodError
);
contract_check!(
    ResponseGenerationMethod,
    validate,
    nctforge_transport::ResponseMethodError
);
contract_check!(
    NeutronResponseSet,
    validate,
    nctforge_transport::ResponseSetError
);
contract_check!(CaseManifest, validate, nctforge_evidence::ManifestError);

/// Transport-backend descriptor with its current capability flags.
///
/// Flags are reported exactly as the Rust backend advertises them; an action
/// that is not implemented remains `False` rather than silently succeeding.
#[pyclass(frozen, name = "Backend")]
struct PyBackend {
    inner: BackendDescriptor,
}

#[pymethods]
impl PyBackend {
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn display_name(&self) -> &str {
        &self.inner.display_name
    }

    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    #[getter]
    fn can_prepare(&self) -> bool {
        self.inner.can_prepare
    }

    #[getter]
    fn can_execute(&self) -> bool {
        self.inner.can_execute
    }

    #[getter]
    fn can_import(&self) -> bool {
        self.inner.can_import
    }

    fn __repr__(&self) -> String {
        format!(
            "Backend(id={:?}, can_prepare={}, can_execute={}, can_import={})",
            self.inner.id, self.inner.can_prepare, self.inner.can_execute, self.inner.can_import
        )
    }
}

/// Descriptors for every compiled-in transport backend.
#[pyfunction]
fn backends() -> Vec<PyBackend> {
    vec![PyBackend {
        inner: OpenMcBackend::default().descriptor(),
    }]
}

/// SHA-256 of a file's exact bytes, lowercase hex.
#[pyfunction]
fn file_sha256(path: PathBuf) -> PyResult<String> {
    sha256_file(&path).map_err(reject)
}

/// Verified ROI summary for a structure in the frozen benchmark case.
#[pyclass(frozen, name = "Structure")]
struct PyStructure {
    number: i32,
    name: String,
    voxel_count: usize,
    volume_cm3: f64,
    centroid_lps_mm: [f64; 3],
}

#[pymethods]
impl PyStructure {
    #[getter]
    fn number(&self) -> i32 {
        self.number
    }

    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn voxel_count(&self) -> usize {
        self.voxel_count
    }

    #[getter]
    fn volume_cm3(&self) -> f64 {
        self.volume_cm3
    }

    #[getter]
    fn centroid_lps_mm(&self) -> (f64, f64, f64) {
        self.centroid_lps_mm.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Structure(name={:?}, voxel_count={}, volume_cm3={})",
            self.name, self.voxel_count, self.volume_cm3
        )
    }
}

/// Result of the independent `NF-BNCT-001` verification oracle.
#[pyclass(frozen, name = "CaseVerification")]
struct PyCaseVerification {
    inner: BenchmarkReport,
}

#[pymethods]
impl PyCaseVerification {
    #[getter]
    fn case_id(&self) -> &str {
        self.inner.case_id
    }

    /// [columns, rows, slices]
    #[getter]
    fn shape(&self) -> (u32, u32, u32) {
        self.inner.shape.into()
    }

    /// Voxel spacing in millimetres as [column, row, slice].
    #[getter]
    fn spacing_mm(&self) -> (f64, f64, f64) {
        self.inner.spacing_mm.into()
    }

    /// LPS position of the centre of voxel [0, 0, 0], millimetres.
    #[getter]
    fn origin_mm(&self) -> (f64, f64, f64) {
        self.inner.origin_mm.into()
    }

    #[getter]
    fn ct_slice_count(&self) -> usize {
        self.inner.ct_slice_count
    }

    #[getter]
    fn verified_artifact_count(&self) -> usize {
        self.inner.verified_artifact_count
    }

    #[getter]
    fn structures(&self) -> Vec<PyStructure> {
        self.inner
            .rois
            .iter()
            .map(|roi| PyStructure {
                number: roi.number,
                name: roi.name.clone(),
                voxel_count: roi.voxel_count,
                volume_cm3: roi.volume_cm3,
                centroid_lps_mm: roi.centroid_lps_mm,
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CaseVerification(case_id={:?}, shape={:?}, verified_artifacts={})",
            self.inner.case_id, self.inner.shape, self.inner.verified_artifact_count
        )
    }
}

/// A fully verified `NF-BNCT-001` case with geometry and structure access.
///
/// Construction runs the complete verification oracle; a case that fails any
/// gate cannot be loaded through this object.
#[pyclass(frozen, name = "VerifiedCase")]
struct PyVerifiedCase {
    inner: VerifiedBenchmarkCase,
}

#[pymethods]
impl PyVerifiedCase {
    #[getter]
    fn report(&self) -> PyCaseVerification {
        PyCaseVerification {
            inner: self.inner.report.clone(),
        }
    }

    /// CT lattice: shape, spacing, origin, and direction in LPS millimetres.
    #[getter]
    fn geometry(&self) -> PyGeometry {
        PyGeometry {
            inner: self.inner.ct.geometry.clone(),
        }
    }

    /// Frame of Reference UID shared by the CT series.
    #[getter]
    fn frame_of_reference_uid(&self) -> &str {
        &self.inner.ct.frame_of_reference_uid
    }

    /// Structure summaries computed from the rasterized ROI masks.
    #[getter]
    fn structures(&self) -> Vec<PyStructure> {
        self.inner
            .structures
            .rois
            .iter()
            .map(|roi| PyStructure {
                number: roi.number,
                name: roi.name.clone(),
                voxel_count: roi.voxel_count(),
                volume_cm3: roi.volume_cm3(&self.inner.ct),
                centroid_lps_mm: roi.centroid_lps_mm(&self.inner.ct).unwrap_or([f64::NAN; 3]),
            })
            .collect()
    }

    /// Modality value at voxel [column, row, slice], after rescale.
    fn ct_value(&self, column: u32, row: u32, slice: u32) -> PyResult<f64> {
        let shape = self.inner.ct.geometry.shape;
        if column >= shape[0] || row >= shape[1] || slice >= shape[2] {
            return Err(reject(format!(
                "voxel [{column}, {row}, {slice}] outside shape {shape:?}"
            )));
        }
        let index = (slice as usize) * (shape[0] as usize) * (shape[1] as usize)
            + (row as usize) * (shape[0] as usize)
            + column as usize;
        Ok(self
            .inner
            .ct
            .modality_value(self.inner.ct.stored_pixels[index]))
    }

    /// Boolean mask for the named structure, columns fastest then rows, slices.
    fn structure_mask(&self, name: &str) -> PyResult<Vec<bool>> {
        self.inner
            .structures
            .roi(name)
            .map(|roi| roi.voxels.clone())
            .ok_or_else(|| reject(format!("unknown structure {name:?}")))
    }
}

/// Validated CT lattice geometry in the DICOM LPS patient frame.
#[pyclass(frozen, name = "Geometry")]
struct PyGeometry {
    inner: nctforge_core::GridGeometry,
}

#[pymethods]
impl PyGeometry {
    /// [columns, rows, slices]
    #[getter]
    fn shape(&self) -> (u32, u32, u32) {
        self.inner.shape.into()
    }

    #[getter]
    fn spacing_mm(&self) -> (f64, f64, f64) {
        self.inner.spacing_mm.into()
    }

    #[getter]
    fn origin_mm(&self) -> (f64, f64, f64) {
        self.inner.origin_mm.into()
    }

    #[getter]
    fn direction(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        self.inner.direction.into()
    }

    #[getter]
    fn voxel_count(&self) -> PyResult<usize> {
        self.inner.voxel_count().map_err(reject)
    }

    /// LPS position of a voxel centre in millimetres.
    fn voxel_center_lps_mm(&self, column: u32, row: u32, slice: u32) -> PyResult<(f64, f64, f64)> {
        self.inner
            .voxel_center_lps_mm([column, row, slice])
            .map(Into::into)
            .map_err(reject)
    }
}

/// Summary of a generated `NF-BNCT-001` case directory.
#[pyclass(frozen, name = "GeneratedCase")]
struct PyGeneratedCase {
    root: PathBuf,
    ct_file_count: usize,
    rtstruct_file: PathBuf,
    manifest_file: PathBuf,
}

#[pymethods]
impl PyGeneratedCase {
    #[getter]
    fn root(&self) -> PathBuf {
        self.root.clone()
    }

    #[getter]
    fn ct_file_count(&self) -> usize {
        self.ct_file_count
    }

    #[getter]
    fn rtstruct_file(&self) -> PathBuf {
        self.rtstruct_file.clone()
    }

    #[getter]
    fn manifest_file(&self) -> PathBuf {
        self.manifest_file.clone()
    }
}

/// Generate the deterministic synthetic `NF-BNCT-001` DICOM case.
///
/// Refuses to overwrite an existing destination, matching the CLI.
#[pyfunction]
fn generate_case(destination: PathBuf) -> PyResult<PyGeneratedCase> {
    let generated = generate_nf_bnct_001(&destination).map_err(reject)?;
    Ok(PyGeneratedCase {
        root: generated.root,
        ct_file_count: generated.ct_files.len(),
        rtstruct_file: generated.rtstruct_file,
        manifest_file: generated.manifest_file,
    })
}

/// Verify a generated `NF-BNCT-001` case against the independent frozen oracle.
#[pyfunction]
fn verify_case(root: PathBuf) -> PyResult<PyCaseVerification> {
    let report = verify_nf_bnct_001(&root).map_err(reject)?;
    Ok(PyCaseVerification { inner: report })
}

/// Load `NF-BNCT-001` only after all geometry and artifact gates pass.
#[pyfunction]
fn load_case(root: PathBuf) -> PyResult<PyVerifiedCase> {
    let case = load_nf_bnct_001(&root).map_err(reject)?;
    Ok(PyVerifiedCase { inner: case })
}

/// One artifact binding inside a verified case manifest.
#[pyclass(frozen, name = "Artifact")]
struct PyArtifact {
    role: String,
    path: String,
    sha256: String,
    media_type: Option<String>,
}

#[pymethods]
impl PyArtifact {
    #[getter]
    fn role(&self) -> &str {
        &self.role
    }

    #[getter]
    fn path(&self) -> &str {
        &self.path
    }

    #[getter]
    fn sha256(&self) -> &str {
        &self.sha256
    }

    #[getter]
    fn media_type(&self) -> Option<String> {
        self.media_type.clone()
    }
}

/// A parsed and schema-validated `case.json` manifest.
#[pyclass(frozen, name = "CaseManifest")]
struct PyCaseManifest {
    inner: CaseManifest,
}

#[pymethods]
impl PyCaseManifest {
    #[getter]
    fn schema_version(&self) -> &str {
        &self.inner.schema_version
    }

    #[getter]
    fn case_id(&self) -> &str {
        &self.inner.case_id
    }

    #[getter]
    fn qualification(&self) -> PyResult<String> {
        serde_json::to_value(&self.inner.qualification)
            .map_err(reject)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| reject("qualification is not a string"))
    }

    #[getter]
    fn coordinate_system(&self) -> PyResult<String> {
        serde_json::to_value(&self.inner.coordinate_system)
            .map_err(reject)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| reject("coordinate_system is not a string"))
    }

    #[getter]
    fn frame_of_reference_uid(&self) -> &str {
        &self.inner.frame_of_reference_uid
    }

    #[getter]
    fn material_model_id(&self) -> &str {
        &self.inner.material_model_id
    }

    #[getter]
    fn source_model_id(&self) -> &str {
        &self.inner.source_model_id
    }

    #[getter]
    fn geometry(&self) -> PyGeometry {
        PyGeometry {
            inner: self.inner.geometry.clone(),
        }
    }

    #[getter]
    fn structures(&self) -> Vec<PyStructure> {
        self.inner
            .structures
            .iter()
            .map(|record| PyStructure {
                number: record.number,
                name: record.name.clone(),
                voxel_count: record.voxel_count,
                volume_cm3: record.volume_cm3,
                centroid_lps_mm: record.centroid_lps_mm,
            })
            .collect()
    }

    #[getter]
    fn artifacts(&self) -> Vec<PyArtifact> {
        self.inner
            .artifacts
            .iter()
            .map(|record| PyArtifact {
                role: record.role.clone(),
                path: record.path.clone(),
                sha256: record.sha256.clone(),
                media_type: record.media_type.clone(),
            })
            .collect()
    }

    /// Canonical JSON bytes as produced by the Rust contract, as text.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(reject)
    }

    /// Re-verify every bound artifact under `case_root`; returns the count.
    fn verify_artifacts(&self, case_root: PathBuf) -> PyResult<usize> {
        self.inner.verify_artifacts(&case_root).map_err(reject)?;
        Ok(self.inner.artifacts.len())
    }
}

/// Read and validate a `case.json` manifest.
#[pyfunction]
fn read_manifest(path: PathBuf) -> PyResult<PyCaseManifest> {
    Ok(PyCaseManifest {
        inner: load_contract(path)?,
    })
}

// PyO3 requires the struct name to match the `name = "..."` attribute target.
macro_rules! contract_wrapper {
    ($py_name:literal, $rust_struct:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        #[pyclass(frozen, name = $py_name)]
        struct $rust_struct {
            inner: $inner,
        }

        #[pymethods]
        impl $rust_struct {
            #[getter]
            fn schema_version(&self) -> &str {
                &self.inner.schema_version
            }

            #[getter]
            fn id(&self) -> &str {
                &self.inner.id
            }

            /// Canonical JSON bytes as produced by the Rust contract, as text.
            fn to_json(&self) -> PyResult<String> {
                serde_json::to_string_pretty(&self.inner).map_err(reject)
            }
        }
    };
}

contract_wrapper!(
    "Material",
    PyMaterial,
    MaterialDefinition,
    "A validated explicit-nuclide material contract."
);
contract_wrapper!(
    "FixedSource",
    PyFixedSource,
    FixedSourceDefinition,
    "A validated backend-neutral fixed-source contract."
);
contract_wrapper!(
    "ComponentProfile",
    PyComponentProfile,
    ComponentDefinitionProfile,
    "A validated four-component dose-definition profile."
);
contract_wrapper!(
    "ResponseGenerationMethod",
    PyResponseGenerationMethod,
    ResponseGenerationMethod,
    "A validated, versioned response-generation recipe."
);

/// A validated material-specific neutron response set.
///
/// `validate()` is enforced on load. `folding_ready` additionally requires the
/// independent-review gate used before dose folding; a set that parses but has
/// not passed review loads successfully but reports `folding_ready == False`.
#[pyclass(frozen, name = "ResponseSet")]
struct PyResponseSet {
    inner: NeutronResponseSet,
}

#[pymethods]
impl PyResponseSet {
    #[getter]
    fn schema_version(&self) -> &str {
        &self.inner.schema_version
    }

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn qualification(&self) -> PyResult<String> {
        serde_json::to_value(self.inner.qualification)
            .map_err(reject)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| reject("qualification is not a string"))
    }

    /// True only when the set passes the independent-review folding gate.
    #[getter]
    fn folding_ready(&self) -> bool {
        self.inner.validate_for_folding().is_ok()
    }

    /// [lower, upper] transported-energy interval the grid must cover, eV.
    #[getter]
    fn transport_energy_range_ev(&self) -> (f64, f64) {
        self.inner.transport_energy_range_ev.into()
    }

    #[getter]
    fn energy_knot_count(&self) -> usize {
        self.inner.energy_ev.len()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(reject)
    }
}

/// Read and validate an explicit-nuclide material contract.
#[pyfunction]
fn load_material(path: PathBuf) -> PyResult<PyMaterial> {
    Ok(PyMaterial {
        inner: load_contract(path)?,
    })
}

/// Read and validate a fixed-source contract.
#[pyfunction]
fn load_fixed_source(path: PathBuf) -> PyResult<PyFixedSource> {
    Ok(PyFixedSource {
        inner: load_contract(path)?,
    })
}

/// Read and validate a component-definition profile.
#[pyfunction]
fn load_component_profile(path: PathBuf) -> PyResult<PyComponentProfile> {
    Ok(PyComponentProfile {
        inner: load_contract(path)?,
    })
}

/// Read and validate a response-generation method contract.
#[pyfunction]
fn load_response_generation_method(path: PathBuf) -> PyResult<PyResponseGenerationMethod> {
    Ok(PyResponseGenerationMethod {
        inner: load_contract(path)?,
    })
}

/// Read and validate a neutron response set (base validation only).
#[pyfunction]
fn load_response_set(path: PathBuf) -> PyResult<PyResponseSet> {
    Ok(PyResponseSet {
        inner: load_contract(path)?,
    })
}

/// NCTForge Python boundary over the authoritative Rust implementation.
///
/// Research software only: not a medical device, not commissioned, and not a
/// dose calculator. Transport actions stay unavailable until the same Rust
/// capability and evidence gates used by the CLI and GUI pass.
#[pymodule]
fn _nctforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("NctForgeError", m.py().get_type::<NctForgeError>())?;
    m.add_class::<PyBackend>()?;
    m.add_class::<PyStructure>()?;
    m.add_class::<PyCaseVerification>()?;
    m.add_class::<PyVerifiedCase>()?;
    m.add_class::<PyGeometry>()?;
    m.add_class::<PyGeneratedCase>()?;
    m.add_class::<PyArtifact>()?;
    m.add_class::<PyCaseManifest>()?;
    m.add_class::<PyMaterial>()?;
    m.add_class::<PyFixedSource>()?;
    m.add_class::<PyComponentProfile>()?;
    m.add_class::<PyResponseGenerationMethod>()?;
    m.add_class::<PyResponseSet>()?;
    m.add_function(wrap_pyfunction!(backends, m)?)?;
    m.add_function(wrap_pyfunction!(file_sha256, m)?)?;
    m.add_function(wrap_pyfunction!(generate_case, m)?)?;
    m.add_function(wrap_pyfunction!(verify_case, m)?)?;
    m.add_function(wrap_pyfunction!(load_case, m)?)?;
    m.add_function(wrap_pyfunction!(read_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(load_material, m)?)?;
    m.add_function(wrap_pyfunction!(load_fixed_source, m)?)?;
    m.add_function(wrap_pyfunction!(load_component_profile, m)?)?;
    m.add_function(wrap_pyfunction!(load_response_generation_method, m)?)?;
    m.add_function(wrap_pyfunction!(load_response_set, m)?)?;
    Ok(())
}
