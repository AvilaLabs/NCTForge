//! Deterministic generation of the material neutron response set from
//! receipt-bound NJOY production-HEATR PENDF pointwise tables.
//!
//! For each material nuclide the generator extracts the MF=3 pseudo-reaction
//! tables HEATR wrote into the production PENDF (MT=301 total KERMA, MT=443
//! kinematic total, and the configured partial KERMA channels), evaluates them
//! on the full pointwise union grid, and folds them with receipt- and
//! manifest-bound atom densities into Gy cm^2 response curves. The residual
//! component is constructed exactly as `total - boron - nitrogen` so the
//! response set's per-knot closure is an identity by construction.
//!
//! Energies outside a table's tabulated domain take the nearest boundary
//! value (constant extrapolation, the same convention ACE data uses below its
//! grid). Every boundary-held knot is counted and reported.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use openbnct_core::{ContentReference, DoseComponent};
use openbnct_openmc::{
    EvaluatedNeutronSourceSelectionDocument, NuclearDataManifest, OpenMcNeutronTransportDomain,
};
use openbnct_transport::{
    ComponentDefinitionProfile, ComponentEstimator, MaterialDefinition, NeutronResponseSet,
    ResponseGenerationMethod, ResponseSetQualification,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::photon_inventory::parse_evaluation_sections;
use crate::reaction_energy_balance::{ReactionTable, take_control_pub};
use crate::{
    NjoyDomainAwareSuitabilityReportDocument, NjoyExecutionReceiptDocument, NjoyTapePurpose,
    sha256_bytes,
};

pub const NJOY_RESPONSE_TABLE_GENERATION_SCHEMA: &str =
    "openbnct.njoy-response-table-generation/0.1.0";
pub const NJOY_RESPONSE_SET_REVIEW_SCHEMA: &str = "openbnct.njoy-response-set-review/0.1.0";
pub const NEUTRON_RESPONSE_SET_SCHEMA: &str = "openbnct.neutron-response-set/0.1.0";

/// CODATA 2018 Avogadro constant, exact by SI definition.
pub const AVOGADRO_CONSTANT_PER_MOL: f64 = 6.022_140_76e23;
/// CODATA 2018 neutron mass in unified atomic mass units. ENDF atomic weight
/// ratios are expressed in units of the neutron mass, so the molar mass in
/// g/mol is `atomic_weight_ratio * NEUTRON_MASS_U`.
pub const NEUTRON_MASS_U: f64 = 1.008_664_915_95;
pub const CM2_PER_BARN: f64 = 1.0e-24;

#[derive(Debug, Error)]
pub enum ResponseTableError {
    #[error("I/O operation failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("response-generation method is invalid: {0}")]
    InvalidMethod(String),
    #[error("component-definition profile is invalid: {0}")]
    InvalidComponentProfile(String),
    #[error("{0} content reference does not match the supplied document")]
    InputBindingMismatch(&'static str),
    #[error("nuclear data manifest is invalid for this material: {0}")]
    Manifest(String),
    #[error("receipt run for nuclide {0} is missing")]
    MissingRun(String),
    #[error("receipt run for nuclide {0} exited with code {1}")]
    FailedRun(String, i32),
    #[error("receipt run for nuclide {0} has no production_heatr_pendf tape")]
    MissingProductionPendf(String),
    #[error("digest mismatch for {path}")]
    DigestMismatch { path: PathBuf },
    #[error("nuclide {nuclide} production PENDF is missing MF=3 MT={mt}")]
    MissingKermaSection { nuclide: String, mt: u16 },
    #[error("nuclide {nuclide} production PENDF section parsing failed: {reason}")]
    SectionParse { nuclide: String, reason: String },
    #[error("nuclide {nuclide} MF=3 MT={mt} tabulation is invalid: {reason}")]
    InvalidTabulation {
        nuclide: String,
        mt: u16,
        reason: String,
    },
    #[error("nuclide {nuclide} MF=3 MT={mt} uses a non linear-linear interpolation law")]
    UnsupportedInterpolation { nuclide: String, mt: u16 },
    #[error("nuclide {nuclide} MF=3 MT={mt} section carries unexpected trailing records")]
    TrailingSectionRecords { nuclide: String, mt: u16 },
    #[error("nuclide {0} has no neutron table in the bound nuclear data manifest")]
    MissingManifestTable(String),
    #[error("receipt nuclide {0} is not a constituent of the bound material")]
    UnexpectedRunNuclide(String),
    #[error(
        "partial KERMA channel for component {component:?} is inconsistent between the method and the component profile"
    )]
    PartialChannelMismatch { component: DoseComponent },
    #[error("folded response tables cannot cover the declared transport energy range")]
    TransportRangeNotCovered,
    #[error("residual component is negative at energy-grid knot {index} ({value} Gy cm^2)")]
    NegativeResidual { index: usize, value: f64 },
    #[error("generated neutron response set failed validation: {0}")]
    ResponseSetInvalid(String),
    #[error("generated report failed validation: {0}")]
    InvalidReport(String),
    #[error("output file already exists: {0}")]
    OutputExists(PathBuf),
    #[error("regenerated {label} does not match the supplied artifact")]
    RegenerationMismatch { label: &'static str },
    #[error("reviewed response set failed validation: {0}")]
    ReviewedSetInvalid(String),
}

/// Bound inputs for response-table generation and verification.
pub struct NjoyResponseTableInputs<'a> {
    pub material: &'a MaterialDefinition,
    pub material_bytes: &'a [u8],
    pub component_profile: &'a ComponentDefinitionProfile,
    pub component_profile_bytes: &'a [u8],
    pub method: &'a ResponseGenerationMethod,
    pub method_bytes: &'a [u8],
    pub nuclear_data: &'a NuclearDataManifest,
    pub nuclear_data_bytes: &'a [u8],
    pub transport_domain: &'a OpenMcNeutronTransportDomain,
    pub transport_domain_bytes: &'a [u8],
    pub selection: &'a EvaluatedNeutronSourceSelectionDocument,
    pub domain_aware: &'a NjoyDomainAwareSuitabilityReportDocument,
    pub execution: &'a NjoyExecutionReceiptDocument,
    pub execution_directory: &'a Path,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NjoyResponseTableGeneration {
    pub response_set: NeutronResponseSet,
    pub report: NjoyResponseTableGenerationReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NjoyResponseTableGenerationResult {
    pub generation: NjoyResponseTableGeneration,
    pub response_set_path: PathBuf,
    pub response_set_sha256: String,
    pub report_path: PathBuf,
    pub report_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NjoyResponseTableGenerationReport {
    #[serde(deserialize_with = "openbnct_core::deserialize_contract_id")]
    pub schema_version: String,
    pub id: String,
    pub qualification: NjoyResponseTableQualification,
    pub material: ContentReference,
    pub component_profile: ContentReference,
    pub generation_method: ContentReference,
    pub evaluated_source_selection: ContentReference,
    pub nuclear_data_manifest: ContentReference,
    pub neutron_transport_domain: ContentReference,
    pub domain_aware_suitability: ContentReference,
    pub execution_receipt: ContentReference,
    pub response_set: ContentReference,
    pub atom_density_basis: String,
    pub avogadro_constant_per_mol: f64,
    pub neutron_mass_u: f64,
    pub joule_per_ev: f64,
    pub cm2_per_barn: f64,
    pub transport_energy_range_ev: [f64; 2],
    pub union_grid_knot_count: u64,
    pub boundary_held_knot_count: u64,
    pub nuclides: Vec<NuclideTableExtraction>,
    pub partial_channels: Vec<PartialChannelExtraction>,
    pub residual: ResidualComponentRecord,
    pub carried_findings: CarriedFindingSummary,
    pub finding_disposition: ResponseTableFindingDisposition,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NjoyResponseTableQualification {
    TablesGeneratedUnreviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseTableFindingDisposition {
    /// Every upstream suitability finding remains recorded in the bound
    /// domain-aware report; none is waived or removed by this generation.
    CarriedInProvenanceFindingsRetained,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuclideTableExtraction {
    pub nuclide: String,
    pub endf_mat: u16,
    pub mass_fraction: f64,
    pub atomic_weight_ratio: f64,
    pub atoms_per_kg: f64,
    pub production_pendf_path: String,
    pub production_pendf_sha256: String,
    pub tables: Vec<ExtractedTableRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedTableRecord {
    pub response_mt: u16,
    pub role: ExtractedTableRole,
    pub point_count: u64,
    pub energy_bounds_ev: [f64; 2],
    pub interpolation_laws: Vec<i64>,
    pub boundary_held_knots: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractedTableRole {
    TotalKerma,
    KinematicTotal,
    PartialKerma,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialChannelExtraction {
    pub component: DoseComponent,
    pub nuclide: String,
    pub reaction_mt: u16,
    pub heatr_partial_kerma_mt: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualComponentRecord {
    pub component: DoseComponent,
    pub subtracted_components: Vec<DoseComponent>,
    pub minimum_residual_gy_cm2: f64,
    pub maximum_residual_fraction_of_total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedFindingSummary {
    pub in_domain_kinematic_violation_count: u64,
    pub rejecting_processor_finding_count: u64,
    pub informational_processor_finding_count: u64,
    pub source_format_finding_count: u64,
    pub nuclides_with_findings: Vec<String>,
}

struct FoldedTable {
    table: ReactionTable,
    role: ExtractedTableRole,
    response_mt: u16,
    boundary_held_knots: u64,
}

struct NuclideFold {
    nuclide: String,
    endf_mat: u16,
    mass_fraction: f64,
    atomic_weight_ratio: f64,
    atoms_per_kg: f64,
    pendf_path: String,
    pendf_sha256: String,
    total: FoldedTable,
    kinematic: FoldedTable,
    partials: Vec<(DoseComponent, u16, u16, FoldedTable)>,
}

impl NjoyResponseTableGeneration {
    /// Extract the receipt-bound PENDF KERMA tables, fold them into component
    /// response curves on the full pointwise union grid, and assemble the
    /// unreviewed response set plus its generation report.
    pub fn generate(
        inputs: &NjoyResponseTableInputs<'_>,
        response_set_id: &str,
        report_id: &str,
    ) -> Result<Self, ResponseTableError> {
        let method = inputs.method;
        method
            .validate()
            .map_err(|error| ResponseTableError::InvalidMethod(error.to_string()))?;
        inputs
            .component_profile
            .validate()
            .map_err(|error| ResponseTableError::InvalidComponentProfile(error.to_string()))?;

        let material_ref = content_reference(&inputs.material.id, inputs.material_bytes);
        let profile_ref =
            content_reference(&inputs.component_profile.id, inputs.component_profile_bytes);
        let method_ref = content_reference(&method.id, inputs.method_bytes);
        let manifest_ref = content_reference(&inputs.nuclear_data.id, inputs.nuclear_data_bytes);
        let domain_ref =
            content_reference(&inputs.transport_domain.id, inputs.transport_domain_bytes);
        let selection_ref = ContentReference {
            id: inputs.selection.selection.id.clone(),
            sha256: inputs.selection.sha256.clone(),
        };
        let domain_aware_ref = ContentReference {
            id: inputs.domain_aware.report.id.clone(),
            sha256: inputs.domain_aware.sha256.clone(),
        };
        let execution_ref = ContentReference {
            id: inputs.execution.receipt.id.clone(),
            sha256: inputs.execution.sha256.clone(),
        };

        if method.material != material_ref {
            return Err(ResponseTableError::InputBindingMismatch("method.material"));
        }
        if method.component_profile != profile_ref {
            return Err(ResponseTableError::InputBindingMismatch(
                "method.component_profile",
            ));
        }
        if inputs.transport_domain.material != material_ref {
            return Err(ResponseTableError::InputBindingMismatch("domain.material"));
        }
        if inputs.transport_domain.nuclear_data_manifest != manifest_ref {
            return Err(ResponseTableError::InputBindingMismatch(
                "domain.nuclear_data_manifest",
            ));
        }
        if inputs.domain_aware.report.execution_receipt != execution_ref {
            return Err(ResponseTableError::InputBindingMismatch(
                "domain_aware.execution_receipt",
            ));
        }
        if inputs.domain_aware.report.neutron_transport_domain != domain_ref {
            return Err(ResponseTableError::InputBindingMismatch(
                "domain_aware.neutron_transport_domain",
            ));
        }
        inputs
            .nuclear_data
            .validate_for_material(inputs.material)
            .map_err(|error| ResponseTableError::Manifest(error.to_string()))?;

        let profile_partials = profile_partial_channels(inputs.component_profile);
        for partial in &method.heatr.partials {
            let expected = (
                partial.component,
                partial.nuclide.as_str(),
                partial.reaction_mt,
                partial.heatr_partial_kerma_mt,
            );
            if !profile_partials.contains(&(
                expected.0,
                expected.1.to_string(),
                expected.2,
                expected.3,
            )) {
                return Err(ResponseTableError::PartialChannelMismatch {
                    component: partial.component,
                });
            }
        }

        let transport_range = inputs.transport_domain.energy_range_ev;
        let mut folds = extract_folds(inputs)?;

        let grid = union_response_grid(&folds, transport_range)?;
        let curves = fold_component_curves(&mut folds, &grid, method.joule_per_ev)?;

        let response_set = NeutronResponseSet {
            schema_version: NEUTRON_RESPONSE_SET_SCHEMA.to_string(),
            id: response_set_id.to_string(),
            qualification: ResponseSetQualification::GeneratedUnreviewed,
            component_profile: profile_ref.clone(),
            material: material_ref.clone(),
            nuclear_data_manifest: manifest_ref.clone(),
            generation_method: method_ref.clone(),
            independent_review: None,
            transport_energy_range_ev: transport_range,
            energy_ev: grid,
            unit: method.response_unit,
            interpolation: method.interpolation,
            boron_gy_cm2: curves.boron,
            nitrogen_gy_cm2: curves.nitrogen,
            hydrogen_gy_cm2: curves.hydrogen,
            total_neutron_gy_cm2: curves.total_neutron,
        };
        response_set
            .validate()
            .map_err(|error| ResponseTableError::ResponseSetInvalid(error.to_string()))?;

        let response_set_bytes = canonical_bytes(&response_set)?;
        let response_set_ref = ContentReference {
            id: response_set.id.clone(),
            sha256: sha256_bytes(&response_set_bytes),
        };

        let mut nuclide_records = Vec::with_capacity(folds.len());
        let mut boundary_held_total = 0_u64;
        for fold in &folds {
            let mut tables = Vec::with_capacity(2 + fold.partials.len());
            for folded in std::iter::once(&fold.total)
                .chain(std::iter::once(&fold.kinematic))
                .chain(fold.partials.iter().map(|(_, _, _, partial)| partial))
            {
                let (lower, upper) = folded.table.energy_bounds();
                boundary_held_total += folded.boundary_held_knots;
                tables.push(ExtractedTableRecord {
                    response_mt: folded.response_mt,
                    role: folded.role,
                    point_count: folded.table.knot_energies().count() as u64,
                    energy_bounds_ev: [lower, upper],
                    interpolation_laws: folded.table.interpolation_laws().collect(),
                    boundary_held_knots: folded.boundary_held_knots,
                });
            }
            nuclide_records.push(NuclideTableExtraction {
                nuclide: fold.nuclide.clone(),
                endf_mat: fold.endf_mat,
                mass_fraction: fold.mass_fraction,
                atomic_weight_ratio: fold.atomic_weight_ratio,
                atoms_per_kg: fold.atoms_per_kg,
                production_pendf_path: fold.pendf_path.clone(),
                production_pendf_sha256: fold.pendf_sha256.clone(),
                tables,
            });
        }

        let report = NjoyResponseTableGenerationReport {
            schema_version: NJOY_RESPONSE_TABLE_GENERATION_SCHEMA.to_string(),
            id: report_id.to_string(),
            qualification: NjoyResponseTableQualification::TablesGeneratedUnreviewed,
            material: material_ref,
            component_profile: profile_ref,
            generation_method: method_ref,
            evaluated_source_selection: selection_ref,
            nuclear_data_manifest: manifest_ref,
            neutron_transport_domain: domain_ref,
            domain_aware_suitability: domain_aware_ref,
            execution_receipt: execution_ref,
            response_set: response_set_ref,
            atom_density_basis: "open_mc_hdf5_atomic_weight_ratios".to_string(),
            avogadro_constant_per_mol: AVOGADRO_CONSTANT_PER_MOL,
            neutron_mass_u: NEUTRON_MASS_U,
            joule_per_ev: method.joule_per_ev,
            cm2_per_barn: CM2_PER_BARN,
            transport_energy_range_ev: transport_range,
            union_grid_knot_count: response_set.energy_ev.len() as u64,
            boundary_held_knot_count: boundary_held_total,
            nuclides: nuclide_records,
            partial_channels: method
                .heatr
                .partials
                .iter()
                .map(|partial| PartialChannelExtraction {
                    component: partial.component,
                    nuclide: partial.nuclide.clone(),
                    reaction_mt: partial.reaction_mt,
                    heatr_partial_kerma_mt: partial.heatr_partial_kerma_mt,
                })
                .collect(),
            residual: ResidualComponentRecord {
                component: method.heatr.residual_component,
                subtracted_components: method.heatr.residual_subtract_components.clone(),
                minimum_residual_gy_cm2: curves.minimum_residual,
                maximum_residual_fraction_of_total: curves.maximum_residual_fraction,
            },
            carried_findings: carried_findings(inputs.domain_aware),
            finding_disposition:
                ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained,
            caveats: vec![
                "response curves carry every transported-photon suitability finding bound in \
                 domain_aware_suitability; none are waived"
                    .to_string(),
                "nuclides without transported photon-production data deposit photon energy \
                 locally in the total-KERMA table; this is a documented data-coverage \
                 property of the evaluation, not a processor artifact"
                    .to_string(),
            ],
        };
        report.validate()?;

        Ok(Self {
            response_set,
            report,
        })
    }

    pub fn write_new(
        &self,
        response_set_path: &Path,
        report_path: &Path,
    ) -> Result<NjoyResponseTableGenerationResult, ResponseTableError> {
        for path in [response_set_path, report_path] {
            if path.exists() {
                return Err(ResponseTableError::OutputExists(path.to_path_buf()));
            }
        }
        let response_set_bytes = canonical_bytes(&self.response_set)?;
        let response_set_sha256 = write_new_file(response_set_path, &response_set_bytes)?;
        let report_bytes = canonical_bytes(&self.report)?;
        let report_sha256 = write_new_file(report_path, &report_bytes)?;
        Ok(NjoyResponseTableGenerationResult {
            generation: self.clone(),
            response_set_path: response_set_path.to_path_buf(),
            response_set_sha256,
            report_path: report_path.to_path_buf(),
            report_sha256,
        })
    }
}

/// Folded component curves plus residual diagnostics.
#[derive(Debug)]
struct ComponentCurves {
    boron: Vec<f64>,
    nitrogen: Vec<f64>,
    hydrogen: Vec<f64>,
    total_neutron: Vec<f64>,
    minimum_residual: f64,
    maximum_residual_fraction: f64,
}

/// Extract and verify the receipt-bound production-HEATR PENDF KERMA tables
/// for every material nuclide.
fn extract_folds(
    inputs: &NjoyResponseTableInputs<'_>,
) -> Result<Vec<NuclideFold>, ResponseTableError> {
    let method = inputs.method;
    // Map each receipt run to its nuclide and require a 1:1 cover of the
    // material composition.
    let runs: BTreeMap<&str, &crate::execution::NjoyExecutionRun> = inputs
        .execution
        .receipt
        .runs
        .iter()
        .map(|run| (run.nuclide.as_str(), run))
        .collect();
    for run in &inputs.execution.receipt.runs {
        if !inputs
            .material
            .nuclides
            .iter()
            .any(|nuclide| nuclide.name == run.nuclide)
        {
            return Err(ResponseTableError::UnexpectedRunNuclide(
                run.nuclide.clone(),
            ));
        }
    }

    let mut folds = Vec::with_capacity(inputs.material.nuclides.len());
    for nuclide in &inputs.material.nuclides {
        let run = runs
            .get(nuclide.name.as_str())
            .ok_or_else(|| ResponseTableError::MissingRun(nuclide.name.clone()))?;
        if run.exit_code != 0 {
            return Err(ResponseTableError::FailedRun(
                nuclide.name.clone(),
                run.exit_code,
            ));
        }
        let tape = run
            .output_tapes
            .iter()
            .find(|tape| tape.purpose == NjoyTapePurpose::ProductionHeatrPendf)
            .ok_or_else(|| ResponseTableError::MissingProductionPendf(nuclide.name.clone()))?;
        let tape_path = inputs.execution_directory.join(&tape.artifact.path);
        let tape_bytes = read_regular_file(&tape_path)?;
        if sha256_bytes(&tape_bytes) != tape.artifact.sha256 {
            return Err(ResponseTableError::DigestMismatch { path: tape_path });
        }

        let mut selected = vec![method.heatr.total_kerma_mt, method.heatr.kinematic_total_mt];
        for partial in &method.heatr.partials {
            if partial.nuclide == nuclide.name {
                selected.push(partial.heatr_partial_kerma_mt);
            }
        }
        let selected: Vec<(u16, u16)> = selected.iter().map(|mt| (3, *mt)).collect();
        let sections =
            parse_evaluation_sections(&tape_bytes, run.endf_mat, &selected).map_err(|error| {
                ResponseTableError::SectionParse {
                    nuclide: nuclide.name.clone(),
                    reason: error.to_string(),
                }
            })?;

        let mut tables = BTreeMap::new();
        for section in &sections {
            let mut cursor = 0_usize;
            let _section_head = take_control_pub(section, &mut cursor).map_err(|error| {
                ResponseTableError::InvalidTabulation {
                    nuclide: nuclide.name.clone(),
                    mt: section.reaction_mt,
                    reason: error.to_string(),
                }
            })?;
            let (_, table) = ReactionTable::parse(section, &mut cursor).map_err(|error| {
                ResponseTableError::InvalidTabulation {
                    nuclide: nuclide.name.clone(),
                    mt: section.reaction_mt,
                    reason: error.to_string(),
                }
            })?;
            if cursor != section.records.len() {
                return Err(ResponseTableError::TrailingSectionRecords {
                    nuclide: nuclide.name.clone(),
                    mt: section.reaction_mt,
                });
            }
            if !table.is_linear_linear() {
                return Err(ResponseTableError::UnsupportedInterpolation {
                    nuclide: nuclide.name.clone(),
                    mt: section.reaction_mt,
                });
            }
            tables.insert(section.reaction_mt, table);
        }

        let awr = inputs
            .nuclear_data
            .neutron_tables
            .iter()
            .find(|table| table.nuclide == nuclide.name)
            .ok_or_else(|| ResponseTableError::MissingManifestTable(nuclide.name.clone()))?
            .atomic_weight_ratio;
        let atoms_per_kg =
            nuclide.mass_fraction * 1.0e3 * AVOGADRO_CONSTANT_PER_MOL / (awr * NEUTRON_MASS_U);

        let mut take =
            |mt: u16, role: ExtractedTableRole| -> Result<FoldedTable, ResponseTableError> {
                let table = tables
                    .remove(&mt)
                    .ok_or(ResponseTableError::MissingKermaSection {
                        nuclide: nuclide.name.clone(),
                        mt,
                    })?;
                Ok(FoldedTable {
                    table,
                    role,
                    response_mt: mt,
                    boundary_held_knots: 0,
                })
            };
        let total = take(method.heatr.total_kerma_mt, ExtractedTableRole::TotalKerma)?;
        let kinematic = take(
            method.heatr.kinematic_total_mt,
            ExtractedTableRole::KinematicTotal,
        )?;
        let mut partials = Vec::new();
        for partial in &method.heatr.partials {
            if partial.nuclide == nuclide.name {
                partials.push((
                    partial.component,
                    partial.reaction_mt,
                    partial.heatr_partial_kerma_mt,
                    take(
                        partial.heatr_partial_kerma_mt,
                        ExtractedTableRole::PartialKerma,
                    )?,
                ));
            }
        }

        folds.push(NuclideFold {
            nuclide: nuclide.name.clone(),
            endf_mat: run.endf_mat,
            mass_fraction: nuclide.mass_fraction,
            atomic_weight_ratio: awr,
            atoms_per_kg,
            pendf_path: tape.artifact.path.clone(),
            pendf_sha256: tape.artifact.sha256.clone(),
            total,
            kinematic,
            partials,
        });
    }
    Ok(folds)
}

/// Full pointwise union grid over every folded table, extended at the ends
/// just enough to cover the declared transport range.
fn union_response_grid(
    folds: &[NuclideFold],
    transport_range: [f64; 2],
) -> Result<Vec<f64>, ResponseTableError> {
    let mut grid: Vec<f64> = Vec::new();
    for fold in folds {
        grid.extend(fold.total.table.knot_energies());
        for (_, _, _, partial) in &fold.partials {
            grid.extend(partial.table.knot_energies());
        }
    }
    grid.sort_by(f64::total_cmp);
    grid.dedup();
    if grid.len() < 2 {
        return Err(ResponseTableError::TransportRangeNotCovered);
    }
    if grid[0] > transport_range[0] {
        grid.insert(0, transport_range[0]);
    }
    if *grid.last().expect("nonempty grid") < transport_range[1] {
        grid.push(transport_range[1]);
    }
    Ok(grid)
}

/// Fold the extracted KERMA tables with bound atom densities into Gy cm^2
/// component response curves. Inside a table's tabulated domain it is
/// interpolated by its own law; outside, the nearest boundary value is held
/// constant and counted.
fn fold_component_curves(
    folds: &mut [NuclideFold],
    grid: &[f64],
    joule_per_ev: f64,
) -> Result<ComponentCurves, ResponseTableError> {
    fn evaluate(folded: &mut FoldedTable, energy: f64) -> f64 {
        match folded.table.evaluate(energy) {
            Some(value) => value,
            None => {
                folded.boundary_held_knots += 1;
                let (lower, upper) = folded.table.energy_bounds();
                let boundary = if energy < lower { lower } else { upper };
                folded
                    .table
                    .evaluate(boundary)
                    .expect("boundary energy is tabulated")
            }
        }
    }

    let knot_count = grid.len();
    let mut boron = vec![0.0_f64; knot_count];
    let mut nitrogen = vec![0.0_f64; knot_count];
    let mut total_neutron = vec![0.0_f64; knot_count];
    let conversion = joule_per_ev * CM2_PER_BARN;

    for fold in folds.iter_mut() {
        let scale = fold.atoms_per_kg * conversion;
        for (index, energy) in grid.iter().copied().enumerate() {
            total_neutron[index] += scale * evaluate(&mut fold.total, energy);
            for (component, _, _, partial) in &mut fold.partials {
                let value = scale * evaluate(partial, energy);
                match component {
                    DoseComponent::Boron => boron[index] += value,
                    DoseComponent::Nitrogen => nitrogen[index] += value,
                    _ => unreachable!("method validation restricts partial components"),
                }
            }
        }
    }

    let mut hydrogen = vec![0.0_f64; knot_count];
    let mut minimum_residual = f64::INFINITY;
    let mut maximum_residual_fraction = 0.0_f64;
    for index in 0..knot_count {
        let residual = total_neutron[index] - boron[index] - nitrogen[index];
        if residual < 0.0 {
            return Err(ResponseTableError::NegativeResidual {
                index,
                value: residual,
            });
        }
        minimum_residual = minimum_residual.min(residual);
        if total_neutron[index] > 0.0 {
            maximum_residual_fraction =
                maximum_residual_fraction.max(residual / total_neutron[index]);
        }
        hydrogen[index] = residual;
    }

    Ok(ComponentCurves {
        boron,
        nitrogen,
        hydrogen,
        total_neutron,
        minimum_residual,
        maximum_residual_fraction,
    })
}

/// Loads a serialized `NeutronResponseSet` and validates it.
pub fn load_response_set(path: &Path) -> Result<(NeutronResponseSet, Vec<u8>), ResponseTableError> {
    let bytes = read_regular_file(path)?;
    let set: NeutronResponseSet = serde_json::from_slice(&bytes)?;
    set.validate()
        .map_err(|error| ResponseTableError::ResponseSetInvalid(error.to_string()))?;
    Ok((set, bytes))
}

/// Loads a serialized generation report and validates it.
pub fn load_generation_report(
    path: &Path,
) -> Result<(NjoyResponseTableGenerationReport, Vec<u8>), ResponseTableError> {
    let bytes = read_regular_file(path)?;
    let report: NjoyResponseTableGenerationReport = serde_json::from_slice(&bytes)?;
    report.validate()?;
    Ok((report, bytes))
}

impl NjoyResponseTableGenerationReport {
    fn validate(&self) -> Result<(), ResponseTableError> {
        let invalid = |message: &str| ResponseTableError::InvalidReport(message.to_string());
        if !openbnct_core::schema_matches(
            &self.schema_version,
            NJOY_RESPONSE_TABLE_GENERATION_SCHEMA,
        ) {
            return Err(invalid("unsupported schema_version"));
        }
        if self.id.trim().is_empty() {
            return Err(invalid("id is empty"));
        }
        if self.qualification != NjoyResponseTableQualification::TablesGeneratedUnreviewed {
            return Err(invalid("unexpected qualification"));
        }
        for (label, reference) in [
            ("material", &self.material),
            ("component_profile", &self.component_profile),
            ("generation_method", &self.generation_method),
            (
                "evaluated_source_selection",
                &self.evaluated_source_selection,
            ),
            ("nuclear_data_manifest", &self.nuclear_data_manifest),
            ("neutron_transport_domain", &self.neutron_transport_domain),
            ("domain_aware_suitability", &self.domain_aware_suitability),
            ("execution_receipt", &self.execution_receipt),
            ("response_set", &self.response_set),
        ] {
            reference.validate().map_err(|_| invalid(label))?;
        }
        if self.nuclides.is_empty() {
            return Err(invalid("no nuclide extractions recorded"));
        }
        if self.union_grid_knot_count < 2 {
            return Err(invalid("union grid too small"));
        }
        if self.finding_disposition
            != ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained
        {
            return Err(invalid("findings must be carried, not waived"));
        }
        Ok(())
    }
}

/// Review report for the deterministic verification path. Under the ADR 0031
/// maintainer disposition this in-house reproducible verification is the
/// evidence bound into `NeutronResponseSet::independent_review`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NjoyResponseSetReviewReport {
    #[serde(deserialize_with = "openbnct_core::deserialize_contract_id")]
    pub schema_version: String,
    pub id: String,
    pub scope: NjoyResponseSetReviewScope,
    pub result: NjoyResponseSetReviewResult,
    pub response_set_unreviewed: ContentReference,
    pub generation_report: ContentReference,
    pub regenerated_response_set_digest_match: bool,
    pub regenerated_report_digest_match: bool,
    pub response_set_validation: NjoyResponseSetCheckStatus,
    pub closure_check: NjoyResponseSetCheckStatus,
    pub transport_domain_coverage: NjoyResponseSetCheckStatus,
    pub nonnegative_curves: NjoyResponseSetCheckStatus,
    pub carried_in_domain_kinematic_violation_count: u64,
    pub carried_rejecting_finding_count: u64,
    pub finding_disposition: ResponseTableFindingDisposition,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NjoyResponseSetReviewScope {
    /// Deterministic regeneration from the receipt-bound PENDF sources plus
    /// re-validation of every response-set invariant. This is the project's
    /// own reproducible verification, not an external physics review of NJOY
    /// HEATR or of the evaluated data.
    DeterministicRegenerationAndInvariantVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NjoyResponseSetReviewResult {
    VerificationPassed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NjoyResponseSetCheckStatus {
    Passed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NjoyResponseSetReviewDocument {
    pub report: NjoyResponseSetReviewReport,
    pub reviewed_response_set: NeutronResponseSet,
    pub review_sha256: String,
    pub reviewed_set_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NjoyResponseSetReviewWriteResult {
    pub document: NjoyResponseSetReviewDocument,
    pub review_path: PathBuf,
    pub reviewed_set_path: PathBuf,
}

impl NjoyResponseSetReviewReport {
    /// Regenerate the response set and generation report from the bound
    /// sources, require identical reproduction, re-validate every invariant,
    /// then emit this review report and the independently reviewed response
    /// set that binds it.
    pub fn verify(
        inputs: &NjoyResponseTableInputs<'_>,
        response_set: &NeutronResponseSet,
        response_set_bytes: &[u8],
        generation_report: &NjoyResponseTableGenerationReport,
        generation_report_bytes: &[u8],
        review_id: &str,
    ) -> Result<(Self, NeutronResponseSet), ResponseTableError> {
        let regenerated =
            NjoyResponseTableGeneration::generate(inputs, &response_set.id, &generation_report.id)?;
        if regenerated.response_set != *response_set {
            return Err(ResponseTableError::RegenerationMismatch {
                label: "response_set",
            });
        }
        if regenerated.report != *generation_report {
            return Err(ResponseTableError::RegenerationMismatch { label: "report" });
        }

        let report = Self {
            schema_version: NJOY_RESPONSE_SET_REVIEW_SCHEMA.to_string(),
            id: review_id.to_string(),
            scope: NjoyResponseSetReviewScope::DeterministicRegenerationAndInvariantVerification,
            result: NjoyResponseSetReviewResult::VerificationPassed,
            response_set_unreviewed: ContentReference {
                id: response_set.id.clone(),
                sha256: sha256_bytes(response_set_bytes),
            },
            generation_report: ContentReference {
                id: generation_report.id.clone(),
                sha256: sha256_bytes(generation_report_bytes),
            },
            regenerated_response_set_digest_match: true,
            regenerated_report_digest_match: true,
            response_set_validation: NjoyResponseSetCheckStatus::Passed,
            closure_check: NjoyResponseSetCheckStatus::Passed,
            transport_domain_coverage: NjoyResponseSetCheckStatus::Passed,
            nonnegative_curves: NjoyResponseSetCheckStatus::Passed,
            carried_in_domain_kinematic_violation_count: inputs
                .domain_aware
                .report
                .in_domain_kinematic_violation_count,
            carried_rejecting_finding_count: inputs
                .domain_aware
                .report
                .rejecting_processor_finding_count,
            finding_disposition:
                ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained,
            caveats: vec![
                "this review is a deterministic regeneration by the project toolchain; it is \
                 not an external review of the evaluation or of NJOY HEATR physics"
                    .to_string(),
                "all upstream suitability findings remain carried in provenance; none are \
                 waived by this qualification"
                    .to_string(),
            ],
        };
        report.validate()?;

        let review_bytes = canonical_bytes(&report)?;
        let reviewed_set = NeutronResponseSet {
            qualification: ResponseSetQualification::IndependentlyReviewed,
            independent_review: Some(ContentReference {
                id: report.id.clone(),
                sha256: sha256_bytes(&review_bytes),
            }),
            ..regenerated.response_set
        };
        reviewed_set
            .validate_for_folding()
            .map_err(|error| ResponseTableError::ReviewedSetInvalid(error.to_string()))?;

        Ok((report, reviewed_set))
    }

    fn validate(&self) -> Result<(), ResponseTableError> {
        let invalid = |message: &str| ResponseTableError::InvalidReport(message.to_string());
        if !openbnct_core::schema_matches(&self.schema_version, NJOY_RESPONSE_SET_REVIEW_SCHEMA) {
            return Err(invalid("unsupported schema_version"));
        }
        if self.id.trim().is_empty() {
            return Err(invalid("id is empty"));
        }
        if self.result != NjoyResponseSetReviewResult::VerificationPassed {
            return Err(invalid("review did not pass"));
        }
        if !self.regenerated_response_set_digest_match || !self.regenerated_report_digest_match {
            return Err(invalid("digest match flags must be true"));
        }
        Ok(())
    }
}

fn profile_partial_channels(
    profile: &ComponentDefinitionProfile,
) -> Vec<(DoseComponent, String, u16, u16)> {
    profile
        .components
        .iter()
        .filter_map(|rule| match &rule.estimator {
            ComponentEstimator::NjoyPartialKermaFluenceFold {
                nuclide,
                reaction_mt,
                heatr_partial_kerma_mt,
                ..
            } => Some((
                rule.component,
                nuclide.clone(),
                *reaction_mt,
                *heatr_partial_kerma_mt,
            )),
            _ => None,
        })
        .collect()
}

fn carried_findings(report: &NjoyDomainAwareSuitabilityReportDocument) -> CarriedFindingSummary {
    let nuclides_with_findings = report
        .report
        .runs
        .iter()
        .filter(|run| {
            run.in_domain_diagnostic_violation_count > 0
                || !run.processor_findings.is_empty()
                || !run.source_format_findings.is_empty()
        })
        .map(|run| run.nuclide.clone())
        .collect();
    CarriedFindingSummary {
        in_domain_kinematic_violation_count: report.report.in_domain_kinematic_violation_count,
        rejecting_processor_finding_count: report.report.rejecting_processor_finding_count,
        informational_processor_finding_count: report.report.informational_processor_finding_count,
        source_format_finding_count: report.report.source_format_finding_count,
        nuclides_with_findings,
    }
}

impl NjoyResponseSetReviewDocument {
    /// Write the review report and the reviewed response set. The report is
    /// serialized first so the reviewed set's `independent_review` reference
    /// binds the exact bytes written.
    pub fn write_new(
        report: &NjoyResponseSetReviewReport,
        reviewed_set: &NeutronResponseSet,
        review_path: &Path,
        reviewed_set_path: &Path,
    ) -> Result<NjoyResponseSetReviewWriteResult, ResponseTableError> {
        for path in [review_path, reviewed_set_path] {
            if path.exists() {
                return Err(ResponseTableError::OutputExists(path.to_path_buf()));
            }
        }
        let review_bytes = canonical_bytes(report)?;
        let review_sha256 = write_new_file(review_path, &review_bytes)?;
        let reviewed_bytes = canonical_bytes(reviewed_set)?;
        let reviewed_set_sha256 = write_new_file(reviewed_set_path, &reviewed_bytes)?;
        Ok(NjoyResponseSetReviewWriteResult {
            document: NjoyResponseSetReviewDocument {
                report: report.clone(),
                reviewed_response_set: reviewed_set.clone(),
                review_sha256,
                reviewed_set_sha256,
            },
            review_path: review_path.to_path_buf(),
            reviewed_set_path: reviewed_set_path.to_path_buf(),
        })
    }
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>, ResponseTableError> {
    fs_read(path).map_err(|source| ResponseTableError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn fs_read(path: &Path) -> std::io::Result<Vec<u8>> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    std::fs::read(path)
}

fn content_reference(id: &str, bytes: &[u8]) -> ContentReference {
    ContentReference {
        id: id.to_string(),
        sha256: sha256_bytes(bytes),
    }
}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ResponseTableError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<String, ResponseTableError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                ResponseTableError::OutputExists(path.to_path_buf())
            } else {
                ResponseTableError::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| ResponseTableError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(sha256_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photon_inventory::{EndfRecord, ParsedSection};

    fn control(c1: f64, c2: f64, l1: i64, l2: i64, n1: i64, n2: i64) -> EndfRecord {
        EndfRecord {
            values: [
                Some(c1),
                Some(c2),
                Some(l1 as f64),
                Some(l2 as f64),
                Some(n1 as f64),
                Some(n2 as f64),
            ],
            c1,
            l1,
            l2,
            n1,
            n2,
            is_control: true,
        }
    }

    fn data(words: &[f64]) -> Vec<EndfRecord> {
        words
            .chunks(6)
            .map(|chunk| {
                let mut values = [None; 6];
                for (slot, word) in values.iter_mut().zip(chunk.iter()) {
                    *slot = Some(*word);
                }
                EndfRecord {
                    values,
                    c1: 0.0,
                    l1: 0,
                    l2: 0,
                    n1: 0,
                    n2: 0,
                    is_control: false,
                }
            })
            .collect()
    }

    /// Builds a PENDF-style MF=3 pseudo-reaction section: ZA/AWR head record
    /// followed by a linear-linear TAB1 over `points`.
    fn kerma_section(reaction_mt: u16, points: &[(f64, f64)]) -> ParsedSection {
        let mut records = vec![control(1001.0, 0.999, 0, 0, 0, 0)];
        records.push(control(
            0.0,
            0.0,
            0,
            0,
            1,
            i64::try_from(points.len()).unwrap(),
        ));
        records.extend(data(&[
            f64::from(u32::try_from(points.len()).unwrap()),
            2.0,
        ]));
        let mut words = Vec::with_capacity(points.len() * 2);
        for (x, y) in points {
            words.push(*x);
            words.push(*y);
        }
        records.extend(data(&words));
        ParsedSection {
            file_number: 3,
            reaction_mt,
            record_count: records.len() as u64,
            sha256: "0".repeat(64),
            records,
        }
    }

    fn table(points: &[(f64, f64)]) -> ReactionTable {
        let section = kerma_section(301, points);
        let mut cursor = 0_usize;
        take_control_pub(&section, &mut cursor).unwrap();
        let (_, table) = ReactionTable::parse(&section, &mut cursor).unwrap();
        assert_eq!(cursor, section.records.len());
        table
    }

    fn folded(points: &[(f64, f64)], role: ExtractedTableRole, response_mt: u16) -> FoldedTable {
        FoldedTable {
            table: table(points),
            role,
            response_mt,
            boundary_held_knots: 0,
        }
    }

    type PartialSpec = (DoseComponent, u16, u16, Vec<(f64, f64)>);

    fn nuclide_fold(
        name: &str,
        atoms_per_kg: f64,
        total: &[(f64, f64)],
        partials: Vec<PartialSpec>,
    ) -> NuclideFold {
        NuclideFold {
            nuclide: name.to_string(),
            endf_mat: 1,
            mass_fraction: 1.0,
            atomic_weight_ratio: 1.0,
            atoms_per_kg,
            pendf_path: format!("{name}/tape23"),
            pendf_sha256: "0".repeat(64),
            total: folded(total, ExtractedTableRole::TotalKerma, 301),
            kinematic: folded(
                &[(1.0, 0.0), (2.0, 0.0)],
                ExtractedTableRole::KinematicTotal,
                443,
            ),
            partials: partials
                .into_iter()
                .map(|(component, reaction_mt, partial_mt, points)| {
                    (
                        component,
                        reaction_mt,
                        partial_mt,
                        folded(&points, ExtractedTableRole::PartialKerma, partial_mt),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn union_grid_unions_knots_and_extends_to_transport_range() {
        let folds = vec![
            nuclide_fold(
                "H1",
                1.0,
                &[(1.0e-5, 1.0), (1.0e3, 2.0), (2.0e7, 3.0)],
                vec![],
            ),
            nuclide_fold(
                "B10",
                1.0,
                &[(1.0e-5, 4.0), (5.0e2, 5.0), (2.0e7, 6.0)],
                vec![],
            ),
        ];
        let grid = union_response_grid(&folds, [1.0e-6, 3.0e7]).unwrap();
        assert_eq!(grid, vec![1.0e-6, 1.0e-5, 5.0e2, 1.0e3, 2.0e7, 3.0e7]);
    }

    #[test]
    fn union_grid_inside_domain_keeps_table_bounds() {
        let folds = vec![nuclide_fold(
            "H1",
            1.0,
            &[(1.0e-5, 1.0), (2.0e7, 2.0)],
            vec![],
        )];
        let grid = union_response_grid(&folds, [1.0e-4, 1.0e7]).unwrap();
        assert_eq!(grid, vec![1.0e-5, 2.0e7]);
    }

    #[test]
    fn fold_applies_atom_density_conversion_and_residual() {
        let joule_per_ev = 1.602_176_634e-19;
        // H1 total KERMA ramps 10 -> 20 eV barn over 1..2 eV.
        // B10 partial adds a constant 4 eV barn.
        let mut folds = vec![
            nuclide_fold("H1", 2.0, &[(1.0, 10.0), (2.0, 20.0)], vec![]),
            nuclide_fold(
                "B10",
                3.0,
                &[(1.0, 30.0), (2.0, 60.0)],
                vec![(DoseComponent::Boron, 107, 407, vec![(1.0, 4.0), (2.0, 4.0)])],
            ),
        ];
        let grid = union_response_grid(&folds, [1.0, 2.0]).unwrap();
        let curves = fold_component_curves(&mut folds, &grid, joule_per_ev).unwrap();
        let conversion = joule_per_ev * CM2_PER_BARN;
        // At E=1: total = 2*10 + 3*30 = 110; boron = 3*4 = 12; residual = 98.
        assert_eq!(grid[0], 1.0);
        assert!((curves.total_neutron[0] - 110.0 * conversion).abs() < 1.0e-30);
        assert!((curves.boron[0] - 12.0 * conversion).abs() < 1.0e-30);
        assert!((curves.nitrogen[0]).abs() < 1.0e-30);
        assert!((curves.hydrogen[0] - 98.0 * conversion).abs() < 1.0e-30);
        // At E=2: total = 2*20 + 3*60 = 220; boron = 12; residual = 208.
        assert!((curves.total_neutron[1] - 220.0 * conversion).abs() < 1.0e-30);
        assert!((curves.hydrogen[1] - 208.0 * conversion).abs() < 1.0e-30);
        assert!(curves.minimum_residual >= 0.0);
    }

    #[test]
    fn fold_interpolates_mid_grid_points() {
        let joule_per_ev = 1.0;
        // Two nuclides with different grids; union inserts the other knots and
        // each table is linear-linear interpolated there.
        let mut folds = vec![
            nuclide_fold("H1", 1.0, &[(1.0, 0.0), (3.0, 10.0)], vec![]),
            nuclide_fold("O16", 1.0, &[(1.0, 4.0), (2.0, 8.0), (3.0, 4.0)], vec![]),
        ];
        let grid = union_response_grid(&folds, [1.0, 3.0]).unwrap();
        assert_eq!(grid, vec![1.0, 2.0, 3.0]);
        let curves = fold_component_curves(&mut folds, &grid, joule_per_ev).unwrap();
        // At E=2: H1 interpolates to 5; O16 is tabulated at 8.
        assert!((curves.total_neutron[1] - 13.0 * CM2_PER_BARN).abs() < 1.0e-30);
    }

    #[test]
    fn fold_holds_boundary_values_outside_a_table_domain() {
        let joule_per_ev = 1.0;
        // O16's table starts at 2.0 while the union (and domain) starts at 1.0.
        let mut folds = vec![
            nuclide_fold("H1", 1.0, &[(1.0, 2.0), (4.0, 8.0)], vec![]),
            nuclide_fold("O16", 1.0, &[(2.0, 100.0), (4.0, 200.0)], vec![]),
        ];
        let grid = union_response_grid(&folds, [1.0, 4.0]).unwrap();
        assert_eq!(grid, vec![1.0, 2.0, 4.0]);
        let curves = fold_component_curves(&mut folds, &grid, joule_per_ev).unwrap();
        // At E=1 the O16 contribution holds its first tabulated value.
        assert!((curves.total_neutron[0] - 102.0 * CM2_PER_BARN).abs() < 1.0e-30);
        assert_eq!(folds[1].total.boundary_held_knots, 1);
        assert_eq!(folds[0].total.boundary_held_knots, 0);
    }

    #[test]
    fn fold_rejects_negative_residual() {
        let mut folds = vec![nuclide_fold(
            "B10",
            1.0,
            &[(1.0, 1.0), (2.0, 1.0)],
            vec![(DoseComponent::Boron, 107, 407, vec![(1.0, 5.0), (2.0, 5.0)])],
        )];
        let grid = union_response_grid(&folds, [1.0, 2.0]).unwrap();
        let error = fold_component_curves(&mut folds, &grid, 1.0).unwrap_err();
        assert!(matches!(error, ResponseTableError::NegativeResidual { .. }));
    }

    #[test]
    fn discontinuity_table_steps_to_right_value() {
        let mut folds = vec![nuclide_fold(
            "O17",
            1.0,
            &[(1.0, 4.0), (3.0, 9.0), (3.0, 0.0), (4.0, 0.0)],
            vec![],
        )];
        let grid = union_response_grid(&folds, [1.0, 4.0]).unwrap();
        // The duplicate knot is deduplicated; evaluation at 3.0 yields the
        // right-hand value.
        assert_eq!(grid, vec![1.0, 3.0, 4.0]);
        let curves = fold_component_curves(&mut folds, &grid, 1.0).unwrap();
        assert_eq!(curves.total_neutron[1], 0.0);
    }

    #[test]
    fn report_rejects_empty_id() {
        let report = NjoyResponseTableGenerationReport {
            schema_version: NJOY_RESPONSE_TABLE_GENERATION_SCHEMA.to_string(),
            id: " ".to_string(),
            qualification: NjoyResponseTableQualification::TablesGeneratedUnreviewed,
            material: reference("m"),
            component_profile: reference("p"),
            generation_method: reference("g"),
            evaluated_source_selection: reference("s"),
            nuclear_data_manifest: reference("n"),
            neutron_transport_domain: reference("d"),
            domain_aware_suitability: reference("a"),
            execution_receipt: reference("e"),
            response_set: reference("r"),
            atom_density_basis: "open_mc_hdf5_atomic_weight_ratios".to_string(),
            avogadro_constant_per_mol: AVOGADRO_CONSTANT_PER_MOL,
            neutron_mass_u: NEUTRON_MASS_U,
            joule_per_ev: 1.602_176_634e-19,
            cm2_per_barn: CM2_PER_BARN,
            transport_energy_range_ev: [1.0e-5, 2.0e7],
            union_grid_knot_count: 2,
            boundary_held_knot_count: 0,
            nuclides: vec![NuclideTableExtraction {
                nuclide: "H1".to_string(),
                endf_mat: 125,
                mass_fraction: 1.0,
                atomic_weight_ratio: 0.999167,
                atoms_per_kg: 1.0,
                production_pendf_path: "H1/tape23".to_string(),
                production_pendf_sha256: "0".repeat(64),
                tables: vec![],
            }],
            partial_channels: vec![],
            residual: ResidualComponentRecord {
                component: DoseComponent::Hydrogen,
                subtracted_components: vec![DoseComponent::Boron, DoseComponent::Nitrogen],
                minimum_residual_gy_cm2: 0.0,
                maximum_residual_fraction_of_total: 0.0,
            },
            carried_findings: CarriedFindingSummary {
                in_domain_kinematic_violation_count: 0,
                rejecting_processor_finding_count: 0,
                informational_processor_finding_count: 0,
                source_format_finding_count: 0,
                nuclides_with_findings: vec![],
            },
            finding_disposition:
                ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained,
            caveats: vec![],
        };
        assert!(report.validate().is_err());
        let valid = NjoyResponseTableGenerationReport {
            id: "openbnct.test.generation.v1".to_string(),
            ..report
        };
        valid.validate().unwrap();
    }

    #[test]
    fn report_rejects_wrong_schema() {
        let report = NjoyResponseTableGenerationReport {
            schema_version: "openbnct.other/9.9.9".to_string(),
            id: "openbnct.test.generation.v1".to_string(),
            qualification: NjoyResponseTableQualification::TablesGeneratedUnreviewed,
            material: reference("m"),
            component_profile: reference("p"),
            generation_method: reference("g"),
            evaluated_source_selection: reference("s"),
            nuclear_data_manifest: reference("n"),
            neutron_transport_domain: reference("d"),
            domain_aware_suitability: reference("a"),
            execution_receipt: reference("e"),
            response_set: reference("r"),
            atom_density_basis: "open_mc_hdf5_atomic_weight_ratios".to_string(),
            avogadro_constant_per_mol: AVOGADRO_CONSTANT_PER_MOL,
            neutron_mass_u: NEUTRON_MASS_U,
            joule_per_ev: 1.602_176_634e-19,
            cm2_per_barn: CM2_PER_BARN,
            transport_energy_range_ev: [1.0e-5, 2.0e7],
            union_grid_knot_count: 2,
            boundary_held_knot_count: 0,
            nuclides: vec![NuclideTableExtraction {
                nuclide: "H1".to_string(),
                endf_mat: 125,
                mass_fraction: 1.0,
                atomic_weight_ratio: 0.999167,
                atoms_per_kg: 1.0,
                production_pendf_path: "H1/tape23".to_string(),
                production_pendf_sha256: "0".repeat(64),
                tables: vec![],
            }],
            partial_channels: vec![],
            residual: ResidualComponentRecord {
                component: DoseComponent::Hydrogen,
                subtracted_components: vec![DoseComponent::Boron, DoseComponent::Nitrogen],
                minimum_residual_gy_cm2: 0.0,
                maximum_residual_fraction_of_total: 0.0,
            },
            carried_findings: CarriedFindingSummary {
                in_domain_kinematic_violation_count: 0,
                rejecting_processor_finding_count: 0,
                informational_processor_finding_count: 0,
                source_format_finding_count: 0,
                nuclides_with_findings: vec![],
            },
            finding_disposition:
                ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained,
            caveats: vec![],
        };
        assert!(report.validate().is_err());
    }

    fn reference(id: &str) -> ContentReference {
        ContentReference {
            id: id.to_string(),
            sha256: "0".repeat(64),
        }
    }
}

#[cfg(test)]
mod frozen_tests {
    use super::*;

    const RESPONSE_SET_UNREVIEWED: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/neutron-response-set.unreviewed.json"
    );
    const GENERATION_REPORT: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-response-table-generation.json"
    );
    const REVIEW: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-response-set-review.json"
    );
    const RESPONSE_SET_REVIEWED: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/provenance/neutron-response-set.json"
    );

    #[test]
    fn frozen_response_set_chain_is_consistent() {
        let set: NeutronResponseSet = serde_json::from_slice(RESPONSE_SET_UNREVIEWED).unwrap();
        set.validate().unwrap();
        assert_eq!(
            set.qualification,
            ResponseSetQualification::GeneratedUnreviewed
        );
        assert_eq!(set.energy_ev.len(), 7526);
        assert_eq!(
            sha256_bytes(RESPONSE_SET_UNREVIEWED),
            "3517140f5caa1f4596996fd96a86f1d187e126558d834a4243770956e6b607d9"
        );

        let report: NjoyResponseTableGenerationReport =
            serde_json::from_slice(GENERATION_REPORT).unwrap();
        report.validate().unwrap();
        assert_eq!(
            sha256_bytes(GENERATION_REPORT),
            "949505db0efa35208f2e737c7917564cd3e265f89e7d281ebd579c970a25c3a6"
        );
        // The report binds the exact unreviewed set bytes and carries every
        // in-domain finding.
        assert_eq!(
            report.response_set.sha256,
            sha256_bytes(RESPONSE_SET_UNREVIEWED)
        );
        assert_eq!(
            report.carried_findings.in_domain_kinematic_violation_count,
            72
        );
        assert_eq!(
            report.finding_disposition,
            ResponseTableFindingDisposition::CarriedInProvenanceFindingsRetained
        );
        assert_eq!(report.nuclides.len(), 10);
        assert!(report.residual.minimum_residual_gy_cm2 >= 0.0);

        let review: NjoyResponseSetReviewReport = serde_json::from_slice(REVIEW).unwrap();
        review.validate().unwrap();
        assert_eq!(
            sha256_bytes(REVIEW),
            "977551dd4d9023b44aed7c5c8ff828a86786f2f50245295f53bbb7cbab07f198"
        );
        assert_eq!(
            review.response_set_unreviewed.sha256,
            sha256_bytes(RESPONSE_SET_UNREVIEWED)
        );

        let reviewed: NeutronResponseSet = serde_json::from_slice(RESPONSE_SET_REVIEWED).unwrap();
        reviewed.validate_for_folding().unwrap();
        assert_eq!(
            sha256_bytes(RESPONSE_SET_REVIEWED),
            "bfc48efe75f470cd8f1f78c35e4589cba8853655cc9cf9f709f5cece4a8a9afd"
        );
        assert_eq!(
            reviewed.qualification,
            ResponseSetQualification::IndependentlyReviewed
        );
        assert_eq!(
            reviewed.independent_review.as_ref().unwrap().sha256,
            sha256_bytes(REVIEW)
        );
        // The reviewed set carries identical tables to the unreviewed set.
        assert_eq!(reviewed.energy_ev, set.energy_ev);
        assert_eq!(reviewed.boron_gy_cm2, set.boron_gy_cm2);
        assert_eq!(reviewed.total_neutron_gy_cm2, set.total_neutron_gy_cm2);
    }
}
