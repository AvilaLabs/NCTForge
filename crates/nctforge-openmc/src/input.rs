// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::path::Path;

use nctforge_core::{ContentReference, DoseComponent, GridGeometry};
use nctforge_transport::{
    AngularDistribution, ComponentDefinitionProfile, EnergyDistribution, FixedSourceDefinition,
    MaterialDefinition, NeutronResponseSet, ParticleType, SourceSpatialDistribution, TransportCase,
};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    NuclearDataError, NuclearDataManifest, TARGET_OPENMC_SOURCE_COMMIT, TARGET_OPENMC_VERSION,
    TEMPERATURE_TOLERANCE_K,
};

pub const OPENMC_DEFAULT_STRIDE: u64 = 152_917;
pub const CANDIDATE_REFERENCE_SEEDS: [u64; 3] = [20260831, 314159265, 271828182];
const EXECUTION_PROFILE_SCHEMA: &str = "nctforge.openmc-execution-profile/0.1.0";
const INPUT_MANIFEST_SCHEMA: &str = "nctforge.openmc-input-manifest/0.1.0";
const XML_MEDIA_TYPE: &str = "application/xml";
const JSON_MEDIA_TYPE: &str = "application/json";
const IDENTITY_DIRECTION: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const DIRECTION_TOLERANCE: f64 = 1.0e-12;

const MESH_ID: u32 = 1;
const MESH_FILTER_ID: u32 = 1;
const NEUTRON_FILTER_ID: u32 = 2;
const PHOTON_FILTER_ID: u32 = 3;
const BORON_RESPONSE_FILTER_ID: u32 = 4;
const NITROGEN_RESPONSE_FILTER_ID: u32 = 5;
const HYDROGEN_RESPONSE_FILTER_ID: u32 = 6;
const NEUTRON_ENERGY_FILTER_ID: u32 = 7;
const PHOTON_ENERGY_FILTER_ID: u32 = 8;
const SURFACE_FILTER_ID: u32 = 9;

const BORON_TALLY_ID: u32 = 1;
const NITROGEN_TALLY_ID: u32 = 2;
const HYDROGEN_TALLY_ID: u32 = 3;
const NEUTRON_HEATING_TALLY_ID: u32 = 4;
const PHOTON_HEATING_TALLY_ID: u32 = 5;
const COUPLED_HEATING_TALLY_ID: u32 = 6;
const BORON_REACTION_TALLY_ID: u32 = 7;
const NITROGEN_REACTION_TALLY_ID: u32 = 8;
const NEUTRON_FLUX_TALLY_ID: u32 = 9;
const PHOTON_FLUX_TALLY_ID: u32 = 10;
const NEUTRON_LEAKAGE_TALLY_ID: u32 = 11;
const PHOTON_LEAKAGE_TALLY_ID: u32 = 12;

/// Versioned controls that materially affect one OpenMC input deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcExecutionProfile {
    pub schema_version: String,
    pub id: String,
    pub purpose: OpenMcExecutionPurpose,
    pub openmc_version: String,
    pub openmc_source_commit: String,
    pub run_mode: OpenMcRunMode,
    pub batches: u32,
    pub seed: u64,
    pub stride: u64,
    pub photon_transport: bool,
    pub atomic_relaxation: bool,
    pub electron_treatment: OpenMcElectronTreatment,
    pub energy_mode: OpenMcEnergyMode,
    pub probability_tables: bool,
    pub survival_biasing: bool,
    pub temperature_method: OpenMcTemperatureMethod,
    pub temperature_tolerance_k: f64,
    pub temperature_multipole: bool,
    pub confidence_intervals: bool,
    pub write_summary: bool,
    pub write_ascii_tallies: bool,
    pub write_sourcepoint: bool,
    pub event_based: bool,
    pub neutron_diagnostic_energy_grid_ev: Vec<f64>,
    pub photon_diagnostic_energy_grid_ev: Vec<f64>,
}

impl OpenMcExecutionProfile {
    pub fn validate(&self) -> Result<(), OpenMcProfileError> {
        if self.schema_version != EXECUTION_PROFILE_SCHEMA {
            return Err(OpenMcProfileError::UnsupportedSchema(
                self.schema_version.clone(),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(OpenMcProfileError::EmptyId);
        }
        if self.openmc_version != TARGET_OPENMC_VERSION {
            return Err(OpenMcProfileError::UnsupportedOpenMcVersion(
                self.openmc_version.clone(),
            ));
        }
        if self.openmc_source_commit != TARGET_OPENMC_SOURCE_COMMIT {
            return Err(OpenMcProfileError::UnsupportedOpenMcCommit(
                self.openmc_source_commit.clone(),
            ));
        }
        if self.batches < 2 {
            return Err(OpenMcProfileError::InsufficientBatches(self.batches));
        }
        if self.purpose == OpenMcExecutionPurpose::CandidateReference {
            if self.batches < 50 {
                return Err(OpenMcProfileError::InsufficientCandidateBatches(
                    self.batches,
                ));
            }
            if !CANDIDATE_REFERENCE_SEEDS.contains(&self.seed) {
                return Err(OpenMcProfileError::UnregisteredCandidateSeed(self.seed));
            }
        }
        if self.seed == 0 {
            return Err(OpenMcProfileError::ZeroSeed);
        }
        if self.stride != OPENMC_DEFAULT_STRIDE {
            return Err(OpenMcProfileError::UnsupportedSetting("stride"));
        }
        for (accepted, label) in [
            (self.photon_transport, "photon_transport"),
            (self.atomic_relaxation, "atomic_relaxation"),
            (self.probability_tables, "probability_tables"),
            (!self.survival_biasing, "survival_biasing"),
            (!self.temperature_multipole, "temperature_multipole"),
            (!self.confidence_intervals, "confidence_intervals"),
            (self.write_summary, "write_summary"),
            (!self.write_ascii_tallies, "write_ascii_tallies"),
            (!self.write_sourcepoint, "write_sourcepoint"),
            (!self.event_based, "event_based"),
        ] {
            if !accepted {
                return Err(OpenMcProfileError::UnsupportedSetting(label));
            }
        }
        if self.temperature_tolerance_k != TEMPERATURE_TOLERANCE_K {
            return Err(OpenMcProfileError::UnsupportedSetting(
                "temperature_tolerance_k",
            ));
        }
        validate_energy_grid(
            "neutron",
            &self.neutron_diagnostic_energy_grid_ev,
            &[0.5, 1_000.0, 10_000.0],
        )?;
        validate_energy_grid(
            "photon",
            &self.photon_diagnostic_energy_grid_ev,
            &[477_000.0, 479_000.0, 2_223_000.0, 2_225_000.0],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcExecutionPurpose {
    SmokeOnly,
    CandidateReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcRunMode {
    FixedSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcElectronTreatment {
    Led,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenMcEnergyMode {
    ContinuousEnergy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcTemperatureMethod {
    Nearest,
}

/// Exact JSON bytes used to create a deck. Hashes are calculated before parse.
#[derive(Debug, Clone, Copy)]
pub struct OpenMcInputArtifacts<'a> {
    pub component_profile_json: &'a [u8],
    pub material_json: &'a [u8],
    pub source_json: &'a [u8],
    pub response_set_json: &'a [u8],
    pub nuclear_data_manifest_json: &'a [u8],
    pub execution_profile_json: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcInputManifest {
    pub schema_version: String,
    pub case_id: String,
    pub backend_id: String,
    pub openmc_version: String,
    pub openmc_source_commit: String,
    pub bindings: OpenMcInputBindings,
    pub execution: OpenMcRunControls,
    pub scoring_mesh: OpenMcScoringMesh,
    pub tallies: Vec<OpenMcTallyContract>,
    pub xml_artifacts: Vec<OpenMcInputManifestArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcInputBindings {
    pub component_profile: ContentReference,
    pub material: ContentReference,
    pub source: ContentReference,
    pub response_set: ContentReference,
    pub nuclear_data_manifest: ContentReference,
    pub response_generation_method: ContentReference,
    pub independent_response_review: ContentReference,
    pub execution_profile: ContentReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcRunControls {
    pub purpose: OpenMcExecutionPurpose,
    pub requested_histories: u64,
    pub batches: u32,
    pub particles_per_batch: u64,
    pub seed: u64,
    pub stride: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcScoringMesh {
    pub mesh_id: u32,
    pub dimensions: [u32; 3],
    pub lower_left_cm: [f64; 3],
    pub upper_right_cm: [f64; 3],
    pub voxel_volume_cm3: f64,
    pub voxel_mass_g: f64,
    pub cell_volume_cm3: f64,
    pub cell_mass_g: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcInputManifestArtifact {
    pub path: String,
    pub sha256: String,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenMcTallyContract {
    pub id: u32,
    pub name: String,
    pub component: Option<DoseComponent>,
    pub particle: Option<ParticleType>,
    pub quantity: OpenMcTallyQuantity,
    pub raw_unit: OpenMcRawTallyUnit,
    pub collection_normalization: OpenMcCollectionNormalization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcTallyQuantity {
    ResponseWeightedTrackLength,
    Heating,
    ReactionRate,
    EnergyBinnedTrackLength,
    SurfaceCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcRawTallyUnit {
    GrayCubicCentimeterPerSourceNeutron,
    ElectronVoltPerSourceNeutron,
    ReactionsPerSourceNeutron,
    CentimeterPerSourceNeutron,
    ParticlesPerSourceNeutron,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMcCollectionNormalization {
    DivideByVoxelVolumeCm3,
    ElectronVoltToJouleDivideByVoxelMassKg,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedOpenMcFile {
    pub relative_path: String,
    pub media_type: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenMcInputDeck {
    pub manifest: OpenMcInputManifest,
    pub files: Vec<GeneratedOpenMcFile>,
}

impl OpenMcInputDeck {
    /// Generate byte-stable OpenMC 0.16 inputs after validating the supplied
    /// JSON bindings and verifying every selected nuclear-data file. This does
    /// not execute OpenMC or qualify results.
    pub fn generate(
        case: &TransportCase,
        nuclear_data_root: &Path,
        artifacts: OpenMcInputArtifacts<'_>,
    ) -> Result<Self, OpenMcInputError> {
        case.validate()
            .map_err(|error| OpenMcInputError::InvalidTransportCase(error.to_string()))?;

        let component_profile: ComponentDefinitionProfile =
            parse_json("component_profile", artifacts.component_profile_json)?;
        component_profile
            .validate()
            .map_err(|error| OpenMcInputError::InvalidComponentProfile(error.to_string()))?;
        let material: MaterialDefinition = parse_json("material", artifacts.material_json)?;
        material
            .validate()
            .map_err(|error| OpenMcInputError::InvalidMaterial(error.to_string()))?;
        let source: FixedSourceDefinition = parse_json("source", artifacts.source_json)?;
        source
            .validate()
            .map_err(|error| OpenMcInputError::InvalidSource(error.to_string()))?;
        let response_set: NeutronResponseSet =
            parse_json("response_set", artifacts.response_set_json)?;
        response_set
            .validate_for_folding()
            .map_err(|error| OpenMcInputError::InvalidResponseSet(error.to_string()))?;
        let nuclear_data: NuclearDataManifest = parse_json(
            "nuclear_data_manifest",
            artifacts.nuclear_data_manifest_json,
        )?;
        let execution_profile: OpenMcExecutionProfile =
            parse_json("execution_profile", artifacts.execution_profile_json)?;
        execution_profile.validate()?;

        if case.material != material {
            return Err(OpenMcInputError::CaseMaterialMismatch);
        }
        if case.source != source {
            return Err(OpenMcInputError::CaseSourceMismatch);
        }

        let component_reference =
            content_reference(&component_profile.id, artifacts.component_profile_json);
        let material_reference = content_reference(&material.id, artifacts.material_json);
        let source_reference = content_reference(&source.id, artifacts.source_json);
        let response_reference = content_reference(&response_set.id, artifacts.response_set_json);
        let nuclear_data_reference =
            content_reference(&nuclear_data.id, artifacts.nuclear_data_manifest_json);
        let execution_reference =
            content_reference(&execution_profile.id, artifacts.execution_profile_json);

        require_binding(
            "response_set.component_profile",
            &response_set.component_profile,
            &component_reference,
        )?;
        require_binding(
            "response_set.material",
            &response_set.material,
            &material_reference,
        )?;
        require_binding(
            "response_set.nuclear_data_manifest",
            &response_set.nuclear_data_manifest,
            &nuclear_data_reference,
        )?;

        nuclear_data.validate_for_case(case)?;
        nuclear_data.verify_files(nuclear_data_root)?;
        let data_energy_range = nuclear_data.neutron_transport_energy_range_for_case(case)?;
        let response_energy_range = response_set.transport_energy_range_ev;
        if response_energy_range[0] > data_energy_range[0]
            || response_energy_range[1] < data_energy_range[1]
        {
            return Err(OpenMcInputError::ResponseEnergyRangeDoesNotCoverData {
                response_ev: response_energy_range,
                data_ev: data_energy_range,
            });
        }

        let source_energy_ev = match source.energy {
            EnergyDistribution::Monoenergetic { energy_ev } => energy_ev,
        };
        if source.particle != ParticleType::Neutron {
            return Err(OpenMcInputError::UnsupportedSourceParticle(source.particle));
        }
        if source_energy_ev < data_energy_range[0] || source_energy_ev >= data_energy_range[1] {
            return Err(OpenMcInputError::SourceEnergyOutsideDataRange {
                source_ev: source_energy_ev,
                data_ev: data_energy_range,
            });
        }

        let scoring_mesh = scoring_mesh(case)?;
        validate_source_containment(&source, &scoring_mesh)?;
        let batch_count = u64::from(execution_profile.batches);
        if !case.requested_histories.is_multiple_of(batch_count) {
            return Err(OpenMcInputError::HistoriesNotDivisibleByBatches {
                histories: case.requested_histories,
                batches: execution_profile.batches,
            });
        }
        let particles_per_batch = case.requested_histories / batch_count;
        if particles_per_batch == 0 || particles_per_batch > i64::MAX as u64 {
            return Err(OpenMcInputError::InvalidParticlesPerBatch(
                particles_per_batch,
            ));
        }

        let geometry_xml = geometry_xml(case, &scoring_mesh)?;
        let materials_xml = materials_xml(&material)?;
        let settings_xml = settings_xml(
            &source,
            &execution_profile,
            particles_per_batch,
            execution_profile.batches,
        )?;
        let tallies_xml = tallies_xml(&response_set, &execution_profile, &scoring_mesh)?;

        let mut files = vec![
            generated_file("geometry.xml", XML_MEDIA_TYPE, geometry_xml),
            generated_file("materials.xml", XML_MEDIA_TYPE, materials_xml),
            generated_file("settings.xml", XML_MEDIA_TYPE, settings_xml),
            generated_file("tallies.xml", XML_MEDIA_TYPE, tallies_xml),
        ];
        let xml_artifacts = files
            .iter()
            .map(|file| OpenMcInputManifestArtifact {
                path: file.relative_path.clone(),
                sha256: file.sha256.clone(),
                media_type: file.media_type.clone(),
            })
            .collect();
        let independent_response_review = response_set
            .independent_review
            .clone()
            .expect("folding validation requires independent review");
        let manifest = OpenMcInputManifest {
            schema_version: INPUT_MANIFEST_SCHEMA.into(),
            case_id: case.case_id.clone(),
            backend_id: "openmc".into(),
            openmc_version: TARGET_OPENMC_VERSION.into(),
            openmc_source_commit: TARGET_OPENMC_SOURCE_COMMIT.into(),
            bindings: OpenMcInputBindings {
                component_profile: component_reference,
                material: material_reference,
                source: source_reference,
                response_set: response_reference,
                nuclear_data_manifest: nuclear_data_reference,
                response_generation_method: response_set.generation_method.clone(),
                independent_response_review,
                execution_profile: execution_reference,
            },
            execution: OpenMcRunControls {
                purpose: execution_profile.purpose,
                requested_histories: case.requested_histories,
                batches: execution_profile.batches,
                particles_per_batch,
                seed: execution_profile.seed,
                stride: execution_profile.stride,
            },
            scoring_mesh,
            tallies: tally_contracts(),
            xml_artifacts,
        };
        let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(OpenMcInputError::ManifestSerialization)?;
        manifest_bytes.push(b'\n');
        files.push(generated_file(
            "nctforge-input-manifest.json",
            JSON_MEDIA_TYPE,
            manifest_bytes,
        ));

        Ok(Self { manifest, files })
    }

    #[must_use]
    pub fn file(&self, relative_path: &str) -> Option<&GeneratedOpenMcFile> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }
}

fn validate_energy_grid(
    particle: &'static str,
    grid: &[f64],
    required_boundaries: &[f64],
) -> Result<(), OpenMcProfileError> {
    if grid.len() < 2
        || grid.iter().any(|value| !value.is_finite() || *value < 0.0)
        || grid.windows(2).any(|pair| pair[0] >= pair[1])
        || grid[0] != 0.0
        || grid[grid.len() - 1] < 20.0e6
    {
        return Err(OpenMcProfileError::InvalidDiagnosticEnergyGrid(particle));
    }
    for boundary in required_boundaries {
        if !grid.contains(boundary) {
            return Err(OpenMcProfileError::MissingDiagnosticBoundary {
                particle,
                boundary_ev: *boundary,
            });
        }
    }
    Ok(())
}

fn parse_json<T: DeserializeOwned>(
    artifact: &'static str,
    bytes: &[u8],
) -> Result<T, OpenMcInputError> {
    serde_json::from_slice(bytes)
        .map_err(|source| OpenMcInputError::InvalidJson { artifact, source })
}

fn content_reference(id: &str, bytes: &[u8]) -> ContentReference {
    ContentReference {
        id: id.into(),
        sha256: sha256_hex(bytes),
    }
}

fn require_binding(
    label: &'static str,
    declared: &ContentReference,
    observed: &ContentReference,
) -> Result<(), OpenMcInputError> {
    if declared != observed {
        return Err(OpenMcInputError::ContentBindingMismatch {
            label,
            declared: declared.clone(),
            observed: observed.clone(),
        });
    }
    Ok(())
}

fn scoring_mesh(case: &TransportCase) -> Result<OpenMcScoringMesh, OpenMcInputError> {
    require_identity_direction(&case.geometry)?;
    let mut lower_left_cm = [0.0; 3];
    let mut upper_right_cm = [0.0; 3];
    for axis in 0..3 {
        lower_left_cm[axis] =
            (case.geometry.origin_mm[axis] - 0.5 * case.geometry.spacing_mm[axis]) / 10.0;
        upper_right_cm[axis] = lower_left_cm[axis]
            + f64::from(case.geometry.shape[axis]) * case.geometry.spacing_mm[axis] / 10.0;
    }
    let voxel_volume_cm3 = case
        .geometry
        .spacing_mm
        .iter()
        .map(|spacing| spacing / 10.0)
        .product::<f64>();
    let voxel_mass_g = voxel_volume_cm3 * case.material.density_g_cm3;
    let voxel_count = case
        .geometry
        .shape
        .iter()
        .map(|extent| f64::from(*extent))
        .product::<f64>();
    Ok(OpenMcScoringMesh {
        mesh_id: MESH_ID,
        dimensions: case.geometry.shape,
        lower_left_cm,
        upper_right_cm,
        voxel_volume_cm3,
        voxel_mass_g,
        cell_volume_cm3: voxel_volume_cm3 * voxel_count,
        cell_mass_g: voxel_mass_g * voxel_count,
    })
}

fn require_identity_direction(geometry: &GridGeometry) -> Result<(), OpenMcInputError> {
    if geometry
        .direction
        .iter()
        .zip(IDENTITY_DIRECTION)
        .any(|(observed, expected)| (*observed - expected).abs() > DIRECTION_TOLERANCE)
    {
        return Err(OpenMcInputError::UnsupportedGeometryDirection(
            geometry.direction,
        ));
    }
    Ok(())
}

fn validate_source_containment(
    source: &FixedSourceDefinition,
    mesh: &OpenMcScoringMesh,
) -> Result<(), OpenMcInputError> {
    let SourceSpatialDistribution::UniformCartesianPlane {
        x_range_cm,
        y_range_cm,
        z_cm,
        ..
    } = source.space;
    let contained = x_range_cm[0] > mesh.lower_left_cm[0]
        && x_range_cm[1] < mesh.upper_right_cm[0]
        && y_range_cm[0] > mesh.lower_left_cm[1]
        && y_range_cm[1] < mesh.upper_right_cm[1]
        && z_cm > mesh.lower_left_cm[2]
        && z_cm < mesh.upper_right_cm[2];
    if !contained {
        return Err(OpenMcInputError::SourceOutsideGeometry);
    }
    Ok(())
}

fn geometry_xml(
    case: &TransportCase,
    mesh: &OpenMcScoringMesh,
) -> Result<Vec<u8>, OpenMcInputError> {
    xml_document("geometry", |writer| {
        let mut cell = BytesStart::new("cell");
        cell.push_attribute(("id", "1"));
        cell.push_attribute(("name", case.case_id.as_str()));
        cell.push_attribute(("material", "1"));
        cell.push_attribute(("region", "1 -2 3 -4 5 -6"));
        cell.push_attribute(("universe", "1"));
        writer.write_event(Event::Empty(cell))?;

        for (id, kind, coefficient) in [
            (1_u32, "x-plane", mesh.lower_left_cm[0]),
            (2, "x-plane", mesh.upper_right_cm[0]),
            (3, "y-plane", mesh.lower_left_cm[1]),
            (4, "y-plane", mesh.upper_right_cm[1]),
            (5, "z-plane", mesh.lower_left_cm[2]),
            (6, "z-plane", mesh.upper_right_cm[2]),
        ] {
            let id = id.to_string();
            let coefficient = format_float(coefficient);
            let mut surface = BytesStart::new("surface");
            surface.push_attribute(("id", id.as_str()));
            surface.push_attribute(("type", kind));
            surface.push_attribute(("boundary", "vacuum"));
            surface.push_attribute(("coeffs", coefficient.as_str()));
            writer.write_event(Event::Empty(surface))?;
        }
        Ok(())
    })
}

fn materials_xml(material: &MaterialDefinition) -> Result<Vec<u8>, OpenMcInputError> {
    xml_document("materials", |writer| {
        let temperature = format_float(material.temperature_k);
        let mut material_element = BytesStart::new("material");
        material_element.push_attribute(("id", "1"));
        material_element.push_attribute(("name", material.id.as_str()));
        material_element.push_attribute(("temperature", temperature.as_str()));
        writer.write_event(Event::Start(material_element))?;

        let density = format_float(material.density_g_cm3);
        let mut density_element = BytesStart::new("density");
        density_element.push_attribute(("value", density.as_str()));
        density_element.push_attribute(("units", "g/cm3"));
        writer.write_event(Event::Empty(density_element))?;

        for nuclide in &material.nuclides {
            let fraction = format_float(nuclide.mass_fraction);
            let mut element = BytesStart::new("nuclide");
            element.push_attribute(("name", nuclide.name.as_str()));
            element.push_attribute(("wo", fraction.as_str()));
            writer.write_event(Event::Empty(element))?;
        }
        writer.write_event(Event::End(BytesEnd::new("material")))?;
        Ok(())
    })
}

fn settings_xml(
    source: &FixedSourceDefinition,
    profile: &OpenMcExecutionProfile,
    particles_per_batch: u64,
    batches: u32,
) -> Result<Vec<u8>, OpenMcInputError> {
    xml_document("settings", |writer| {
        text_element(writer, "run_mode", "fixed source")?;
        text_element(writer, "particles", &particles_per_batch.to_string())?;
        text_element(writer, "batches", &batches.to_string())?;

        let mut source_element = BytesStart::new("source");
        source_element.push_attribute(("type", "independent"));
        source_element.push_attribute(("strength", "1.0"));
        source_element.push_attribute(("particle", "neutron"));
        writer.write_event(Event::Start(source_element))?;

        let SourceSpatialDistribution::UniformCartesianPlane {
            x_range_cm,
            y_range_cm,
            z_cm,
            ..
        } = source.space;
        let mut space = BytesStart::new("space");
        space.push_attribute(("type", "box"));
        writer.write_event(Event::Start(space))?;
        text_element(
            writer,
            "parameters",
            &format_numbers(&[
                x_range_cm[0],
                y_range_cm[0],
                z_cm,
                x_range_cm[1],
                y_range_cm[1],
                z_cm,
            ]),
        )?;
        writer.write_event(Event::End(BytesEnd::new("space")))?;

        let AngularDistribution::Monodirectional { unit_vector } = source.angle;
        let direction = format_numbers(&unit_vector);
        let mut angle = BytesStart::new("angle");
        angle.push_attribute(("type", "monodirectional"));
        angle.push_attribute(("reference_uvw", direction.as_str()));
        writer.write_event(Event::Empty(angle))?;

        let EnergyDistribution::Monoenergetic { energy_ev } = source.energy;
        let mut energy = BytesStart::new("energy");
        energy.push_attribute(("type", "discrete"));
        writer.write_event(Event::Start(energy))?;
        text_element(
            writer,
            "parameters",
            &format!("{} 1.0", format_float(energy_ev)),
        )?;
        writer.write_event(Event::End(BytesEnd::new("energy")))?;
        writer.write_event(Event::End(BytesEnd::new("source")))?;

        writer.write_event(Event::Start(BytesStart::new("output")))?;
        text_element(writer, "summary", bool_text(profile.write_summary))?;
        text_element(writer, "tallies", bool_text(profile.write_ascii_tallies))?;
        writer.write_event(Event::End(BytesEnd::new("output")))?;

        writer.write_event(Event::Start(BytesStart::new("state_point")))?;
        text_element(writer, "batches", &batches.to_string())?;
        writer.write_event(Event::End(BytesEnd::new("state_point")))?;
        writer.write_event(Event::Start(BytesStart::new("source_point")))?;
        text_element(writer, "write", bool_text(profile.write_sourcepoint))?;
        writer.write_event(Event::End(BytesEnd::new("source_point")))?;

        text_element(
            writer,
            "confidence_intervals",
            bool_text(profile.confidence_intervals),
        )?;
        text_element(writer, "electron_treatment", "led")?;
        text_element(
            writer,
            "atomic_relaxation",
            bool_text(profile.atomic_relaxation),
        )?;
        text_element(writer, "energy_mode", "continuous-energy")?;
        text_element(
            writer,
            "photon_transport",
            bool_text(profile.photon_transport),
        )?;
        text_element(writer, "ptables", bool_text(profile.probability_tables))?;
        text_element(writer, "seed", &profile.seed.to_string())?;
        text_element(writer, "stride", &profile.stride.to_string())?;
        text_element(
            writer,
            "survival_biasing",
            bool_text(profile.survival_biasing),
        )?;
        text_element(writer, "temperature_method", "nearest")?;
        text_element(
            writer,
            "temperature_multipole",
            bool_text(profile.temperature_multipole),
        )?;
        text_element(
            writer,
            "temperature_tolerance",
            &format_float(profile.temperature_tolerance_k),
        )?;
        text_element(writer, "event_based", bool_text(profile.event_based))?;
        Ok(())
    })
}

fn tallies_xml(
    response: &NeutronResponseSet,
    profile: &OpenMcExecutionProfile,
    mesh: &OpenMcScoringMesh,
) -> Result<Vec<u8>, OpenMcInputError> {
    xml_document("tallies", |writer| {
        let mut mesh_element = BytesStart::new("mesh");
        mesh_element.push_attribute(("id", "1"));
        writer.write_event(Event::Start(mesh_element))?;
        text_element(writer, "dimension", &format_integers(&mesh.dimensions))?;
        text_element(writer, "lower_left", &format_numbers(&mesh.lower_left_cm))?;
        text_element(writer, "upper_right", &format_numbers(&mesh.upper_right_cm))?;
        writer.write_event(Event::End(BytesEnd::new("mesh")))?;

        filter_with_bins(writer, MESH_FILTER_ID, "mesh", "1")?;
        filter_with_bins(writer, NEUTRON_FILTER_ID, "particle", "neutron")?;
        filter_with_bins(writer, PHOTON_FILTER_ID, "particle", "photon")?;
        energy_function_filter(
            writer,
            BORON_RESPONSE_FILTER_ID,
            &response.energy_ev,
            &response.boron_gy_cm2,
        )?;
        energy_function_filter(
            writer,
            NITROGEN_RESPONSE_FILTER_ID,
            &response.energy_ev,
            &response.nitrogen_gy_cm2,
        )?;
        energy_function_filter(
            writer,
            HYDROGEN_RESPONSE_FILTER_ID,
            &response.energy_ev,
            &response.hydrogen_gy_cm2,
        )?;
        filter_with_bins(
            writer,
            NEUTRON_ENERGY_FILTER_ID,
            "energy",
            &format_numbers(&profile.neutron_diagnostic_energy_grid_ev),
        )?;
        filter_with_bins(
            writer,
            PHOTON_ENERGY_FILTER_ID,
            "energy",
            &format_numbers(&profile.photon_diagnostic_energy_grid_ev),
        )?;
        filter_with_bins(writer, SURFACE_FILTER_ID, "surface", "1 2 3 4 5 6")?;

        tally(
            writer,
            BORON_TALLY_ID,
            "nctforge.component.boron.response",
            &[MESH_FILTER_ID, NEUTRON_FILTER_ID, BORON_RESPONSE_FILTER_ID],
            &[],
            &["flux"],
            "tracklength",
        )?;
        tally(
            writer,
            NITROGEN_TALLY_ID,
            "nctforge.component.nitrogen.response",
            &[
                MESH_FILTER_ID,
                NEUTRON_FILTER_ID,
                NITROGEN_RESPONSE_FILTER_ID,
            ],
            &[],
            &["flux"],
            "tracklength",
        )?;
        tally(
            writer,
            HYDROGEN_TALLY_ID,
            "nctforge.component.hydrogen.response",
            &[
                MESH_FILTER_ID,
                NEUTRON_FILTER_ID,
                HYDROGEN_RESPONSE_FILTER_ID,
            ],
            &[],
            &["flux"],
            "tracklength",
        )?;
        tally(
            writer,
            NEUTRON_HEATING_TALLY_ID,
            "nctforge.audit.neutron_heating",
            &[MESH_FILTER_ID, NEUTRON_FILTER_ID],
            &[],
            &["heating"],
            "tracklength",
        )?;
        tally(
            writer,
            PHOTON_HEATING_TALLY_ID,
            "nctforge.component.photon.heating",
            &[MESH_FILTER_ID, PHOTON_FILTER_ID],
            &[],
            &["heating"],
            "collision",
        )?;
        tally(
            writer,
            COUPLED_HEATING_TALLY_ID,
            "nctforge.physical_total.coupled_heating",
            &[MESH_FILTER_ID],
            &[],
            &["heating"],
            "collision",
        )?;
        tally(
            writer,
            BORON_REACTION_TALLY_ID,
            "nctforge.audit.b10_mt107",
            &[MESH_FILTER_ID, NEUTRON_FILTER_ID],
            &["B10"],
            &["(n,a)"],
            "tracklength",
        )?;
        tally(
            writer,
            NITROGEN_REACTION_TALLY_ID,
            "nctforge.audit.n14_mt103",
            &[MESH_FILTER_ID, NEUTRON_FILTER_ID],
            &["N14"],
            &["(n,p)"],
            "tracklength",
        )?;
        tally(
            writer,
            NEUTRON_FLUX_TALLY_ID,
            "nctforge.diagnostic.neutron_fluence",
            &[MESH_FILTER_ID, NEUTRON_FILTER_ID, NEUTRON_ENERGY_FILTER_ID],
            &[],
            &["flux"],
            "tracklength",
        )?;
        tally(
            writer,
            PHOTON_FLUX_TALLY_ID,
            "nctforge.diagnostic.photon_fluence",
            &[MESH_FILTER_ID, PHOTON_FILTER_ID, PHOTON_ENERGY_FILTER_ID],
            &[],
            &["flux"],
            "tracklength",
        )?;
        tally(
            writer,
            NEUTRON_LEAKAGE_TALLY_ID,
            "nctforge.diagnostic.neutron_surface_current",
            &[SURFACE_FILTER_ID, NEUTRON_FILTER_ID],
            &[],
            &["current"],
            "analog",
        )?;
        tally(
            writer,
            PHOTON_LEAKAGE_TALLY_ID,
            "nctforge.diagnostic.photon_surface_current",
            &[SURFACE_FILTER_ID, PHOTON_FILTER_ID],
            &[],
            &["current"],
            "analog",
        )?;
        Ok(())
    })
}

fn energy_function_filter(
    writer: &mut Writer<Vec<u8>>,
    id: u32,
    energy_ev: &[f64],
    response: &[f64],
) -> io::Result<()> {
    let id = id.to_string();
    let mut filter = BytesStart::new("filter");
    filter.push_attribute(("id", id.as_str()));
    filter.push_attribute(("type", "energyfunction"));
    writer.write_event(Event::Start(filter))?;
    text_element(writer, "energy", &format_numbers(energy_ev))?;
    text_element(writer, "y", &format_numbers(response))?;
    text_element(writer, "interpolation", "linear-linear")?;
    writer.write_event(Event::End(BytesEnd::new("filter")))
}

fn filter_with_bins(
    writer: &mut Writer<Vec<u8>>,
    id: u32,
    filter_type: &str,
    bins: &str,
) -> io::Result<()> {
    let id = id.to_string();
    let mut filter = BytesStart::new("filter");
    filter.push_attribute(("id", id.as_str()));
    filter.push_attribute(("type", filter_type));
    writer.write_event(Event::Start(filter))?;
    text_element(writer, "bins", bins)?;
    writer.write_event(Event::End(BytesEnd::new("filter")))
}

fn tally(
    writer: &mut Writer<Vec<u8>>,
    id: u32,
    name: &str,
    filters: &[u32],
    nuclides: &[&str],
    scores: &[&str],
    estimator: &str,
) -> io::Result<()> {
    let id = id.to_string();
    let mut tally = BytesStart::new("tally");
    tally.push_attribute(("id", id.as_str()));
    tally.push_attribute(("name", name));
    writer.write_event(Event::Start(tally))?;
    if !filters.is_empty() {
        text_element(writer, "filters", &format_integers(filters))?;
    }
    if !nuclides.is_empty() {
        text_element(writer, "nuclides", &nuclides.join(" "))?;
    }
    text_element(writer, "scores", &scores.join(" "))?;
    text_element(writer, "estimator", estimator)?;
    writer.write_event(Event::End(BytesEnd::new("tally")))
}

fn xml_document<F>(root: &str, body: F) -> Result<Vec<u8>, OpenMcInputError>
where
    F: FnOnce(&mut Writer<Vec<u8>>) -> io::Result<()>,
{
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(OpenMcInputError::XmlWrite)?;
    writer
        .write_event(Event::Start(BytesStart::new(root)))
        .map_err(OpenMcInputError::XmlWrite)?;
    body(&mut writer).map_err(OpenMcInputError::XmlWrite)?;
    writer
        .write_event(Event::End(BytesEnd::new(root)))
        .map_err(OpenMcInputError::XmlWrite)?;
    let mut bytes = writer.into_inner();
    bytes.push(b'\n');
    Ok(bytes)
}

fn text_element(writer: &mut Writer<Vec<u8>>, name: &str, text: &str) -> io::Result<()> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))
}

fn tally_contracts() -> Vec<OpenMcTallyContract> {
    vec![
        response_tally_contract(
            BORON_TALLY_ID,
            "nctforge.component.boron.response",
            DoseComponent::Boron,
        ),
        response_tally_contract(
            NITROGEN_TALLY_ID,
            "nctforge.component.nitrogen.response",
            DoseComponent::Nitrogen,
        ),
        response_tally_contract(
            HYDROGEN_TALLY_ID,
            "nctforge.component.hydrogen.response",
            DoseComponent::Hydrogen,
        ),
        heating_tally_contract(
            NEUTRON_HEATING_TALLY_ID,
            "nctforge.audit.neutron_heating",
            None,
            Some(ParticleType::Neutron),
        ),
        heating_tally_contract(
            PHOTON_HEATING_TALLY_ID,
            "nctforge.component.photon.heating",
            Some(DoseComponent::Photon),
            Some(ParticleType::Photon),
        ),
        heating_tally_contract(
            COUPLED_HEATING_TALLY_ID,
            "nctforge.physical_total.coupled_heating",
            None,
            None,
        ),
        reaction_tally_contract(BORON_REACTION_TALLY_ID, "nctforge.audit.b10_mt107"),
        reaction_tally_contract(NITROGEN_REACTION_TALLY_ID, "nctforge.audit.n14_mt103"),
        flux_tally_contract(
            NEUTRON_FLUX_TALLY_ID,
            "nctforge.diagnostic.neutron_fluence",
            ParticleType::Neutron,
        ),
        flux_tally_contract(
            PHOTON_FLUX_TALLY_ID,
            "nctforge.diagnostic.photon_fluence",
            ParticleType::Photon,
        ),
        leakage_tally_contract(
            NEUTRON_LEAKAGE_TALLY_ID,
            "nctforge.diagnostic.neutron_surface_current",
            ParticleType::Neutron,
        ),
        leakage_tally_contract(
            PHOTON_LEAKAGE_TALLY_ID,
            "nctforge.diagnostic.photon_surface_current",
            ParticleType::Photon,
        ),
    ]
}

fn response_tally_contract(id: u32, name: &str, component: DoseComponent) -> OpenMcTallyContract {
    OpenMcTallyContract {
        id,
        name: name.into(),
        component: Some(component),
        particle: Some(ParticleType::Neutron),
        quantity: OpenMcTallyQuantity::ResponseWeightedTrackLength,
        raw_unit: OpenMcRawTallyUnit::GrayCubicCentimeterPerSourceNeutron,
        collection_normalization: OpenMcCollectionNormalization::DivideByVoxelVolumeCm3,
    }
}

fn heating_tally_contract(
    id: u32,
    name: &str,
    component: Option<DoseComponent>,
    particle: Option<ParticleType>,
) -> OpenMcTallyContract {
    OpenMcTallyContract {
        id,
        name: name.into(),
        component,
        particle,
        quantity: OpenMcTallyQuantity::Heating,
        raw_unit: OpenMcRawTallyUnit::ElectronVoltPerSourceNeutron,
        collection_normalization:
            OpenMcCollectionNormalization::ElectronVoltToJouleDivideByVoxelMassKg,
    }
}

fn reaction_tally_contract(id: u32, name: &str) -> OpenMcTallyContract {
    OpenMcTallyContract {
        id,
        name: name.into(),
        component: None,
        particle: Some(ParticleType::Neutron),
        quantity: OpenMcTallyQuantity::ReactionRate,
        raw_unit: OpenMcRawTallyUnit::ReactionsPerSourceNeutron,
        collection_normalization: OpenMcCollectionNormalization::None,
    }
}

fn flux_tally_contract(id: u32, name: &str, particle: ParticleType) -> OpenMcTallyContract {
    OpenMcTallyContract {
        id,
        name: name.into(),
        component: None,
        particle: Some(particle),
        quantity: OpenMcTallyQuantity::EnergyBinnedTrackLength,
        raw_unit: OpenMcRawTallyUnit::CentimeterPerSourceNeutron,
        collection_normalization: OpenMcCollectionNormalization::DivideByVoxelVolumeCm3,
    }
}

fn leakage_tally_contract(id: u32, name: &str, particle: ParticleType) -> OpenMcTallyContract {
    OpenMcTallyContract {
        id,
        name: name.into(),
        component: None,
        particle: Some(particle),
        quantity: OpenMcTallyQuantity::SurfaceCurrent,
        raw_unit: OpenMcRawTallyUnit::ParticlesPerSourceNeutron,
        collection_normalization: OpenMcCollectionNormalization::None,
    }
}

fn generated_file(path: &str, media_type: &str, bytes: Vec<u8>) -> GeneratedOpenMcFile {
    GeneratedOpenMcFile {
        relative_path: path.into(),
        media_type: media_type.into(),
        sha256: sha256_hex(&bytes),
        bytes,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn format_float(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

fn format_numbers(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format_float(*value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_integers<T: ToString>(values: &[T]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[derive(Debug, Error, PartialEq)]
pub enum OpenMcProfileError {
    #[error("unsupported OpenMC execution-profile schema {0:?}")]
    UnsupportedSchema(String),
    #[error("OpenMC execution-profile ID is empty")]
    EmptyId,
    #[error("OpenMC version {0:?} is unsupported by this execution profile")]
    UnsupportedOpenMcVersion(String),
    #[error("OpenMC source commit {0:?} is unsupported by this execution profile")]
    UnsupportedOpenMcCommit(String),
    #[error("at least two active tally batches are required; observed {0}")]
    InsufficientBatches(u32),
    #[error("candidate-reference runs require at least 50 active batches; observed {0}")]
    InsufficientCandidateBatches(u32),
    #[error("candidate-reference seed {0} is not in the frozen three-seed set")]
    UnregisteredCandidateSeed(u64),
    #[error("OpenMC seed must be nonzero")]
    ZeroSeed,
    #[error("OpenMC execution setting {0} is outside the frozen profile")]
    UnsupportedSetting(&'static str),
    #[error("{0} diagnostic energy grid is invalid or does not cover 0 to 20 MeV")]
    InvalidDiagnosticEnergyGrid(&'static str),
    #[error("{particle} diagnostic grid lacks required boundary {boundary_ev} eV")]
    MissingDiagnosticBoundary {
        particle: &'static str,
        boundary_ev: f64,
    },
}

#[derive(Debug, Error)]
pub enum OpenMcInputError {
    #[error("transport case is invalid: {0}")]
    InvalidTransportCase(String),
    #[error("{artifact} JSON is invalid: {source}")]
    InvalidJson {
        artifact: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("component profile is invalid: {0}")]
    InvalidComponentProfile(String),
    #[error("material artifact is invalid: {0}")]
    InvalidMaterial(String),
    #[error("source artifact is invalid: {0}")]
    InvalidSource(String),
    #[error("neutron response set is invalid or unreviewed: {0}")]
    InvalidResponseSet(String),
    #[error(transparent)]
    InvalidExecutionProfile(#[from] OpenMcProfileError),
    #[error(transparent)]
    InvalidNuclearData(#[from] NuclearDataError),
    #[error("transport case material differs from the content-bound material artifact")]
    CaseMaterialMismatch,
    #[error("transport case source differs from the content-bound source artifact")]
    CaseSourceMismatch,
    #[error("{label} content binding mismatch: declared {declared:?}, observed {observed:?}")]
    ContentBindingMismatch {
        label: &'static str,
        declared: ContentReference,
        observed: ContentReference,
    },
    #[error(
        "reviewed response energy range {response_ev:?} does not cover selected neutron-data range {data_ev:?} eV"
    )]
    ResponseEnergyRangeDoesNotCoverData {
        response_ev: [f64; 2],
        data_ev: [f64; 2],
    },
    #[error("source energy {source_ev} eV is outside selected neutron-data range {data_ev:?}")]
    SourceEnergyOutsideDataRange { source_ev: f64, data_ev: [f64; 2] },
    #[error("source particle {0:?} is unsupported by the first OpenMC profile")]
    UnsupportedSourceParticle(ParticleType),
    #[error("first OpenMC profile supports only an identity LPS direction; observed {0:?}")]
    UnsupportedGeometryDirection([f64; 9]),
    #[error("source plane must lie strictly inside all six transport boundaries")]
    SourceOutsideGeometry,
    #[error("{histories} histories are not divisible by {batches} active batches")]
    HistoriesNotDivisibleByBatches { histories: u64, batches: u32 },
    #[error("particles per batch must fit a positive signed 64-bit OpenMC count; observed {0}")]
    InvalidParticlesPerBatch(u64),
    #[error("failed to write deterministic OpenMC XML: {0}")]
    XmlWrite(#[source] io::Error),
    #[error("failed to serialize NCTForge input manifest: {0}")]
    ManifestSerialization(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nctforge_core::GridGeometry;
    use nctforge_transport::{ResponseInterpolation, ResponseSetQualification, ResponseUnit};
    use quick_xml::Reader;

    use crate::{
        DataArtifact, DataDistributionIdentity, DataInspectionIdentity, NeutronTableCapability,
        PhotonTableCapability, TARGET_DATA_HDF5_VERSION, TARGET_EVALUATED_DATA_RELEASE,
        TARGET_INSPECTION_METHOD,
    };

    use super::*;

    const COMPONENT_PROFILE_JSON: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/component-profile.json"
    );
    const MATERIAL_JSON: &[u8] =
        include_bytes!("../../../benchmarks/synthetic/nf-bnct-001/transport/material.json");
    const SOURCE_JSON: &[u8] =
        include_bytes!("../../../benchmarks/synthetic/nf-bnct-001/transport/source.json");
    const PROFILE_JSON: &[u8] = include_bytes!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/openmc-smoke-profile.json"
    );

    fn case() -> TransportCase {
        TransportCase {
            schema_version: "nctforge.transport-case/0.1.0".into(),
            case_id: "nf-bnct-001".into(),
            geometry: GridGeometry {
                shape: [40; 3],
                spacing_mm: [5.0; 3],
                origin_mm: [-97.5; 3],
                direction: IDENTITY_DIRECTION,
            },
            material: serde_json::from_slice(MATERIAL_JSON).unwrap(),
            source: serde_json::from_slice(SOURCE_JSON).unwrap(),
            requested_histories: 1_000,
        }
    }

    fn artifact(path: impl Into<String>) -> DataArtifact {
        DataArtifact {
            relative_path: path.into(),
            sha256: "a".repeat(64),
        }
    }

    fn nuclear_data() -> NuclearDataManifest {
        let case = case();
        let mut neutron_tables = case
            .material
            .nuclides
            .iter()
            .map(|nuclide| NeutronTableCapability {
                nuclide: nuclide.name.clone(),
                artifact: artifact(format!("neutron/{}.h5", nuclide.name)),
                hdf5_version: TARGET_DATA_HDF5_VERSION,
                atomic_weight_ratio: 1.0,
                temperatures_k: vec![294.0],
                energy_ranges_ev: vec![[1.0e-5, 20.0e6]],
                reactions_mt: match nuclide.name.as_str() {
                    "B10" => vec![107, 301],
                    "N14" => vec![103, 301],
                    _ => vec![301],
                },
                photon_production_mts: match nuclide.name.as_str() {
                    "B10" => vec![107],
                    "H1" => vec![102],
                    _ => Vec::new(),
                },
            })
            .collect::<Vec<_>>();
        neutron_tables.sort_by(|left, right| left.nuclide.cmp(&right.nuclide));
        let elements = case
            .material
            .nuclides
            .iter()
            .map(|nuclide| {
                nuclide
                    .name
                    .chars()
                    .take_while(char::is_ascii_alphabetic)
                    .collect::<String>()
            })
            .collect::<BTreeSet<_>>();
        let photon_tables = elements
            .into_iter()
            .map(|element| PhotonTableCapability {
                artifact: artifact(format!("photon/{element}.h5")),
                element,
                hdf5_version: TARGET_DATA_HDF5_VERSION,
                reactions_mt: vec![502, 504, 522],
                has_atomic_relaxation_data: true,
                has_compton_profile_data: true,
            })
            .collect();
        NuclearDataManifest {
            schema_version: crate::data::TARGET_NUCLEAR_DATA_MANIFEST_SCHEMA.into(),
            id: "nctforge.nf-bnct-001.endf-b-viii.1.test".into(),
            openmc_version: TARGET_OPENMC_VERSION.into(),
            openmc_source_commit: TARGET_OPENMC_SOURCE_COMMIT.into(),
            evaluated_data_release: TARGET_EVALUATED_DATA_RELEASE.into(),
            inspection: DataInspectionIdentity {
                method: TARGET_INSPECTION_METHOD.into(),
                source_sha256: "b".repeat(64),
                python_version: "3.14.4".into(),
                numpy_version: "2.5.2".into(),
                h5py_version: "3.16.0".into(),
                hdf5_library_version: "2.0.0".into(),
            },
            distribution: DataDistributionIdentity {
                id: "synthetic-test-data".into(),
                source_uri: crate::data::TARGET_DISTRIBUTION_SOURCE_URI.into(),
                archive_size_bytes: crate::data::TARGET_DISTRIBUTION_ARCHIVE_SIZE_BYTES,
                archive_sha256: "c".repeat(64),
                acquisition_profile_id: crate::data::TARGET_ACQUISITION_PROFILE_ID.into(),
                acquisition_profile_sha256: crate::data::TARGET_ACQUISITION_PROFILE_SHA256.into(),
                acquisition_receipt_sha256: "0".repeat(64),
                publisher_digest_status: crate::PublisherDigestStatus::Unavailable,
                acquisition_evidence_state: crate::AcquisitionEvidenceState::AcquisitionOnly,
            },
            cross_sections: artifact("cross_sections.xml"),
            neutron_tables,
            photon_tables,
        }
    }

    fn json_bytes<T: Serialize>(value: &T) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(value).unwrap();
        bytes.push(b'\n');
        bytes
    }

    fn response_set(nuclear_data_json: &[u8]) -> NeutronResponseSet {
        let component: ComponentDefinitionProfile =
            serde_json::from_slice(COMPONENT_PROFILE_JSON).unwrap();
        let material: MaterialDefinition = serde_json::from_slice(MATERIAL_JSON).unwrap();
        let nuclear_data: NuclearDataManifest = serde_json::from_slice(nuclear_data_json).unwrap();
        let boron = vec![1.0e-12, 2.0e-12, 3.0e-12];
        let nitrogen = vec![2.0e-12, 3.0e-12, 4.0e-12];
        let hydrogen = vec![3.0e-12, 4.0e-12, 5.0e-12];
        let total_neutron_gy_cm2 = boron
            .iter()
            .zip(&nitrogen)
            .zip(&hydrogen)
            .map(|((boron, nitrogen), hydrogen)| boron + nitrogen + hydrogen)
            .collect();
        NeutronResponseSet {
            schema_version: "nctforge.neutron-response-set/0.1.0".into(),
            id: "nctforge.nf-bnct-001.response.test".into(),
            qualification: ResponseSetQualification::IndependentlyReviewed,
            component_profile: content_reference(&component.id, COMPONENT_PROFILE_JSON),
            material: content_reference(&material.id, MATERIAL_JSON),
            nuclear_data_manifest: content_reference(&nuclear_data.id, nuclear_data_json),
            generation_method: ContentReference {
                id: "nctforge.nf-bnct-001.response-generation.test".into(),
                sha256: "d".repeat(64),
            },
            independent_review: Some(ContentReference {
                id: "nctforge.nf-bnct-001.response-review.test".into(),
                sha256: "e".repeat(64),
            }),
            transport_energy_range_ev: [1.0e-5, 20.0e6],
            energy_ev: vec![1.0e-5, 1_000.0, 20.0e6],
            unit: ResponseUnit::GraySquareCentimeter,
            interpolation: ResponseInterpolation::LinearLinear,
            boron_gy_cm2: boron,
            nitrogen_gy_cm2: nitrogen,
            hydrogen_gy_cm2: hydrogen,
            total_neutron_gy_cm2,
        }
    }

    struct InputBytes {
        data_root: tempfile::TempDir,
        nuclear_data_json: Vec<u8>,
        response_set_json: Vec<u8>,
    }

    fn input_bytes() -> InputBytes {
        let data_root = tempfile::tempdir().unwrap();
        std::fs::create_dir(data_root.path().join("neutron")).unwrap();
        std::fs::create_dir(data_root.path().join("photon")).unwrap();
        let mut nuclear_data = nuclear_data();
        let mut libraries = Vec::new();
        for table in &mut nuclear_data.neutron_tables {
            let bytes = format!("synthetic neutron table {}\n", table.nuclide).into_bytes();
            std::fs::write(data_root.path().join(&table.artifact.relative_path), &bytes).unwrap();
            table.artifact.sha256 = sha256_hex(&bytes);
            libraries.push(format!(
                "  <library materials=\"{}\" path=\"{}\" type=\"neutron\"/>",
                table.nuclide, table.artifact.relative_path
            ));
        }
        for table in &mut nuclear_data.photon_tables {
            let bytes = format!("synthetic photon table {}\n", table.element).into_bytes();
            std::fs::write(data_root.path().join(&table.artifact.relative_path), &bytes).unwrap();
            table.artifact.sha256 = sha256_hex(&bytes);
            libraries.push(format!(
                "  <library materials=\"{}\" path=\"{}\" type=\"photon\"/>",
                table.element, table.artifact.relative_path
            ));
        }
        let cross_sections = format!(
            "<cross_sections>\n{}\n</cross_sections>\n",
            libraries.join("\n")
        );
        std::fs::write(
            data_root.path().join("cross_sections.xml"),
            cross_sections.as_bytes(),
        )
        .unwrap();
        nuclear_data.cross_sections.sha256 = sha256_hex(cross_sections.as_bytes());

        let nuclear_data_json = json_bytes(&nuclear_data);
        let response_set_json = json_bytes(&response_set(&nuclear_data_json));
        InputBytes {
            data_root,
            nuclear_data_json,
            response_set_json,
        }
    }

    fn generate() -> OpenMcInputDeck {
        let inputs = input_bytes();
        OpenMcInputDeck::generate(
            &case(),
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &inputs.response_set_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap()
    }

    #[test]
    fn generates_deterministic_complete_openmc_deck() {
        let first = generate();
        let second = generate();
        assert_eq!(first, second);
        assert_eq!(first.files.len(), 5);
        assert_eq!(first.manifest.execution.particles_per_batch, 200);
        assert_eq!(first.manifest.scoring_mesh.lower_left_cm, [-10.0; 3]);
        assert_eq!(first.manifest.scoring_mesh.upper_right_cm, [10.0; 3]);
        assert_eq!(first.manifest.scoring_mesh.voxel_volume_cm3, 0.125);
        assert_eq!(first.manifest.scoring_mesh.voxel_mass_g, 0.125);
        assert_eq!(first.manifest.tallies.len(), 12);

        for file in &first.files {
            assert_eq!(file.sha256, sha256_hex(&file.bytes));
            if file.relative_path.ends_with(".xml") {
                let mut reader = Reader::from_reader(file.bytes.as_slice());
                loop {
                    if matches!(reader.read_event().unwrap(), Event::Eof) {
                        break;
                    }
                }
            }
        }
        assert_eq!(
            first
                .files
                .iter()
                .map(|file| (file.relative_path.as_str(), file.sha256.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "geometry.xml",
                    "ef76acd269a40e33e97ee43879da974e909354d8e26dd8b7e563b81bc0a5eef9"
                ),
                (
                    "materials.xml",
                    "d1451578d62dc078b97780b8ccfd16f7db27105ec14811ec02770f54d5dcfdd0"
                ),
                (
                    "settings.xml",
                    "4df825f159916242fb048312fcec4f9c16ffdf2454843e8ea7939e6b7b49a3aa"
                ),
                (
                    "tallies.xml",
                    "96eeeff3433f1946d82b64b199b450c3e15ba9481c6be25b5a75aeabb024faaa"
                ),
                (
                    "nctforge-input-manifest.json",
                    "a0a99a8e8c21d7b02d90c943f8dc43c104a5254b7100f9660cf999eac2b5957b"
                ),
            ]
        );

        let settings = std::str::from_utf8(&first.file("settings.xml").unwrap().bytes).unwrap();
        assert!(settings.contains("<run_mode>fixed source</run_mode>"));
        assert!(settings.contains("<particles>200</particles>"));
        assert!(settings.contains("<electron_treatment>led</electron_treatment>"));
        assert!(settings.contains("<photon_transport>true</photon_transport>"));

        let tallies = std::str::from_utf8(&first.file("tallies.xml").unwrap().bytes).unwrap();
        assert!(tallies.contains("type=\"energyfunction\""));
        assert!(tallies.contains("<scores>(n,a)</scores>"));
        assert!(tallies.contains("<scores>(n,p)</scores>"));
        assert!(tallies.contains("nctforge.physical_total.coupled_heating"));
    }

    #[test]
    fn rejects_content_binding_drift() {
        let inputs = input_bytes();
        let mut response = response_set(&inputs.nuclear_data_json);
        response.material.sha256 = "f".repeat(64);
        let response_json = json_bytes(&response);
        let error = OpenMcInputDeck::generate(
            &case(),
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &response_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OpenMcInputError::ContentBindingMismatch {
                label: "response_set.material",
                ..
            }
        ));
    }

    #[test]
    fn rejects_tampered_selected_nuclear_data() {
        let inputs = input_bytes();
        std::fs::write(inputs.data_root.path().join("neutron/B10.h5"), b"tampered").unwrap();
        let error = OpenMcInputDeck::generate(
            &case(),
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &inputs.response_set_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OpenMcInputError::InvalidNuclearData(NuclearDataError::HashMismatch {
                path,
                ..
            }) if path == "neutron/B10.h5"
        ));
    }

    #[test]
    fn rejects_response_domain_that_would_score_silent_zeros() {
        let inputs = input_bytes();
        let mut response = response_set(&inputs.nuclear_data_json);
        response.transport_energy_range_ev[1] = 19.0e6;
        let response_json = json_bytes(&response);
        let error = OpenMcInputDeck::generate(
            &case(),
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &response_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OpenMcInputError::ResponseEnergyRangeDoesNotCoverData { .. }
        ));
    }

    #[test]
    fn rejects_nonidentity_first_profile_geometry() {
        let inputs = input_bytes();
        let mut case = case();
        case.geometry.direction = [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let error = OpenMcInputDeck::generate(
            &case,
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &inputs.response_set_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OpenMcInputError::UnsupportedGeometryDirection(_)
        ));
    }

    #[test]
    fn rejects_fractional_batch_partition() {
        let inputs = input_bytes();
        let mut case = case();
        case.requested_histories = 1_001;
        let error = OpenMcInputDeck::generate(
            &case,
            inputs.data_root.path(),
            OpenMcInputArtifacts {
                component_profile_json: COMPONENT_PROFILE_JSON,
                material_json: MATERIAL_JSON,
                source_json: SOURCE_JSON,
                response_set_json: &inputs.response_set_json,
                nuclear_data_manifest_json: &inputs.nuclear_data_json,
                execution_profile_json: PROFILE_JSON,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OpenMcInputError::HistoriesNotDivisibleByBatches {
                histories: 1_001,
                batches: 5
            }
        ));
    }
}
