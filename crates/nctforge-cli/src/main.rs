// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use nctforge_bio::{BiologicalModel, RegionMask, apply_biological_model};
use nctforge_core::{ExposurePlan, PhysicalDoseBundle, accumulate_exposures};
use nctforge_dicom::synthetic::generate_nf_bnct_001;
use nctforge_dicom::{load_nf_bnct_001, verify_nf_bnct_001};
use nctforge_nifti::read_nifti_file;
use nctforge_njoy::{
    DEFAULT_CAPTURE_ENERGY_BALANCE_RELATIVE_TOLERANCE,
    DEFAULT_LAW7_BREAKUP_NORMALIZATION_TOLERANCE, DEFAULT_LAW7_BREAKUP_RELATIVE_ENERGY_TOLERANCE,
    DEFAULT_NJOY_CAPTURE_PRINT_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_ENERGY_BALANCE_PRINT_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_LAW7_PRINT_RELATIVE_TOLERANCE, DEFAULT_NJOY_LAW7_SOURCE_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_PRINT_RELATIVE_TOLERANCE, DEFAULT_NJOY_TIMEOUT_SECONDS,
    DEFAULT_REACTION_BALANCE_RELATIVE_TOLERANCE, DEFAULT_SPECTRUM_NORMALIZATION_TOLERANCE,
    EndfContinuumPhotonMomentReport, EndfContinuumPhotonMomentReportDocument,
    EndfMf6CapturePhotonBalanceQualification, EndfMf6CapturePhotonBalanceReport,
    EndfMf6CapturePhotonBalanceReportDocument, EndfMf6Law7ImplicitResidualQualification,
    EndfMf6Law7ImplicitResidualReport, EndfMf6Law7ImplicitResidualReportDocument,
    EndfPhotonProductionInventory, EndfPhotonProductionInventoryDocument,
    EndfReactionBalanceQualification, EndfReactionEnergyBalanceDocument,
    EndfReactionEnergyBalanceReport, NjoyAcquisitionArtifacts, NjoyCandidateComparisonCheckResult,
    NjoyCapturePhotonMomentComparison, NjoyCapturePhotonMomentComparisonDocument,
    NjoyDiagnosticTriageCheckResult, NjoyDiagnosticTriageReport,
    NjoyDiagnosticTriageReportDocument, NjoyDomainAwareSuitabilityReport,
    NjoyDomainAwareSuitabilityReportDocument, NjoyEnergyBalanceAttribution,
    NjoyEnergyBalanceAttributionDocument, NjoyEnergyBalanceAttributionQualification,
    NjoyEvidenceAwareCheckResult, NjoyEvidenceAwareSuitabilityReport,
    NjoyEvidenceAwareSuitabilityReportDocument, NjoyExecutionOptions, NjoyExecutionReceipt,
    NjoyExecutionReceiptDocument, NjoyInputArtifacts, NjoyInputBundle,
    NjoyLaw7ImplicitResidualComparison, NjoyLaw7ImplicitResidualComparisonDocument,
    NjoyLaw7ImplicitResidualComparisonQualification, NjoyPhotonMomentComparison,
    NjoyPhotonMomentComparisonDocument, NjoyResponseSetReviewDocument, NjoyResponseSetReviewReport,
    NjoyResponseTableGeneration, NjoyResponseTableInputs, NjoySourceAwareSuitabilityReport,
    NjoySourceAwareSuitabilityReportDocument, NjoySuitabilityComparison,
    NjoySuitabilityComparisonDocument, NjoySuitabilityComparisonQualification,
    NjoySuitabilityQualification, NjoySuitabilityReport, NjoySuitabilityReportDocument,
    load_generation_report, load_response_set,
};
use nctforge_openmc::{
    DataAcquisitionClient, DataAcquisitionProfileDocument, DataAcquisitionReceiptDocument,
    EvaluatedNeutronSourceSelectionDocument, EvaluatedSourceQualification, NuclearDataManifest,
    OpenMcBackend, OpenMcInputArtifacts, OpenMcInputDeck, OpenMcNeutronTransportDomain,
    OpenMcNeutronTransportDomainDocument, evaluate_runs,
};
use nctforge_transport::{
    CompletedRun, ComponentDefinitionProfile, MATERIAL_ASSIGNMENT_SCHEMA, MaterialAssignment,
    MaterialDefinition, MaterialRegion, ResponseGenerationMethod, TransportBackend, TransportCase,
};

#[derive(Debug, Parser)]
#[command(
    name = "nctforge",
    version,
    about = "Transport-neutral BNCT research and verification workbench"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the current transport-adapter boundary.
    Backends,
    /// Generate or verify frozen public benchmark cases.
    Benchmark(BenchmarkArgs),
    /// Prepare and audit OpenMC-specific research artifacts.
    Openmc(OpenMcArgs),
    /// Prepare deterministic NJOY response-generation artifacts.
    Njoy(NjoyArgs),
    /// Apply a separately versioned biological model to a physical dose bundle.
    Bio(BioArgs),
    /// Compute a dose-volume histogram over a named voxel mask.
    Dvh {
        /// Physical or biological dose bundle JSON.
        #[arg(long)]
        dose: PathBuf,
        /// `component:boron|nitrogen|hydrogen|photon`, `physical_total`, or
        /// `biological_total` (the last only for biological bundles).
        #[arg(long)]
        quantity: String,
        /// RegionMask JSON (`name` + per-voxel `voxels` booleans).
        #[arg(long)]
        mask: PathBuf,
        /// Number of equal-width dose bins over [0, max].
        #[arg(long, default_value_t = 100)]
        bins: usize,
        /// New output path for the DVH JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Import, inspect, and export NIfTI-1 volumes on the patient grid.
    Nifti(NiftiArgs),
    /// Accumulate weighted exposures (fields/fractions) into one physical
    /// dose bundle under a declared exposure plan.
    Accumulate {
        /// `nctforge.exposure-plan/0.1.0` JSON; bundle paths resolve
        /// relative to this file's directory.
        #[arg(long)]
        plan: PathBuf,
        /// New output path for the accumulated physical dose bundle.
        #[arg(long)]
        output: PathBuf,
    },
    /// Export or verify a deterministic evidence bundle.
    Evidence(EvidenceArgs),
    /// Combine or construct RegionMask volumes (subtraction, union,
    /// intersection, CT-threshold regions) for limiting-organ construction.
    Mask(MaskArgs),
    /// Aim a fixed source at a region centroid or rotate a source about a
    /// patient axis (research positioning helpers).
    Position(PositionArgs),
    /// Evaluate organ-limited irradiation time over a per-source-particle
    /// dose endpoint, reporting the limiting structure and assumptions.
    IrradiationTime {
        /// Physical or biological dose bundle JSON.
        #[arg(long)]
        dose: PathBuf,
        /// `component:NAME`, `physical_total`, or `biological_total`.
        #[arg(long)]
        quantity: String,
        /// Source strength in source particles per second.
        #[arg(long)]
        source_strength: f64,
        /// Region limit `NAME=max|mean:LIMIT` in endpoint dose units;
        /// repeatable.
        #[arg(long = "limit", required = true)]
        limits: Vec<String>,
        /// RegionMask binding `NAME=path`; repeatable.
        #[arg(long = "mask", required = true)]
        masks: Vec<String>,
        /// New output path for the irradiation-time report JSON.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct NiftiArgs {
    #[command(subcommand)]
    command: NiftiCommand,
}

#[derive(Debug, Subcommand)]
enum NiftiCommand {
    /// Print a NIfTI file's grid, transform provenance, and datatype.
    Info {
        /// `.nii` or gzip-compressed `.nii.gz` file.
        #[arg(long)]
        input: PathBuf,
    },
    /// Convert a NIfTI volume to a RegionMask (nonzero voxels included).
    ToMask {
        /// `.nii` or gzip-compressed `.nii.gz` file.
        #[arg(long)]
        input: PathBuf,
        /// Mask name recorded in the RegionMask JSON.
        #[arg(long)]
        name: String,
        /// New output path for the mask JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Export a dose-bundle component or total as a float64 `.nii` volume.
    ExportDose {
        /// Physical dose bundle JSON.
        #[arg(long)]
        dose: PathBuf,
        /// `component:boron|nitrogen|hydrogen|photon` or `physical_total`.
        #[arg(long)]
        quantity: String,
        /// New output path (`.nii`, or `.nii.gz` for gzip).
        #[arg(long)]
        output: PathBuf,
    },
    /// Resample a NIfTI volume onto a dose bundle's grid.
    Resample {
        /// `.nii` or gzip-compressed `.nii.gz` file.
        #[arg(long)]
        input: PathBuf,
        /// Physical dose bundle JSON supplying the target grid.
        #[arg(long)]
        target: PathBuf,
        /// `nearest` (masks/labels) or `trilinear` (dose/intensity).
        #[arg(long)]
        interpolation: String,
        /// New output path for the resampled `.nii` file.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct PositionArgs {
    #[command(subcommand)]
    command: PositionCommand,
}

#[derive(Debug, Subcommand)]
enum PositionCommand {
    /// Derive a source aimed so the beam axis passes through a mask's
    /// centroid, emitting a positioned source JSON and a position report.
    Aim {
        /// Transport-case JSON supplying the patient grid.
        #[arg(long)]
        case: PathBuf,
        /// Source JSON whose particle/energy/id the result inherits.
        #[arg(long)]
        source: PathBuf,
        /// RegionMask JSON whose centroid is the aim target.
        #[arg(long)]
        mask: PathBuf,
        /// Axis approach `+x|-x|+y|-y|+z|-z` (conflicts with --direction).
        #[arg(
            long,
            conflicts_with = "direction",
            required_unless_present = "direction"
        )]
        approach: Option<String>,
        /// Arbitrary beam direction `dx,dy,dz` in LPS (conflicts with --approach).
        #[arg(long)]
        direction: Option<String>,
        /// Aperture half-widths `HU,HV` in cm along the plane's two axes.
        #[arg(long, value_delimiter = ',')]
        half_widths_cm: Vec<f64>,
        /// How far inside the entry face the source plane sits, in cm.
        #[arg(long, default_value_t = 0.01)]
        margin_cm: f64,
        /// New output path for the positioned source JSON.
        #[arg(long)]
        output_source: PathBuf,
        /// New output path for the position report JSON.
        #[arg(long)]
        output_report: PathBuf,
    },
    /// Rotate a source's plane, aperture, and beam direction about a world
    /// axis by a multiple of 90 degrees (right-hand rule).
    Rotate {
        /// Source JSON to rotate.
        #[arg(long)]
        source: PathBuf,
        /// World axis to rotate about: `x`, `y`, or `z`.
        #[arg(long)]
        axis: String,
        /// Rotation in degrees; must be a multiple of 90.
        #[arg(long)]
        degrees: f64,
        /// Rotation center `x,y,z` in LPS mm (default: world origin).
        #[arg(long, value_delimiter = ',', default_values_t = [0.0, 0.0, 0.0])]
        center_mm: Vec<f64>,
        /// New output path for the rotated source JSON.
        #[arg(long)]
        output_source: PathBuf,
    },
}

#[derive(Debug, Args)]
struct MaskArgs {
    #[command(subcommand)]
    command: MaskCommand,
}

#[derive(Debug, Subcommand)]
enum MaskCommand {
    /// Subtract one or more masks from an input mask (e.g. organ minus tumor).
    Subtract {
        /// RegionMask JSON to subtract from.
        #[arg(long)]
        input: PathBuf,
        /// RegionMask JSON subtracted from the input; repeatable.
        #[arg(long, required = true)]
        minus: Vec<PathBuf>,
        /// Name recorded in the output mask.
        #[arg(long)]
        name: String,
        /// New output path for the mask JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Union two or more masks.
    Union {
        /// RegionMask JSONs to combine; repeatable, at least two.
        #[arg(long, required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        /// Name recorded in the output mask.
        #[arg(long)]
        name: String,
        /// New output path for the mask JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Intersect two or more masks.
    Intersect {
        /// RegionMask JSONs to intersect; repeatable, at least two.
        #[arg(long, required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        /// Name recorded in the output mask.
        #[arg(long)]
        name: String,
        /// New output path for the mask JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Construct a mask from a DICOM CT series whose rescaled modality
    /// values (HU) fall inside an inclusive window.
    Threshold {
        /// Directory containing the CT slice DICOM files.
        #[arg(long)]
        ct_dir: PathBuf,
        /// Inclusive lower bound of the modality-value window.
        #[arg(long, allow_hyphen_values = true)]
        min: f64,
        /// Inclusive upper bound of the modality-value window.
        #[arg(long, allow_hyphen_values = true)]
        max: f64,
        /// Name recorded in the output mask.
        #[arg(long)]
        name: String,
        /// New output path for the mask JSON.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct EvidenceArgs {
    #[command(subcommand)]
    command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Copy artifacts into a new directory and freeze an artifact manifest.
    Export {
        /// New bundle directory; it must not already exist.
        #[arg(long)]
        root: PathBuf,
        /// Case identifier recorded in the manifest.
        #[arg(long)]
        case_id: String,
        /// `synthetic_research_only`, `cross_code_research_only`, or
        /// `experimentally_validated_research_only`.
        #[arg(long)]
        qualification: String,
        /// One `role=SOURCE:DEST` triple per artifact; DEST is the
        /// bundle-relative path and may not escape the root.
        #[arg(long = "artifact", required = true)]
        artifacts: Vec<String>,
    },
    /// Re-hash every artifact declared by a bundle's manifest.
    Verify {
        /// Bundle directory containing artifact-manifest.json.
        #[arg(long)]
        root: PathBuf,
    },
}

#[derive(Debug, Args)]
struct BenchmarkArgs {
    #[command(subcommand)]
    command: BenchmarkCommand,
}

#[derive(Debug, Subcommand)]
enum BenchmarkCommand {
    /// Generate deterministic DICOM inputs for NF-BNCT-001.
    Generate {
        /// New destination directory; it must not already exist.
        output: PathBuf,
    },
    /// Import and verify an NF-BNCT-001 directory against the frozen oracle.
    Verify {
        /// Directory containing ct/*.dcm and rtstruct.dcm.
        input: PathBuf,
    },
    /// Derive a material assignment from a verified case's RT structure set.
    /// ROIs that fill their bounding box become exact CSG box regions; other
    /// masks become exact voxel-set regions realized as lattice elements.
    /// Emits the assignment plus a derived transport case whose base material
    /// is the supplied file.
    DeriveMaterials {
        /// Verified NF-BNCT-001 case directory (ct/, rtstruct.dcm, case.json).
        #[arg(long)]
        case_root: PathBuf,
        /// Transport-case JSON supplying geometry, source, and histories.
        #[arg(long)]
        case: PathBuf,
        /// Base material filling all voxels outside mapped regions.
        #[arg(long)]
        base_material: PathBuf,
        /// JSON object {"regions": {"ROI_NAME": "material-file.json"}};
        /// material paths resolve relative to this file's directory.
        #[arg(long)]
        map: PathBuf,
        /// Region masks as `NAME=path` pairs (RegionMask JSON, e.g. from
        /// `nifti to-mask`). When supplied, map keys name these masks
        /// instead of RT Structure Set ROIs.
        #[arg(long = "mask")]
        masks: Vec<String>,
        /// New output path for the material-assignment JSON.
        #[arg(long)]
        output_assignment: PathBuf,
        /// New output path for the derived transport-case JSON.
        #[arg(long)]
        output_case: PathBuf,
    },
}

#[derive(Debug, Args)]
struct OpenMcArgs {
    #[command(subcommand)]
    command: OpenMcCommand,
}

#[derive(Debug, Subcommand)]
enum OpenMcCommand {
    /// Probe or acquire externally published nuclear data.
    Data(OpenMcDataArgs),
    /// Generate a deterministic OpenMC input deck bound to a reviewed response set.
    Generate {
        /// Transport-case JSON embedding the frozen geometry, material, and source.
        #[arg(long)]
        case: PathBuf,
        /// Component definition profile bound by the response set.
        #[arg(long)]
        component_profile: PathBuf,
        /// Exact material JSON bound by the case and response set.
        #[arg(long)]
        material: PathBuf,
        /// Exact source JSON bound by the case.
        #[arg(long)]
        source: PathBuf,
        /// Reviewed neutron response set (must satisfy `validate_for_folding`).
        #[arg(long)]
        response_set: PathBuf,
        /// Case-scoped OpenMC nuclear-data manifest.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Frozen OpenMC execution profile (for example the smoke profile).
        #[arg(long)]
        execution_profile: PathBuf,
        /// Root containing cross_sections.xml and every selected HDF5 file.
        #[arg(long)]
        nuclear_data_root: PathBuf,
        /// Predeclared acceptance contract (required for candidate-reference
        /// profiles, forbidden otherwise).
        #[arg(long)]
        acceptance: Option<PathBuf>,
        /// DICOM-derived material assignment (structure-derived
        /// cases only).
        #[arg(long)]
        assignment: Option<PathBuf>,
        /// New output directory for the generated deck; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Prepare, execute, and collect a run in one step, optionally exporting
    /// an evidence bundle over every input and output artifact.
    Run {
        /// Transport-case JSON embedding the frozen geometry, material, and source.
        #[arg(long)]
        case: PathBuf,
        /// Component definition profile bound by the response set.
        #[arg(long)]
        component_profile: PathBuf,
        /// Exact material JSON bound by the case and response set.
        #[arg(long)]
        material: PathBuf,
        /// Exact source JSON bound by the case.
        #[arg(long)]
        source: PathBuf,
        /// Reviewed neutron response set.
        #[arg(long)]
        response_set: PathBuf,
        /// Case-scoped OpenMC nuclear-data manifest.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Frozen OpenMC execution profile.
        #[arg(long)]
        execution_profile: PathBuf,
        /// Predeclared acceptance contract (required for candidate-reference
        /// profiles, forbidden otherwise).
        #[arg(long)]
        acceptance: Option<PathBuf>,
        /// DICOM-derived material assignment (structure-derived
        /// cases only).
        #[arg(long)]
        assignment: Option<PathBuf>,
        /// Root containing cross_sections.xml and every selected HDF5 file.
        #[arg(long)]
        nuclear_data_root: PathBuf,
        /// OpenMC executable to launch.
        #[arg(long)]
        openmc: PathBuf,
        /// Environment overlay as KEY=VALUE; may repeat (for example
        /// `LD_LIBRARY_PATH=...` or `OMP_NUM_THREADS=...`).
        #[arg(long = "env")]
        environment: Vec<String>,
        /// New working directory for the run; it must not already exist.
        #[arg(long)]
        working_directory: PathBuf,
        /// New output path for the collected physical dose bundle JSON.
        #[arg(long)]
        dose_output: PathBuf,
        /// New directory for a hash-bound evidence bundle over the run.
        #[arg(long)]
        evidence_root: Option<PathBuf>,
    },
    /// Collect a completed run's statepoint into a normalized dose bundle.
    Collect {
        /// Completed run directory containing the deck manifest and statepoint.
        #[arg(long)]
        working_directory: PathBuf,
        /// Exit code recorded for the run (refuses collection unless zero).
        #[arg(long, default_value_t = 0)]
        exit_code: i32,
        /// New output path for the physical dose bundle JSON.
        #[arg(long)]
        output: PathBuf,
    },
    /// Evaluate completed candidate-reference runs against their bound
    /// acceptance contract (precision, estimator, and seed-consistency gates).
    Evaluate {
        /// Completed run directory; repeat once per evaluated seed.
        #[arg(long = "run", required = true)]
        runs: Vec<PathBuf>,
        /// Exit code for each run, in the same order (default zero for all).
        #[arg(long = "exit-code")]
        exit_codes: Vec<i32>,
        /// New output path for the acceptance report JSON.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct OpenMcDataArgs {
    #[command(subcommand)]
    command: OpenMcDataCommand,
}

#[derive(Debug, Args)]
struct BioArgs {
    #[command(subcommand)]
    command: BioCommand,
}

#[derive(Debug, Subcommand)]
enum BioCommand {
    /// Produce a biological dose bundle from a physical dose bundle.
    Apply {
        /// Biological model JSON (`nctforge.biological-model/0.1.0`).
        #[arg(long)]
        model: PathBuf,
        /// Physical dose bundle JSON produced by `openmc collect`.
        #[arg(long)]
        physical_bundle: PathBuf,
        /// Region mask as `name=path` pairs; required when the model
        /// declares region weight overrides.
        #[arg(long = "region-mask")]
        region_masks: Vec<String>,
        /// New output path for the biological dose bundle JSON.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct NjoyArgs {
    #[command(subcommand)]
    command: NjoyCommand,
}

#[derive(Debug, Subcommand)]
enum NjoyCommand {
    /// Verify every binding and write deterministic per-nuclide NJOY decks.
    Prepare {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Exact material JSON bound by the response-generation method.
        #[arg(long)]
        material: PathBuf,
        /// Frozen response-generation method JSON.
        #[arg(long)]
        generation_method: PathBuf,
        /// Reviewed acquisition profile bound by the source selection; repeat
        /// once per bound acquisition, paired positionally with --receipt.
        #[arg(long)]
        profile: Vec<PathBuf>,
        /// Acquisition receipt bound by the source selection; repeat once per
        /// bound acquisition, paired positionally with --profile.
        #[arg(long)]
        receipt: Vec<PathBuf>,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// New output directory; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Inventory MF=6/12/13/14/15 photon-production records in exact ENDF sources.
    InventoryPhotonData {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// New source-bound JSON inventory path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a source-bound ENDF photon-production inventory.
    VerifyPhotonInventory {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Inventory to validate and regenerate.
        #[arg(long)]
        inventory: PathBuf,
    },
    /// Independently integrate File 15 spectra and fold them with File 13 cross sections.
    CalculatePhotonMoments {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Maximum accepted absolute error in weighted spectrum normalization.
        #[arg(long, default_value_t = DEFAULT_SPECTRUM_NORMALIZATION_TOLERANCE)]
        normalization_tolerance: f64,
        /// New source-moment JSON report path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify an independent continuum photon-moment report.
    VerifyPhotonMoments {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Continuum photon-moment report to validate and regenerate.
        #[arg(long)]
        moment_report: PathBuf,
    },
    /// Compare independent source moments with NJOY's diagnostic print tables.
    ComparePhotonMoments {
        /// Independently calculated continuum photon-moment report.
        #[arg(long)]
        moment_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Relative tolerance appropriate to NJOY's five-significant-digit printout.
        #[arg(long, default_value_t = DEFAULT_NJOY_PRINT_RELATIVE_TOLERANCE)]
        relative_tolerance: f64,
        /// New content-bound comparison JSON path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify an NJOY photon-moment print comparison.
    VerifyPhotonMomentComparison {
        /// Independently calculated continuum photon-moment report.
        #[arg(long)]
        moment_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Comparison report to validate and regenerate.
        #[arg(long)]
        comparison_report: PathBuf,
    },
    /// Independently test an MF=6/MT=102 photon source against its capture energy budget.
    CalculateCapturePhotonBalance {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Nuclide identifier in the source selection (for example, N15).
        #[arg(long)]
        nuclide: String,
        /// Maximum accepted absolute spectrum-normalization error.
        #[arg(long, default_value_t = DEFAULT_SPECTRUM_NORMALIZATION_TOLERANCE)]
        normalization_tolerance: f64,
        /// Maximum accepted relative residual in the capture energy budget.
        #[arg(long, default_value_t = DEFAULT_CAPTURE_ENERGY_BALANCE_RELATIVE_TOLERANCE)]
        relative_energy_tolerance: f64,
        /// New source-bound capture-balance JSON report; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify an independent MF=6 capture photon-balance report.
    VerifyCapturePhotonBalance {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Capture photon-balance report to validate and regenerate.
        #[arg(long)]
        balance_report: PathBuf,
    },
    /// Integrate deuterium MF=6/MT=16 LAW=7 and test the implicit proton energy.
    CalculateLaw7ImplicitResidual {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Deuterium nuclide identifier in the source selection (H2).
        #[arg(long, default_value = "H2")]
        nuclide: String,
        /// Maximum accepted absolute joint-distribution normalization error.
        #[arg(long, default_value_t = DEFAULT_LAW7_BREAKUP_NORMALIZATION_TOLERANCE)]
        normalization_tolerance: f64,
        /// Maximum accepted relative negative implicit-residual energy.
        #[arg(long, default_value_t = DEFAULT_LAW7_BREAKUP_RELATIVE_ENERGY_TOLERANCE)]
        relative_energy_tolerance: f64,
        /// New source-bound implicit-residual JSON report; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a deuterium LAW=7 implicit-residual report.
    VerifyLaw7ImplicitResidual {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Verified source-bound photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// LAW=7 implicit-residual report to validate and regenerate.
        #[arg(long)]
        residual_report: PathBuf,
    },
    /// Attribute H-2 LAW=7 warnings to NJOY's printed residual approximation.
    CompareLaw7ImplicitResidual {
        /// Independently calculated deuterium LAW=7 residual report.
        #[arg(long)]
        residual_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Maximum relative difference between source integration and NJOY quadrature.
        #[arg(long, default_value_t = DEFAULT_NJOY_LAW7_SOURCE_RELATIVE_TOLERANCE)]
        source_relative_tolerance: f64,
        /// Maximum relative difference for five-significant-digit print identities.
        #[arg(long, default_value_t = DEFAULT_NJOY_LAW7_PRINT_RELATIVE_TOLERANCE)]
        print_relative_tolerance: f64,
        /// New receipt-bound comparison JSON path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify the H-2 LAW=7 processor attribution.
    VerifyLaw7ImplicitResidualComparison {
        /// Independently calculated deuterium LAW=7 residual report.
        #[arg(long)]
        residual_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Comparison report to validate and regenerate.
        #[arg(long)]
        comparison_report: PathBuf,
    },
    /// Attribute in-domain MT=301 flags to NJOY's printed File 6 accounting.
    AttributeEnergyBalance {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Nuclide whose high MT=301 findings will be attributed.
        #[arg(long, default_value = "O17")]
        nuclide: String,
        /// Relative tolerance for NJOY's five-significant-digit print identities.
        #[arg(long, default_value_t = DEFAULT_NJOY_ENERGY_BALANCE_PRINT_RELATIVE_TOLERANCE)]
        print_relative_tolerance: f64,
        /// New receipt-bound attribution JSON path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a processor-only energy-balance attribution.
    VerifyEnergyBalanceAttribution {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Energy-balance attribution to validate and regenerate.
        #[arg(long)]
        attribution_report: PathBuf,
    },
    /// Integrate File 3/File 6 reaction-level energy balances independent of NJOY.
    CalculateReactionEnergyBalance {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing the selected extracted evaluations.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Receipt-bound processor energy-balance attribution for the nuclide.
        #[arg(long)]
        attribution_report: PathBuf,
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Nuclide whose reaction remainders will be integrated.
        #[arg(long, default_value = "O17")]
        nuclide: String,
        /// Relative tolerance for the printed ebal/ebar comparisons.
        #[arg(long, default_value_t = DEFAULT_REACTION_BALANCE_RELATIVE_TOLERANCE)]
        relative_tolerance: f64,
        /// New unreviewed balance JSON path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify an independent reaction energy-balance report.
    VerifyReactionEnergyBalance {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Directory containing the selected extracted evaluations.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Receipt-bound processor energy-balance attribution for the nuclide.
        #[arg(long)]
        attribution_report: PathBuf,
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Nuclide whose reaction remainders were integrated.
        #[arg(long, default_value = "O17")]
        nuclide: String,
        /// Energy-balance report to validate and regenerate.
        #[arg(long)]
        balance_report: PathBuf,
    },
    /// Generate the material neutron response set from production-HEATR PENDF tables.
    GenerateResponseTables {
        /// Exact material JSON bound by the response-generation method.
        #[arg(long)]
        material: PathBuf,
        /// Component-definition profile JSON bound by the method.
        #[arg(long)]
        component_profile: PathBuf,
        /// Frozen response-generation method JSON.
        #[arg(long)]
        generation_method: PathBuf,
        /// Processed nuclear-data manifest JSON supplying atomic weight ratios.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Derived neutron transport-domain JSON.
        #[arg(long)]
        transport_domain: PathBuf,
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Verified domain-aware suitability report carrying the findings.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// New unreviewed response-set JSON path; it must not already exist.
        #[arg(long)]
        response_set_output: PathBuf,
        /// New generation-report JSON path; it must not already exist.
        #[arg(long)]
        report_output: PathBuf,
    },
    /// Regenerate and verify a response set, then emit the review report and
    /// the independently reviewed response set that binds it.
    VerifyResponseTables {
        /// Exact material JSON bound by the response-generation method.
        #[arg(long)]
        material: PathBuf,
        /// Component-definition profile JSON bound by the method.
        #[arg(long)]
        component_profile: PathBuf,
        /// Frozen response-generation method JSON.
        #[arg(long)]
        generation_method: PathBuf,
        /// Processed nuclear-data manifest JSON supplying atomic weight ratios.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Derived neutron transport-domain JSON.
        #[arg(long)]
        transport_domain: PathBuf,
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Verified domain-aware suitability report carrying the findings.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Unreviewed response set to regenerate and validate.
        #[arg(long)]
        response_set: PathBuf,
        /// Generation report to regenerate and validate.
        #[arg(long)]
        generation_report: PathBuf,
        /// New review-report JSON path; it must not already exist.
        #[arg(long)]
        review_output: PathBuf,
        /// New independently reviewed response-set JSON path; it must not
        /// already exist.
        #[arg(long)]
        reviewed_set_output: PathBuf,
    },
    /// Compare independent capture moments with NJOY's photon and recoil print tables.
    CompareCapturePhotonMoments {
        /// Independently calculated MF=6 capture photon-balance report.
        #[arg(long)]
        balance_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Relative tolerance appropriate to NJOY's five-significant-digit printout.
        #[arg(long, default_value_t = DEFAULT_NJOY_CAPTURE_PRINT_RELATIVE_TOLERANCE)]
        relative_tolerance: f64,
        /// New content-bound comparison JSON path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify an NJOY MF=6 capture-moment print comparison.
    VerifyCapturePhotonMomentComparison {
        /// Independently calculated MF=6 capture photon-balance report.
        #[arg(long)]
        balance_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Comparison report to validate and regenerate.
        #[arg(long)]
        comparison_report: PathBuf,
    },
    /// Execute a verified input bundle and emit an unreviewed evidence receipt.
    Execute {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Exact material JSON bound by the response-generation method.
        #[arg(long)]
        material: PathBuf,
        /// Frozen response-generation method JSON.
        #[arg(long)]
        generation_method: PathBuf,
        /// Reviewed acquisition profile bound by the source selection; repeat
        /// once per bound acquisition, paired positionally with --receipt.
        #[arg(long)]
        profile: Vec<PathBuf>,
        /// Acquisition receipt bound by the source selection; repeat once per
        /// bound acquisition, paired positionally with --profile.
        #[arg(long)]
        receipt: Vec<PathBuf>,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
        /// Exact prepared bundle to verify before execution.
        #[arg(long)]
        input_bundle: PathBuf,
        /// Real regular NJOY executable to invoke with an empty environment.
        #[arg(long)]
        njoy_executable: PathBuf,
        /// Additional processor/runtime artifact to bind by hash; repeatable.
        #[arg(long = "processor-support-artifact")]
        processor_support_artifacts: Vec<PathBuf>,
        /// Per-nuclide wall-clock timeout.
        #[arg(long, default_value_t = DEFAULT_NJOY_TIMEOUT_SECONDS)]
        timeout_seconds: u64,
        /// New evidence directory; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify every execution artifact against an external receipt.
    VerifyExecution {
        /// Receipt used as the independent trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory, including its byte-identical receipt.
        #[arg(long)]
        execution_directory: PathBuf,
    },
    /// Derive a transported-photon KERMA suitability report from verified logs.
    AssessExecution {
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// New JSON report path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and compare a transported-photon suitability report.
    VerifySuitability {
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Suitability report to validate and regenerate.
        #[arg(long)]
        suitability_report: PathBuf,
    },
    /// Reinterpret verified diagnostics using source-bound ENDF photon records.
    AssessSourceAware {
        /// Verified legacy v0.1 transported-photon suitability report.
        #[arg(long)]
        legacy_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Exact NJOY input manifest bound by the execution receipt.
        #[arg(long)]
        input_manifest: PathBuf,
        /// Source-bound ENDF photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// New v0.2 JSON report path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a source-aware v0.2 suitability report.
    VerifySourceAware {
        /// Verified legacy v0.1 transported-photon suitability report.
        #[arg(long)]
        legacy_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Exact NJOY input manifest bound by the execution receipt.
        #[arg(long)]
        input_manifest: PathBuf,
        /// Source-bound ENDF photon-production inventory.
        #[arg(long)]
        photon_inventory: PathBuf,
        /// Source-aware v0.2 report to validate and regenerate.
        #[arg(long)]
        source_aware_report: PathBuf,
    },
    /// Scope source-aware kinematic findings to a content-bound transport domain.
    AssessDomainAware {
        /// Verified source-aware v0.2 transported-photon suitability report.
        #[arg(long)]
        source_aware_report: PathBuf,
        /// Verified legacy v0.1 transported-photon suitability report.
        #[arg(long)]
        legacy_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Exact NJOY input manifest bound by the execution receipt.
        #[arg(long)]
        input_manifest: PathBuf,
        /// Exact OpenMC nuclear-data manifest used to derive the domain.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Exact material JSON shared by the NJOY run and transport domain.
        #[arg(long)]
        material: PathBuf,
        /// Derived OpenMC neutron transport-domain document.
        #[arg(long)]
        transport_domain: PathBuf,
        /// New v0.3 JSON report path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a transport-domain-aware v0.3 suitability report.
    VerifyDomainAware {
        /// Verified source-aware v0.2 transported-photon suitability report.
        #[arg(long)]
        source_aware_report: PathBuf,
        /// Verified legacy v0.1 transported-photon suitability report.
        #[arg(long)]
        legacy_report: PathBuf,
        /// External execution receipt used as the trust anchor.
        #[arg(long)]
        receipt: PathBuf,
        /// Complete execution directory bound by the receipt.
        #[arg(long)]
        execution_directory: PathBuf,
        /// Exact NJOY input manifest bound by the execution receipt.
        #[arg(long)]
        input_manifest: PathBuf,
        /// Exact OpenMC nuclear-data manifest used to derive the domain.
        #[arg(long)]
        nuclear_data_manifest: PathBuf,
        /// Exact material JSON shared by the NJOY run and transport domain.
        #[arg(long)]
        material: PathBuf,
        /// Derived OpenMC neutron transport-domain document.
        #[arg(long)]
        transport_domain: PathBuf,
        /// Domain-aware v0.3 report to validate and regenerate.
        #[arg(long)]
        domain_aware_report: PathBuf,
    },
    /// Apply reaction-level H-2 and N-15 evidence over immutable v0.3 suitability.
    AssessEvidenceAware {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// Independent H-2 LAW=7 implicit-residual report.
        #[arg(long)]
        law7_residual_report: PathBuf,
        /// Receipt-bound H-2 LAW=7 processor attribution.
        #[arg(long)]
        law7_comparison_report: PathBuf,
        /// Independent N-15 MF=6 capture-balance report.
        #[arg(long)]
        capture_balance_report: PathBuf,
        /// Receipt-bound N-15 capture-moment comparison.
        #[arg(long)]
        capture_comparison_report: PathBuf,
        /// New v0.4 JSON report path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a reaction-evidence-aware v0.4 suitability report.
    VerifyEvidenceAware {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// Independent H-2 LAW=7 implicit-residual report.
        #[arg(long)]
        law7_residual_report: PathBuf,
        /// Receipt-bound H-2 LAW=7 processor attribution.
        #[arg(long)]
        law7_comparison_report: PathBuf,
        /// Independent N-15 MF=6 capture-balance report.
        #[arg(long)]
        capture_balance_report: PathBuf,
        /// Receipt-bound N-15 capture-moment comparison.
        #[arg(long)]
        capture_comparison_report: PathBuf,
        /// Evidence-aware v0.4 report to validate and regenerate.
        #[arg(long)]
        evidence_aware_report: PathBuf,
    },
    /// Separate source-data blockers from findings needing independent diagnostics.
    AssessDiagnosticTriage {
        /// Verified reaction-evidence-aware v0.4 suitability report.
        #[arg(long)]
        evidence_aware_report: PathBuf,
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// New diagnostic-triage JSON report; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a diagnostic-triage report.
    VerifyDiagnosticTriage {
        /// Verified reaction-evidence-aware v0.4 suitability report.
        #[arg(long)]
        evidence_aware_report: PathBuf,
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// Diagnostic-triage report to validate and regenerate.
        #[arg(long)]
        triage_report: PathBuf,
    },
    /// Verify the complete triage evidence chain and write a compact machine result.
    CheckDiagnosticTriage {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// Independent H-2 LAW=7 implicit-residual report.
        #[arg(long)]
        law7_residual_report: PathBuf,
        /// Receipt-bound H-2 LAW=7 processor attribution.
        #[arg(long)]
        law7_comparison_report: PathBuf,
        /// Independent N-15 MF=6 capture-balance report.
        #[arg(long)]
        capture_balance_report: PathBuf,
        /// Receipt-bound N-15 capture-moment comparison.
        #[arg(long)]
        capture_comparison_report: PathBuf,
        /// Verified reaction-evidence-aware v0.4 suitability report.
        #[arg(long)]
        evidence_aware_report: PathBuf,
        /// Verified diagnostic-triage report.
        #[arg(long)]
        triage_report: PathBuf,
        /// New deterministic JSON result; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify v0.4 evidence and write a compact machine-facing check result.
    CheckEvidenceAware {
        /// Verified domain-aware v0.3 transported-photon suitability report.
        #[arg(long)]
        domain_aware_report: PathBuf,
        /// Independent H-2 LAW=7 implicit-residual report.
        #[arg(long)]
        law7_residual_report: PathBuf,
        /// Receipt-bound H-2 LAW=7 processor attribution.
        #[arg(long)]
        law7_comparison_report: PathBuf,
        /// Independent N-15 MF=6 capture-balance report.
        #[arg(long)]
        capture_balance_report: PathBuf,
        /// Receipt-bound N-15 capture-moment comparison.
        #[arg(long)]
        capture_comparison_report: PathBuf,
        /// Evidence-aware v0.4 report to validate and regenerate.
        #[arg(long)]
        evidence_aware_report: PathBuf,
        /// New deterministic JSON result; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare a candidate suitability report against a rejected baseline.
    CompareSuitability {
        /// Rejected baseline transported-photon suitability report.
        #[arg(long)]
        baseline_report: PathBuf,
        /// Candidate transported-photon suitability report.
        #[arg(long)]
        candidate_report: PathBuf,
        /// New JSON comparison path; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a response-treatment candidate comparison.
    VerifyComparison {
        /// Rejected baseline transported-photon suitability report.
        #[arg(long)]
        baseline_report: PathBuf,
        /// Candidate transported-photon suitability report.
        #[arg(long)]
        candidate_report: PathBuf,
        /// Comparison report to validate and regenerate.
        #[arg(long)]
        comparison_report: PathBuf,
    },
    /// Verify a candidate comparison and write a compact machine result.
    CheckCandidateComparison {
        /// Rejected baseline transported-photon suitability report.
        #[arg(long)]
        baseline_report: PathBuf,
        /// Candidate transported-photon suitability report.
        #[arg(long)]
        candidate_report: PathBuf,
        /// Comparison report to validate and regenerate.
        #[arg(long)]
        comparison_report: PathBuf,
        /// New deterministic JSON result; it must not already exist.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum OpenMcDataCommand {
    /// Make a one-byte range probe and retain no response body.
    Probe {
        /// Reviewed NCTForge data-acquisition profile.
        #[arg(long)]
        profile: PathBuf,
    },
    /// Download or resume an artifact and emit a content-addressed receipt.
    Acquire {
        /// Reviewed NCTForge data-acquisition profile.
        #[arg(long)]
        profile: PathBuf,
        /// Existing directory for the artifact, partial file, and receipt.
        #[arg(long)]
        output_directory: PathBuf,
        /// Exact byte count reported by `data probe`; required as a size guard.
        #[arg(long)]
        confirm_size_bytes: u64,
    },
    /// Verify a case-scoped evaluated-neutron selection and every extracted file.
    VerifySelection {
        /// Case-scoped evaluated-neutron source-selection manifest.
        #[arg(long)]
        selection: PathBuf,
        /// Exact material JSON bound by the selection.
        #[arg(long)]
        material: PathBuf,
        /// Reviewed acquisition profile bound by the receipt; repeat once per
        /// bound acquisition, paired positionally with --receipt.
        #[arg(long)]
        profile: Vec<PathBuf>,
        /// Acquisition receipt checked into the case provenance; repeat once
        /// per bound acquisition, paired positionally with --profile.
        #[arg(long)]
        receipt: Vec<PathBuf>,
        /// Directory containing exactly the selected extracted ENDF files.
        #[arg(long)]
        evaluations_directory: PathBuf,
    },
    /// Verify a processed-data manifest, selected files, and optional material capabilities.
    VerifyManifest {
        /// Case-scoped OpenMC nuclear-data manifest generated by the inspector.
        #[arg(long)]
        manifest: PathBuf,
        /// Root containing cross_sections.xml and every selected HDF5 file.
        #[arg(long)]
        data_root: PathBuf,
        /// Optional material definition whose required capabilities must pass.
        #[arg(long)]
        material: Option<PathBuf>,
    },
    /// Derive the common neutron transport interval for an exact material.
    DeriveTransportDomain {
        /// Case-scoped OpenMC nuclear-data capability manifest.
        #[arg(long)]
        manifest: PathBuf,
        /// Exact material definition selecting nuclides and temperature.
        #[arg(long)]
        material: PathBuf,
        /// New content-bound transport-domain JSON path.
        #[arg(long)]
        output: PathBuf,
    },
    /// Regenerate and verify a neutron transport-domain document.
    VerifyTransportDomain {
        /// Case-scoped OpenMC nuclear-data capability manifest.
        #[arg(long)]
        manifest: PathBuf,
        /// Exact material definition selecting nuclides and temperature.
        #[arg(long)]
        material: PathBuf,
        /// Transport-domain document to validate and regenerate.
        #[arg(long)]
        transport_domain: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Pair each `--profile` with the `--receipt` at the same position; mixed
/// selections pass one pair per bound acquisition.
fn acquisition_pairs<'a>(
    profiles: &'a [PathBuf],
    receipts: &'a [PathBuf],
) -> Result<Vec<(&'a PathBuf, &'a PathBuf)>, Box<dyn Error>> {
    if profiles.is_empty() || profiles.len() != receipts.len() {
        return Err(
            io::Error::other("each --profile requires one --receipt, and vice versa").into(),
        );
    }
    Ok(profiles.iter().zip(receipts).collect())
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Some(Command::Backends) => {
            let backend = OpenMcBackend::default();
            let descriptor = backend.descriptor();
            println!(
                "{} ({}) prepare={} execute={} import={}",
                descriptor.display_name,
                descriptor.id,
                descriptor.can_prepare,
                descriptor.can_execute,
                descriptor.can_import
            );
        }
        Some(Command::Benchmark(args)) => match args.command {
            BenchmarkCommand::Generate { output } => {
                let generated = generate_nf_bnct_001(&output)?;
                println!("generated NF-BNCT-001 at {}", generated.root.display());
                println!("CT slices: {}", generated.ct_files.len());
                println!("RT Structure Set: {}", generated.rtstruct_file.display());
                println!("Case manifest: {}", generated.manifest_file.display());
            }
            BenchmarkCommand::Verify { input } => {
                let report = verify_nf_bnct_001(&input)?;
                println!(
                    "verified {}: shape={:?}, spacing_mm={:?}, CT slices={}",
                    report.case_id, report.shape, report.spacing_mm, report.ct_slice_count
                );
                println!(
                    "artifact integrity: {} files verified",
                    report.verified_artifact_count
                );
                for roi in report.rois {
                    println!(
                        "ROI {}: voxels={}, volume_cm3={}, centroid_lps_mm={:?}",
                        roi.name, roi.voxel_count, roi.volume_cm3, roi.centroid_lps_mm
                    );
                }
            }
            BenchmarkCommand::DeriveMaterials {
                case_root,
                case,
                base_material,
                map,
                masks,
                output_assignment,
                output_case,
            } => {
                let verified = load_nf_bnct_001(&case_root)?;
                let mut derived_case: TransportCase = serde_json::from_slice(&fs::read(&case)?)?;
                let base: MaterialDefinition = serde_json::from_slice(&fs::read(&base_material)?)?;
                if derived_case.geometry != verified.ct.geometry {
                    return Err(io::Error::other(
                        "transport-case geometry differs from the verified DICOM geometry",
                    )
                    .into());
                }
                derived_case.material = base;
                // External RegionMask files (e.g. from `nifti to-mask`)
                // supplement or replace RT Structure Set ROIs when given.
                let external_masks: std::collections::BTreeMap<String, nctforge_core::RegionMask> =
                    masks
                        .iter()
                        .map(|binding| {
                            let (name, path) = binding.split_once('=').ok_or_else(|| {
                                io::Error::other("--mask entries must be NAME=path")
                            })?;
                            let mask: nctforge_core::RegionMask =
                                serde_json::from_slice(&fs::read(path)?)?;
                            if mask.name != name {
                                return Err(io::Error::other(format!(
                                    "--mask {name}: mask file names itself {:?}",
                                    mask.name
                                )));
                            }
                            let expected_voxels = verified
                                .ct
                                .geometry
                                .voxel_count()
                                .map_err(|error| io::Error::other(error.to_string()))?;
                            if mask.voxels.len() != expected_voxels {
                                return Err(io::Error::other(format!(
                                    "--mask {name}: {} voxels, grid expects {}",
                                    mask.voxels.len(),
                                    expected_voxels,
                                )));
                            }
                            Ok((name.to_owned(), mask))
                        })
                        .collect::<Result<_, io::Error>>()?;
                let map_bytes = fs::read(&map)?;
                let mapping: serde_json::Value = serde_json::from_slice(&map_bytes)?;
                let regions = mapping
                    .get("regions")
                    .and_then(|value| value.as_object())
                    .ok_or_else(|| {
                        io::Error::other(
                            "material map must be {\"regions\": {\"ROI\": \"material.json\"}}",
                        )
                    })?;
                let map_dir = map.parent().unwrap_or(Path::new("."));
                let geometry = &verified.ct.geometry;
                let [nx, ny, _] = geometry.shape.map(|v| v as usize);

                let mut material_regions = Vec::with_capacity(regions.len());
                for (name, material_path) in regions {
                    let material_path = material_path.as_str().ok_or_else(|| {
                        io::Error::other(format!("region {name:?} must map to a file path"))
                    })?;
                    let material: MaterialDefinition =
                        serde_json::from_slice(&fs::read(map_dir.join(material_path))?).map_err(
                            |error| io::Error::other(format!("region {name:?} material: {error}")),
                        )?;
                    let mask_voxels: &[bool] = if let Some(mask) = external_masks.get(name.as_str())
                    {
                        &mask.voxels
                    } else if external_masks.is_empty() {
                        &verified
                            .structures
                            .roi(name)
                            .ok_or_else(|| io::Error::other(format!("no ROI named {name:?}")))?
                            .voxels
                    } else {
                        return Err(io::Error::other(format!("no --mask named {name:?}")).into());
                    };

                    // Rasterize the mask to voxel indices; when it fills its
                    // own bounding box exactly it becomes a CSG box region,
                    // otherwise an exact voxel-set region realized as
                    // per-voxel lattice elements.
                    let mut lower = [u32::MAX; 3];
                    let mut upper = [0_u32; 3];
                    let mut indices = Vec::new();
                    for (index, included) in mask_voxels.iter().copied().enumerate() {
                        if !included {
                            continue;
                        }
                        let k = index / (nx * ny);
                        let j = (index % (nx * ny)) / nx;
                        let i = index % nx;
                        indices.push([i as u32, j as u32, k as u32]);
                        for (axis, value) in [i, j, k].iter().enumerate() {
                            lower[axis] = lower[axis].min(*value as u32);
                            upper[axis] = upper[axis].max(*value as u32);
                        }
                    }
                    if indices.is_empty() {
                        return Err(io::Error::other(format!("ROI {name:?} is empty")).into());
                    }
                    let box_voxels: u64 = (0..3)
                        .map(|axis| u64::from(upper[axis] - lower[axis] + 1))
                        .product();
                    let shape = if box_voxels == indices.len() as u64 {
                        nctforge_transport::MaterialRegionShape::VoxelBox { lower, upper }
                    } else {
                        nctforge_transport::MaterialRegionShape::VoxelSet { indices }
                    };
                    material_regions.push(MaterialRegion {
                        name: name.clone(),
                        material,
                        shape,
                    });
                }

                let case_sha = nctforge_evidence::sha256_file(&case_root.join("case.json"))?;
                let assignment = MaterialAssignment {
                    schema_version: MATERIAL_ASSIGNMENT_SCHEMA.into(),
                    // The assignment binds the transport case it is validated
                    // against; the DICOM case is bound through provenance_id.
                    case_id: derived_case.case_id.clone(),
                    base_material: derived_case.material.clone(),
                    regions: material_regions,
                    provenance_id: format!("case:sha256:{case_sha}"),
                };
                assignment.validate(geometry).map_err(|error| {
                    io::Error::other(format!("derived assignment is invalid: {error}"))
                })?;
                derived_case.validate().map_err(|error| {
                    io::Error::other(format!("derived transport case is invalid: {error}"))
                })?;
                for (path, document) in [
                    (&output_assignment, serde_json::to_value(&assignment)?),
                    (&output_case, serde_json::to_value(&derived_case)?),
                ] {
                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)?;
                    serde_json::to_writer_pretty(&mut file, &document)?;
                    file.write_all(b"\n")?;
                }
                println!(
                    "derived {} material region(s) for {}",
                    assignment.regions.len(),
                    assignment.case_id
                );
                for region in &assignment.regions {
                    println!(
                        "region {}: {} voxel(s) ({}) -> {}",
                        region.name,
                        region.voxel_count(),
                        match &region.shape {
                            nctforge_transport::MaterialRegionShape::VoxelBox { lower, upper } =>
                                format!("box {lower:?}..{upper:?}"),
                            nctforge_transport::MaterialRegionShape::VoxelSet { .. } =>
                                "voxel set".to_owned(),
                        },
                        region.material.id
                    );
                }
                println!("assignment: {}", output_assignment.display());
                println!("derived case: {}", output_case.display());
            }
        },
        Some(Command::Openmc(args)) => match args.command {
            OpenMcCommand::Data(args) => match args.command {
                OpenMcDataCommand::Probe { profile } => {
                    let document = DataAcquisitionProfileDocument::from_path(&profile)?;
                    let result = DataAcquisitionClient::new()?.probe(&document)?;
                    println!("profile: {}", result.profile_id);
                    println!(
                        "artifact: {} ({} bytes; {:.2} GiB)",
                        result.expected_filename,
                        result.size_bytes,
                        bytes_to_gib(result.size_bytes)
                    );
                    println!("range resume: {}", result.accepts_ranges);
                    println!("final HTTPS origin: {}", result.final_origin);
                    if document.profile.artifact.publisher_digest.is_none() {
                        println!("publisher digest: unavailable; acquisition remains unqualified");
                    } else {
                        println!("publisher digest: pinned in profile and checked on acquisition");
                    }
                    println!(
                        "acquisition requires --confirm-size-bytes {}",
                        result.size_bytes
                    );
                }
                OpenMcDataCommand::Acquire {
                    profile,
                    output_directory,
                    confirm_size_bytes,
                } => {
                    let document = DataAcquisitionProfileDocument::from_path(&profile)?;
                    let client = DataAcquisitionClient::new()?;
                    let total = document.profile.artifact.expected_size_bytes;
                    let mut next_report = 0_u64;
                    let acquired = client.acquire_with_progress(
                        &document,
                        &output_directory,
                        confirm_size_bytes,
                        |progress| {
                            if progress.completed_bytes >= next_report
                                || progress.completed_bytes == progress.total_bytes
                            {
                                eprintln!(
                                    "acquired {} / {} bytes ({:.1}%)",
                                    progress.completed_bytes,
                                    progress.total_bytes,
                                    100.0 * progress.completed_bytes as f64
                                        / progress.total_bytes as f64
                                );
                                next_report = progress
                                    .completed_bytes
                                    .saturating_add((total / 100).max(64 * 1024 * 1024));
                            }
                        },
                    )?;
                    println!("artifact: {}", acquired.artifact_path.display());
                    println!("SHA-256: {}", acquired.receipt.artifact.sha256);
                    println!("receipt: {}", acquired.receipt_path.display());
                    println!("evidence state: acquisition_only");
                }
                OpenMcDataCommand::VerifySelection {
                    selection,
                    material,
                    profile,
                    receipt,
                    evaluations_directory,
                } => {
                    let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                    let material_bytes = fs::read(&material)?;
                    let material: MaterialDefinition = serde_json::from_slice(&material_bytes)?;
                    let pairs = acquisition_pairs(&profile, &receipt)?;
                    let pairs = pairs
                        .iter()
                        .map(|(profile, receipt)| {
                            Ok::<_, Box<dyn Error>>((
                                DataAcquisitionProfileDocument::from_path(profile)?,
                                DataAcquisitionReceiptDocument::from_path(receipt)?,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let pair_refs = pairs
                        .iter()
                        .map(|(profile, receipt)| (profile, receipt))
                        .collect::<Vec<_>>();

                    selection
                        .selection
                        .validate_for_material(&material, &material_bytes)?;
                    selection.selection.validate_acquisitions(&pair_refs)?;
                    selection.selection.verify_files(&evaluations_directory)?;

                    println!("selection: {}", selection.selection.id);
                    println!("selection SHA-256: {}", selection.sha256);
                    println!(
                        "verified evaluations: {}",
                        selection.selection.evaluations.len()
                    );
                    for acquisition in selection.selection.declared_acquisitions() {
                        println!("archive SHA-256: {}", acquisition.archive_sha256);
                    }
                    println!(
                        "qualification: {}",
                        match selection.selection.qualification {
                            EvaluatedSourceQualification::CandidateArchiveEquivalenceUnresolved =>
                                "candidate_archive_equivalence_unresolved",
                            EvaluatedSourceQualification::ResponseTreatmentCandidateUnreviewed =>
                                "response_treatment_candidate_unreviewed",
                        }
                    );
                }
                OpenMcDataCommand::VerifyManifest {
                    manifest,
                    data_root,
                    material,
                } => {
                    let manifest_json = fs::read(&manifest)?;
                    let manifest: NuclearDataManifest = serde_json::from_slice(&manifest_json)?;
                    manifest.verify_files(&data_root)?;

                    println!("manifest: {}", manifest.id);
                    println!(
                        "verified processed-data artifacts: {}",
                        1 + manifest.neutron_tables.len() + manifest.photon_tables.len()
                    );
                    println!("archive SHA-256: {}", manifest.distribution.archive_sha256);
                    println!("qualification: processed_data_identity_verified");

                    if let Some(material) = material {
                        let material_json = fs::read(material)?;
                        let material: MaterialDefinition = serde_json::from_slice(&material_json)?;
                        manifest.validate_for_material(&material)?;
                        println!("material capabilities: verified");
                    }
                }
                OpenMcDataCommand::DeriveTransportDomain {
                    manifest,
                    material,
                    output,
                } => {
                    let manifest_bytes = fs::read(manifest)?;
                    let material_bytes = fs::read(material)?;
                    let domain =
                        OpenMcNeutronTransportDomain::derive(&manifest_bytes, &material_bytes)?;
                    let result = domain.write_new(&output)?;
                    println!("derived OpenMC neutron transport domain");
                    println!("domain: {}", result.domain_path.display());
                    println!("domain SHA-256: {}", result.domain_sha256);
                    println!(
                        "closed diagnostic interval: [{}, {}] eV",
                        result.domain.energy_range_ev[0], result.domain.energy_range_ev[1]
                    );
                    println!("qualification: backend_capability_derived_unreviewed");
                }
                OpenMcDataCommand::VerifyTransportDomain {
                    manifest,
                    material,
                    transport_domain,
                } => {
                    let manifest_bytes = fs::read(manifest)?;
                    let material_bytes = fs::read(material)?;
                    let domain =
                        OpenMcNeutronTransportDomainDocument::from_path(&transport_domain)?;
                    domain.verify_against_inputs(&manifest_bytes, &material_bytes)?;
                    println!(
                        "verified OpenMC neutron transport domain {}",
                        transport_domain.display()
                    );
                    println!("domain SHA-256: {}", domain.sha256);
                    println!(
                        "closed diagnostic interval: [{}, {}] eV",
                        domain.domain.energy_range_ev[0], domain.domain.energy_range_ev[1]
                    );
                    println!("qualification: backend_capability_derived_unreviewed");
                }
            },
            OpenMcCommand::Generate {
                case,
                component_profile,
                material,
                source,
                response_set,
                nuclear_data_manifest,
                execution_profile,
                nuclear_data_root,
                acceptance,
                assignment,
                output,
            } => {
                let case_json = fs::read(&case)?;
                let case: TransportCase = serde_json::from_slice(&case_json)?;
                let component_profile_json = fs::read(&component_profile)?;
                let material_json = fs::read(&material)?;
                let source_json = fs::read(&source)?;
                let response_set_json = fs::read(&response_set)?;
                let nuclear_data_manifest_json = fs::read(&nuclear_data_manifest)?;
                let execution_profile_json = fs::read(&execution_profile)?;
                let acceptance_json = acceptance.as_ref().map(fs::read).transpose()?;
                let assignment_json = assignment.as_ref().map(fs::read).transpose()?;
                let deck = OpenMcInputDeck::generate(
                    &case,
                    &nuclear_data_root,
                    OpenMcInputArtifacts {
                        component_profile_json: &component_profile_json,
                        material_json: &material_json,
                        source_json: &source_json,
                        response_set_json: &response_set_json,
                        nuclear_data_manifest_json: &nuclear_data_manifest_json,
                        execution_profile_json: &execution_profile_json,
                        acceptance_json: acceptance_json.as_deref(),
                        material_assignment_json: assignment_json.as_deref(),
                    },
                )?;
                deck.write_new(&output)?;
                println!(
                    "generated deterministic OpenMC input deck at {}",
                    output.display()
                );
                println!("case: {}", deck.manifest.case_id);
                println!("xml artifacts: {}", deck.manifest.xml_artifacts.len());
                println!("tallies: {}", deck.manifest.tallies.len());
                println!(
                    "particles per batch: {}",
                    deck.manifest.execution.particles_per_batch
                );
            }
            OpenMcCommand::Run {
                case,
                component_profile,
                material,
                source,
                response_set,
                nuclear_data_manifest,
                execution_profile,
                acceptance,
                assignment,
                nuclear_data_root,
                openmc,
                environment,
                working_directory,
                dose_output,
                evidence_root,
            } => {
                let case_json = fs::read(&case)?;
                let case_document: TransportCase = serde_json::from_slice(&case_json)?;
                let config = nctforge_openmc::OpenMcBackendConfig {
                    component_profile,
                    material,
                    source,
                    response_set,
                    nuclear_data_manifest,
                    execution_profile,
                    acceptance,
                    material_assignment: assignment,
                    nuclear_data_root,
                };
                let mut backend = OpenMcBackend::new(&openmc).configured(config);
                for pair in &environment {
                    let (key, value) = pair.split_once('=').ok_or_else(|| {
                        io::Error::other(format!(
                            "environment overlay {pair:?} must be written as KEY=VALUE"
                        ))
                    })?;
                    backend = backend.with_env(key, value);
                }
                let prepared = backend.prepare(&case_document, &working_directory)?;
                println!("prepared deck in {}", prepared.working_directory);
                let completed = backend.execute(&prepared)?;
                if completed.exit_code != 0 {
                    return Err(io::Error::other(format!(
                        "openmc exited with code {}",
                        completed.exit_code
                    ))
                    .into());
                }
                println!("execution finished with exit code 0");
                let bundle = backend.collect(&completed)?;
                let json = serde_json::to_vec_pretty(&bundle)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&dose_output)?;
                file.write_all(&json)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                println!(
                    "collected physical dose bundle at {}",
                    dose_output.display()
                );
                println!("provenance: {}", bundle.provenance_id);

                if let Some(evidence_root) = evidence_root {
                    let config = backend
                        .config()
                        .expect("backend was configured above")
                        .clone();
                    let working = working_directory.clone();
                    let mut artifacts: Vec<(&str, PathBuf, String)> = vec![
                        ("case", case.clone(), "case.json".into()),
                        (
                            "component_profile",
                            config.component_profile.clone(),
                            "component-profile.json".into(),
                        ),
                        ("material", config.material.clone(), "material.json".into()),
                        ("source", config.source.clone(), "source.json".into()),
                        (
                            "response_set",
                            config.response_set.clone(),
                            "response-tables/response-set.json".into(),
                        ),
                        (
                            "nuclear_data_manifest",
                            config.nuclear_data_manifest.clone(),
                            "nuclear-data-manifest.json".into(),
                        ),
                        (
                            "execution_profile",
                            config.execution_profile.clone(),
                            "run-settings.json".into(),
                        ),
                        (
                            "input_manifest",
                            working.join(nctforge_openmc::OPENMC_INPUT_MANIFEST_FILE),
                            "inputs/nctforge-input-manifest.json".into(),
                        ),
                        (
                            "run_receipt",
                            working.join(nctforge_openmc::OPENMC_RUN_RECEIPT_FILE),
                            "nctforge-openmc-run-receipt.json".into(),
                        ),
                        (
                            "dose_bundle",
                            dose_output.clone(),
                            "normalized-dose.json".into(),
                        ),
                    ];
                    if let Some(acceptance) = &config.acceptance {
                        artifacts.push((
                            "acceptance_contract",
                            acceptance.clone(),
                            "nctforge-acceptance-contract.json".into(),
                        ));
                    }
                    for xml in [
                        "settings.xml",
                        "materials.xml",
                        "geometry.xml",
                        "tallies.xml",
                    ] {
                        artifacts.push(("deck", working.join(xml), xml.into()));
                    }
                    for entry in fs::read_dir(&working)? {
                        let path = entry?.path();
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default()
                            .to_string();
                        if name.ends_with(".log") {
                            artifacts.push(("log", path, format!("logs/{name}")));
                        } else if name.starts_with("statepoint.") && name.ends_with(".h5") {
                            artifacts.push((
                                "statepoint",
                                path,
                                format!("statepoints-or-native-results/{name}"),
                            ));
                        }
                    }
                    let inputs: Vec<nctforge_evidence::BundleInput> = artifacts
                        .into_iter()
                        .map(|(role, source, dest)| nctforge_evidence::BundleInput {
                            role: role.into(),
                            source,
                            relative_path: dest,
                            media_type: None,
                        })
                        .collect();
                    let manifest = nctforge_evidence::export_evidence_bundle(
                        &evidence_root,
                        &bundle.case_id,
                        nctforge_evidence::QualificationBoundary::SyntheticResearchOnly,
                        &inputs,
                    )?;
                    println!("evidence bundle exported at {}", evidence_root.display());
                    println!("artifacts: {}", manifest.artifacts.len());
                }
            }
            OpenMcCommand::Collect {
                working_directory,
                exit_code,
                output,
            } => {
                let completed = CompletedRun {
                    backend_id: "openmc".into(),
                    case_id: String::new(),
                    working_directory: working_directory.display().to_string(),
                    exit_code,
                };
                let bundle = OpenMcBackend::default().collect(&completed)?;
                let json = serde_json::to_vec_pretty(&bundle)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output)?;
                file.write_all(&json)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                println!("collected physical dose bundle at {}", output.display());
                println!("case: {}", bundle.case_id);
                println!("components: {}", bundle.components.len());
                println!("provenance: {}", bundle.provenance_id);
            }
            OpenMcCommand::Evaluate {
                runs,
                exit_codes,
                output,
            } => {
                let exit_codes = if exit_codes.is_empty() {
                    vec![0; runs.len()]
                } else {
                    exit_codes
                };
                let run_refs: Vec<&Path> = runs.iter().map(PathBuf::as_path).collect();
                let report = evaluate_runs(&run_refs, &exit_codes)?;
                let json = serde_json::to_vec_pretty(&report)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output)?;
                file.write_all(&json)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                println!("acceptance report at {}", output.display());
                println!("case: {}", report.case_id);
                println!("runs evaluated: {}", report.runs.len());
                println!(
                    "estimator comparisons: {} ({} failed)",
                    report.estimator_comparisons.len(),
                    report
                        .estimator_comparisons
                        .iter()
                        .filter(|c| !c.passed)
                        .count()
                );
                println!("gates passed: {}", report.gates_passed);
            }
        },
        Some(Command::Njoy(args)) => match args.command {
            NjoyCommand::Prepare {
                selection,
                material,
                generation_method,
                profile,
                receipt,
                evaluations_directory,
                output,
            } => {
                let selection_json = fs::read(selection)?;
                let material_json = fs::read(material)?;
                let generation_method_json = fs::read(generation_method)?;
                let acquisition_bytes = acquisition_pairs(&profile, &receipt)?
                    .iter()
                    .map(|(profile, receipt)| {
                        Ok::<_, io::Error>((fs::read(profile)?, fs::read(receipt)?))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let acquisitions = acquisition_bytes
                    .iter()
                    .map(|(profile, receipt)| NjoyAcquisitionArtifacts {
                        profile_json: profile,
                        receipt_json: receipt,
                    })
                    .collect();
                let bundle = NjoyInputBundle::generate(
                    &evaluations_directory,
                    NjoyInputArtifacts {
                        evaluated_source_selection_json: &selection_json,
                        material_json: &material_json,
                        generation_method_json: &generation_method_json,
                        acquisitions,
                    },
                )?;
                bundle.write_new(&output)?;
                println!("prepared NJOY2016.78 inputs at {}", output.display());
                println!("nuclide runs: {}", bundle.manifest.runs.len());
                println!(
                    "source selection SHA-256: {}",
                    bundle.manifest.bindings.evaluated_source_selection.sha256
                );
                println!("qualification: input_preparation_only");
            }
            NjoyCommand::InventoryPhotonData {
                selection,
                evaluations_directory,
                output,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventory::inspect(&selection, &evaluations_directory)?;
                let result = inventory.write_new(&output)?;
                println!("inventoried exact ENDF photon-production records");
                println!("inventory: {}", result.inventory_path.display());
                println!("inventory SHA-256: {}", result.inventory_sha256);
                println!("evaluations: {}", result.inventory.evaluations.len());
                println!(
                    "MF=6/12/13/14/15 sections: {}",
                    result.inventory.section_count
                );
                println!(
                    "evaluations with a HEATR photon source: {}",
                    result.inventory.evaluations_with_heatr_photon_source_count
                );
                println!("format findings: {}", result.inventory.format_finding_count);
                println!("qualification: source_inventory_unreviewed");
            }
            NjoyCommand::VerifyPhotonInventory {
                selection,
                evaluations_directory,
                inventory,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let document = EndfPhotonProductionInventoryDocument::from_path(&inventory)?;
                document.verify_against_selection(&selection, &evaluations_directory)?;
                println!(
                    "verified ENDF photon-production inventory {}",
                    inventory.display()
                );
                println!("inventory SHA-256: {}", document.sha256);
                println!("evaluations: {}", document.inventory.evaluations.len());
                println!(
                    "format findings: {}",
                    document.inventory.format_finding_count
                );
                println!("qualification: source_inventory_unreviewed");
            }
            NjoyCommand::CalculatePhotonMoments {
                selection,
                evaluations_directory,
                photon_inventory,
                normalization_tolerance,
                output,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = EndfContinuumPhotonMomentReport::calculate(
                    &selection,
                    &evaluations_directory,
                    &inventory,
                    normalization_tolerance,
                )?;
                let result = report.write_new(&output)?;
                println!("calculated independent ENDF continuum photon moments");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!("reactions: {}", result.report.reaction_count);
                println!("incident-energy samples: {}", result.report.sample_count);
                println!(
                    "maximum absolute normalization error: {:.12e}",
                    result.report.maximum_absolute_normalization_error
                );
                if result.report.failed_sample_count == 0 {
                    println!("qualification: source_moments_checked_unreviewed");
                } else {
                    println!("qualification: spectrum_normalization_rejected");
                    return Err(io::Error::other(format!(
                        "{} continuum spectrum sample(s) failed normalization; report was preserved",
                        result.report.failed_sample_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyPhotonMoments {
                selection,
                evaluations_directory,
                photon_inventory,
                moment_report,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = EndfContinuumPhotonMomentReportDocument::from_path(&moment_report)?;
                report.verify_against_sources(&selection, &evaluations_directory, &inventory)?;
                println!(
                    "verified continuum photon moments {}",
                    moment_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!("reactions: {}", report.report.reaction_count);
                println!("incident-energy samples: {}", report.report.sample_count);
                println!(
                    "failed normalization samples: {}",
                    report.report.failed_sample_count
                );
                println!(
                    "qualification: {}",
                    if report.report.failed_sample_count == 0 {
                        "source_moments_checked_unreviewed"
                    } else {
                        "spectrum_normalization_rejected"
                    }
                );
            }
            NjoyCommand::ComparePhotonMoments {
                moment_report,
                receipt,
                execution_directory,
                relative_tolerance,
                output,
            } => {
                let moments = EndfContinuumPhotonMomentReportDocument::from_path(&moment_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison = NjoyPhotonMomentComparison::compare(
                    &moments,
                    &execution,
                    &execution_directory,
                    relative_tolerance,
                )?;
                let result = comparison.write_new(&output)?;
                println!("compared independent photon moments with NJOY diagnostics");
                println!("comparison: {}", result.comparison_path.display());
                println!("comparison SHA-256: {}", result.comparison_sha256);
                println!(
                    "compared diagnostic samples: {}",
                    result.comparison.compared_sample_count
                );
                println!(
                    "uncompared independent samples: {}",
                    result.comparison.uncompared_independent_sample_count
                );
                println!(
                    "skipped processor-only samples: {}",
                    result.comparison.skipped_interpolated_sample_count
                );
                println!(
                    "maximum relative difference: {:.12e}",
                    result.comparison.maximum_relative_difference
                );
                if result.comparison.failed_sample_count == 0 {
                    println!("qualification: independent_moments_match_processor_print_unreviewed");
                } else {
                    println!("qualification: processor_print_mismatch_rejected");
                    return Err(io::Error::other(format!(
                        "{} photon-moment sample(s) disagree with NJOY diagnostics; comparison was preserved",
                        result.comparison.failed_sample_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyPhotonMomentComparison {
                moment_report,
                receipt,
                execution_directory,
                comparison_report,
            } => {
                let moments = EndfContinuumPhotonMomentReportDocument::from_path(&moment_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison = NjoyPhotonMomentComparisonDocument::from_path(&comparison_report)?;
                comparison.verify_against_evidence(&moments, &execution, &execution_directory)?;
                println!(
                    "verified NJOY photon-moment comparison {}",
                    comparison_report.display()
                );
                println!("comparison SHA-256: {}", comparison.sha256);
                println!(
                    "compared diagnostic samples: {}",
                    comparison.comparison.compared_sample_count
                );
                println!(
                    "uncompared independent samples: {}",
                    comparison.comparison.uncompared_independent_sample_count
                );
                println!(
                    "skipped processor-only samples: {}",
                    comparison.comparison.skipped_interpolated_sample_count
                );
                println!(
                    "failed samples: {}",
                    comparison.comparison.failed_sample_count
                );
                println!(
                    "qualification: {}",
                    if comparison.comparison.failed_sample_count == 0 {
                        "independent_moments_match_processor_print_unreviewed"
                    } else {
                        "processor_print_mismatch_rejected"
                    }
                );
            }
            NjoyCommand::CalculateCapturePhotonBalance {
                selection,
                evaluations_directory,
                photon_inventory,
                nuclide,
                normalization_tolerance,
                relative_energy_tolerance,
                output,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = EndfMf6CapturePhotonBalanceReport::calculate(
                    &selection,
                    &evaluations_directory,
                    &inventory,
                    &nuclide,
                    normalization_tolerance,
                    relative_energy_tolerance,
                )?;
                let result = report.write_new(&output)?;
                println!("calculated independent MF=6 capture photon balance");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!("nuclide: {}", result.report.nuclide);
                println!("incident-energy samples: {}", result.report.sample_count);
                println!(
                    "failed normalization samples: {}",
                    result.report.failed_normalization_sample_count
                );
                println!(
                    "failed energy-balance samples: {}",
                    result.report.failed_energy_balance_sample_count
                );
                println!(
                    "maximum absolute relative energy residual: {:.12e}",
                    result.report.maximum_absolute_relative_energy_residual
                );
                println!(
                    "qualification: {}",
                    qualification_name(result.report.qualification)
                );
                if result.report.failed_normalization_sample_count > 0
                    || result.report.failed_energy_balance_sample_count > 0
                    || result.report.sample_count == 0
                {
                    return Err(io::Error::other(format!(
                        "{} MF=6 capture photon source did not pass the independent screening gate; report was preserved",
                        result.report.nuclide
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyCapturePhotonBalance {
                selection,
                evaluations_directory,
                photon_inventory,
                balance_report,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = EndfMf6CapturePhotonBalanceReportDocument::from_path(&balance_report)?;
                report.verify_against_sources(&selection, &evaluations_directory, &inventory)?;
                println!(
                    "verified MF=6 capture photon balance {}",
                    balance_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!("nuclide: {}", report.report.nuclide);
                println!("incident-energy samples: {}", report.report.sample_count);
                println!(
                    "failed normalization samples: {}",
                    report.report.failed_normalization_sample_count
                );
                println!(
                    "failed energy-balance samples: {}",
                    report.report.failed_energy_balance_sample_count
                );
                println!(
                    "qualification: {}",
                    qualification_name(report.report.qualification)
                );
            }
            NjoyCommand::CalculateLaw7ImplicitResidual {
                selection,
                evaluations_directory,
                photon_inventory,
                nuclide,
                normalization_tolerance,
                relative_energy_tolerance,
                output,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = EndfMf6Law7ImplicitResidualReport::calculate(
                    &selection,
                    &evaluations_directory,
                    &inventory,
                    &nuclide,
                    normalization_tolerance,
                    relative_energy_tolerance,
                )?;
                let result = report.write_new(&output)?;
                println!("calculated deuterium MF=6/MT=16 LAW=7 implicit residual");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "source incident-energy nodes: {}",
                    result.report.source_incident_node_count
                );
                println!("active samples: {}", result.report.sample_count);
                println!(
                    "failed normalization samples: {}",
                    result.report.failed_normalization_sample_count
                );
                println!(
                    "failed residual-energy samples: {}",
                    result.report.failed_residual_energy_sample_count
                );
                println!(
                    "maximum absolute normalization error: {:.12e}",
                    result.report.maximum_absolute_normalization_error
                );
                println!(
                    "minimum implicit residual energy: {:.12e} eV",
                    result
                        .report
                        .samples
                        .iter()
                        .map(|sample| sample.implicit_residual_energy_ev)
                        .fold(f64::INFINITY, f64::min)
                );
                println!(
                    "qualification: {}",
                    law7_qualification_name(result.report.qualification)
                );
                if result.report.failed_normalization_sample_count > 0
                    || result.report.failed_residual_energy_sample_count > 0
                    || result.report.sample_count == 0
                {
                    return Err(io::Error::other(
                        "deuterium LAW=7 source did not pass the independent screening gate; report was preserved",
                    )
                    .into());
                }
            }
            NjoyCommand::VerifyLaw7ImplicitResidual {
                selection,
                evaluations_directory,
                photon_inventory,
                residual_report,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&residual_report)?;
                report.verify_against_sources(&selection, &evaluations_directory, &inventory)?;
                println!(
                    "verified deuterium LAW=7 implicit-residual report {}",
                    residual_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!("active samples: {}", report.report.sample_count);
                println!(
                    "failed normalization samples: {}",
                    report.report.failed_normalization_sample_count
                );
                println!(
                    "failed residual-energy samples: {}",
                    report.report.failed_residual_energy_sample_count
                );
                println!(
                    "qualification: {}",
                    law7_qualification_name(report.report.qualification)
                );
            }
            NjoyCommand::CompareLaw7ImplicitResidual {
                residual_report,
                receipt,
                execution_directory,
                source_relative_tolerance,
                print_relative_tolerance,
                output,
            } => {
                let residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&residual_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison = NjoyLaw7ImplicitResidualComparison::compare(
                    &residual,
                    &execution,
                    &execution_directory,
                    source_relative_tolerance,
                    print_relative_tolerance,
                )?;
                let result = comparison.write_new(&output)?;
                println!("attributed deuterium LAW=7 processor diagnostics");
                println!("comparison: {}", result.comparison_path.display());
                println!("comparison SHA-256: {}", result.comparison_sha256);
                println!(
                    "shared source samples: {}",
                    result.comparison.shared_sample_count
                );
                println!(
                    "receipt violations attributed: {}/{}",
                    result.comparison.attributed_violation_count,
                    result.comparison.receipt_violation_count
                );
                println!("failed samples: {}", result.comparison.failed_sample_count);
                println!(
                    "maximum source/processor neutron-mean difference: {:.12e}",
                    result
                        .comparison
                        .maximum_source_neutron_mean_relative_difference
                );
                println!(
                    "maximum violation remainder/excess difference: {:.12e}",
                    result
                        .comparison
                        .maximum_violation_excess_relative_difference
                );
                println!(
                    "qualification: {}",
                    law7_comparison_qualification_name(result.comparison.qualification)
                );
                if result.comparison.qualification
                    != NjoyLaw7ImplicitResidualComparisonQualification::
                        ProcessorApproximationFullyAttributedUnreviewed
                {
                    return Err(io::Error::other(
                        "H-2 LAW=7 processor attribution did not pass; comparison was preserved",
                    )
                    .into());
                }
            }
            NjoyCommand::VerifyLaw7ImplicitResidualComparison {
                residual_report,
                receipt,
                execution_directory,
                comparison_report,
            } => {
                let residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&residual_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison =
                    NjoyLaw7ImplicitResidualComparisonDocument::from_path(&comparison_report)?;
                comparison.verify_against_evidence(&residual, &execution, &execution_directory)?;
                println!(
                    "verified H-2 LAW=7 processor attribution {}",
                    comparison_report.display()
                );
                println!("comparison SHA-256: {}", comparison.sha256);
                println!(
                    "receipt violations attributed: {}/{}",
                    comparison.comparison.attributed_violation_count,
                    comparison.comparison.receipt_violation_count
                );
                println!(
                    "failed samples: {}",
                    comparison.comparison.failed_sample_count
                );
                println!(
                    "qualification: {}",
                    law7_comparison_qualification_name(comparison.comparison.qualification)
                );
            }
            NjoyCommand::AttributeEnergyBalance {
                domain_aware_report,
                receipt,
                execution_directory,
                nuclide,
                print_relative_tolerance,
                output,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let attribution = NjoyEnergyBalanceAttribution::attribute(
                    &domain,
                    &execution,
                    &execution_directory,
                    &nuclide,
                    print_relative_tolerance,
                )?;
                let result = attribution.write_new(&output)?;
                println!("attributed {nuclide} NJOY processor energy-balance accounting");
                println!("attribution: {}", result.attribution_path.display());
                println!("attribution SHA-256: {}", result.attribution_sha256);
                println!(
                    "in-domain findings attributed: {}/{}",
                    result.attribution.attributed_in_domain_violation_count,
                    result.attribution.in_domain_violation_count
                );
                println!(
                    "physical validations still required: {}",
                    result.attribution.physical_validation_required_count
                );
                println!(
                    "waived findings: {}",
                    result.attribution.waived_violation_count
                );
                println!(
                    "maximum printed-remainder/final-excess difference: {:.12e}",
                    result
                        .attribution
                        .maximum_remainder_excess_relative_difference
                );
                println!(
                    "qualification: {}",
                    energy_balance_attribution_qualification_name(result.attribution.qualification)
                );
                if result.attribution.qualification
                    != NjoyEnergyBalanceAttributionQualification::
                        ProcessorAccountingMechanismAttributedPhysicalValidationRequired
                {
                    return Err(io::Error::other(
                        "NJOY energy-balance accounting was not fully attributed; report was preserved",
                    )
                    .into());
                }
            }
            NjoyCommand::VerifyEnergyBalanceAttribution {
                domain_aware_report,
                receipt,
                execution_directory,
                attribution_report,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let attribution =
                    NjoyEnergyBalanceAttributionDocument::from_path(&attribution_report)?;
                attribution.verify_against_evidence(&domain, &execution, &execution_directory)?;
                println!(
                    "verified processor-only energy-balance attribution {}",
                    attribution_report.display()
                );
                println!("attribution SHA-256: {}", attribution.sha256);
                println!(
                    "in-domain findings attributed: {}/{}",
                    attribution.attribution.attributed_in_domain_violation_count,
                    attribution.attribution.in_domain_violation_count
                );
                println!(
                    "physical validations still required: {}",
                    attribution.attribution.physical_validation_required_count
                );
                println!(
                    "qualification: {}",
                    energy_balance_attribution_qualification_name(
                        attribution.attribution.qualification
                    )
                );
            }
            NjoyCommand::CalculateReactionEnergyBalance {
                selection,
                evaluations_directory,
                attribution_report,
                domain_aware_report,
                receipt,
                execution_directory,
                nuclide,
                relative_tolerance,
                output,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let attribution =
                    NjoyEnergyBalanceAttributionDocument::from_path(&attribution_report)?;
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let report = EndfReactionEnergyBalanceReport::calculate(
                    &selection,
                    &evaluations_directory,
                    &attribution,
                    &domain,
                    &execution,
                    &execution_directory,
                    &nuclide,
                    relative_tolerance,
                )?;
                let result = report.write_new(&output)?;
                println!("integrated {nuclide} File 6 reaction energy balances from source");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "samples computed: {}/{} ({} partially computable)",
                    result.report.computed_sample_count,
                    result.report.sample_count,
                    result.report.partially_computed_sample_count
                );
                println!(
                    "independent/printed remainders matched: {}/{}",
                    result.report.remainder_matched_sample_count,
                    result.report.computed_sample_count
                );
                println!(
                    "maximum remainder relative difference: {:.12e}",
                    result.report.maximum_remainder_relative_difference
                );
                println!(
                    "maximum product ebar relative difference: {:.12e} over {} comparisons",
                    result.report.maximum_ebar_relative_difference,
                    result.report.ebar_comparison_count
                );
                println!(
                    "qualification: {}",
                    reaction_balance_qualification_name(result.report.qualification)
                );
            }
            NjoyCommand::VerifyReactionEnergyBalance {
                selection,
                evaluations_directory,
                attribution_report,
                domain_aware_report,
                receipt,
                execution_directory,
                nuclide,
                balance_report,
            } => {
                let selection = EvaluatedNeutronSourceSelectionDocument::from_path(&selection)?;
                let attribution =
                    NjoyEnergyBalanceAttributionDocument::from_path(&attribution_report)?;
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let balance = EndfReactionEnergyBalanceDocument::from_path(&balance_report)?;
                balance.verify_against_sources(
                    &selection,
                    &evaluations_directory,
                    &attribution,
                    &domain,
                    &execution,
                    &execution_directory,
                    &nuclide,
                )?;
                println!(
                    "verified independent reaction energy-balance report {}",
                    balance_report.display()
                );
                println!("report SHA-256: {}", balance.sha256);
                println!(
                    "samples computed: {}/{}",
                    balance.report.computed_sample_count, balance.report.sample_count
                );
                println!(
                    "independent/printed remainders matched: {}/{}",
                    balance.report.remainder_matched_sample_count,
                    balance.report.computed_sample_count
                );
                println!(
                    "qualification: {}",
                    reaction_balance_qualification_name(balance.report.qualification)
                );
            }
            NjoyCommand::GenerateResponseTables {
                material,
                component_profile,
                generation_method,
                nuclear_data_manifest,
                transport_domain,
                selection,
                domain_aware_report,
                receipt,
                execution_directory,
                response_set_output,
                report_output,
            } => {
                let artifacts = ResponseTableArtifacts::load(
                    &material,
                    &component_profile,
                    &generation_method,
                    &nuclear_data_manifest,
                    &transport_domain,
                    &selection,
                    &domain_aware_report,
                    &receipt,
                    execution_directory,
                )?;
                let inputs = artifacts.inputs();
                let case_id = &artifacts.execution.receipt.case_id;
                let generation = NjoyResponseTableGeneration::generate(
                    &inputs,
                    &format!("nctforge.{case_id}.neutron-response-set.v1"),
                    &format!("nctforge.{case_id}.response-table-generation.v1"),
                )?;
                let result = generation.write_new(&response_set_output, &report_output)?;
                println!("generated neutron response set from production-HEATR PENDF tables");
                println!("response set: {}", result.response_set_path.display());
                println!("response set SHA-256: {}", result.response_set_sha256);
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "union grid knots: {}",
                    result.generation.report.union_grid_knot_count
                );
                println!(
                    "carried in-domain kinematic violations: {}",
                    result
                        .generation
                        .report
                        .carried_findings
                        .in_domain_kinematic_violation_count
                );
                println!("qualification: tables_generated_unreviewed");
            }
            NjoyCommand::VerifyResponseTables {
                material,
                component_profile,
                generation_method,
                nuclear_data_manifest,
                transport_domain,
                selection,
                domain_aware_report,
                receipt,
                execution_directory,
                response_set,
                generation_report,
                review_output,
                reviewed_set_output,
            } => {
                let artifacts = ResponseTableArtifacts::load(
                    &material,
                    &component_profile,
                    &generation_method,
                    &nuclear_data_manifest,
                    &transport_domain,
                    &selection,
                    &domain_aware_report,
                    &receipt,
                    execution_directory,
                )?;
                let inputs = artifacts.inputs();
                let (set, set_bytes) = load_response_set(&response_set)?;
                let (report, report_bytes) = load_generation_report(&generation_report)?;
                let case_id = &artifacts.execution.receipt.case_id;
                let (review, reviewed_set) = NjoyResponseSetReviewReport::verify(
                    &inputs,
                    &set,
                    &set_bytes,
                    &report,
                    &report_bytes,
                    &format!("nctforge.{case_id}.response-set-review.v1"),
                )?;
                let result = NjoyResponseSetReviewDocument::write_new(
                    &review,
                    &reviewed_set,
                    &review_output,
                    &reviewed_set_output,
                )?;
                println!("verified response set by deterministic regeneration");
                println!("review: {}", result.review_path.display());
                println!("review SHA-256: {}", result.document.review_sha256);
                println!(
                    "reviewed response set: {}",
                    result.reviewed_set_path.display()
                );
                println!(
                    "reviewed set SHA-256: {}",
                    result.document.reviewed_set_sha256
                );
                println!(
                    "qualification: independently_reviewed (in-house deterministic verification)"
                );
            }
            NjoyCommand::CompareCapturePhotonMoments {
                balance_report,
                receipt,
                execution_directory,
                relative_tolerance,
                output,
            } => {
                let balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&balance_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison = NjoyCapturePhotonMomentComparison::compare(
                    &balance,
                    &execution,
                    &execution_directory,
                    relative_tolerance,
                )?;
                let result = comparison.write_new(&output)?;
                println!("compared independent capture moments with NJOY diagnostics");
                println!("comparison: {}", result.comparison_path.display());
                println!("comparison SHA-256: {}", result.comparison_sha256);
                println!(
                    "compared diagnostic samples: {}",
                    result.comparison.compared_sample_count
                );
                println!(
                    "uncompared independent samples: {}",
                    result.comparison.uncompared_independent_sample_count
                );
                println!(
                    "skipped processor-only samples: {}",
                    result.comparison.skipped_processor_sample_count
                );
                println!(
                    "maximum relative difference: {:.12e}",
                    result.comparison.maximum_relative_difference
                );
                if result.comparison.failed_sample_count == 0 {
                    println!(
                        "qualification: independent_capture_moments_match_processor_print_unreviewed"
                    );
                } else {
                    println!("qualification: processor_capture_print_mismatch_rejected");
                    return Err(io::Error::other(format!(
                        "{} capture-moment sample(s) disagree with NJOY diagnostics; comparison was preserved",
                        result.comparison.failed_sample_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyCapturePhotonMomentComparison {
                balance_report,
                receipt,
                execution_directory,
                comparison_report,
            } => {
                let balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&balance_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let comparison =
                    NjoyCapturePhotonMomentComparisonDocument::from_path(&comparison_report)?;
                comparison.verify_against_evidence(&balance, &execution, &execution_directory)?;
                println!(
                    "verified NJOY capture-moment comparison {}",
                    comparison_report.display()
                );
                println!("comparison SHA-256: {}", comparison.sha256);
                println!(
                    "compared diagnostic samples: {}",
                    comparison.comparison.compared_sample_count
                );
                println!(
                    "failed samples: {}",
                    comparison.comparison.failed_sample_count
                );
                println!(
                    "qualification: {}",
                    if comparison.comparison.failed_sample_count == 0 {
                        "independent_capture_moments_match_processor_print_unreviewed"
                    } else {
                        "processor_capture_print_mismatch_rejected"
                    }
                );
            }
            NjoyCommand::Execute {
                selection,
                material,
                generation_method,
                profile,
                receipt,
                evaluations_directory,
                input_bundle,
                njoy_executable,
                processor_support_artifacts,
                timeout_seconds,
                output,
            } => {
                let selection_json = fs::read(selection)?;
                let material_json = fs::read(material)?;
                let generation_method_json = fs::read(generation_method)?;
                let acquisition_bytes = acquisition_pairs(&profile, &receipt)?
                    .iter()
                    .map(|(profile, receipt)| {
                        Ok::<_, io::Error>((fs::read(profile)?, fs::read(receipt)?))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let acquisitions = acquisition_bytes
                    .iter()
                    .map(|(profile, receipt)| NjoyAcquisitionArtifacts {
                        profile_json: profile,
                        receipt_json: receipt,
                    })
                    .collect();
                let bundle = NjoyInputBundle::generate(
                    &evaluations_directory,
                    NjoyInputArtifacts {
                        evaluated_source_selection_json: &selection_json,
                        material_json: &material_json,
                        generation_method_json: &generation_method_json,
                        acquisitions,
                    },
                )?;
                let result = NjoyExecutionReceipt::execute(
                    &bundle,
                    NjoyExecutionOptions {
                        executable: &njoy_executable,
                        processor_support_artifacts: &processor_support_artifacts,
                        input_bundle_root: &input_bundle,
                        evaluations_root: &evaluations_directory,
                        output_root: &output,
                        timeout_seconds,
                    },
                )?;
                println!("executed NJOY2016.78 at {}", output.display());
                println!("nuclide runs: {}", result.receipt.runs.len());
                println!(
                    "processor SHA-256: {}",
                    result.receipt.processor.executable.sha256
                );
                println!("receipt: {}", result.receipt_path.display());
                println!("receipt SHA-256: {}", result.receipt_sha256);
                if result.receipt.rejected_run_count == 0 {
                    println!("qualification: execution_observed_unreviewed");
                } else {
                    println!("qualification: execution_observed_diagnostics_failed");
                    println!(
                        "rejected nuclide runs: {}",
                        result.receipt.rejected_run_count
                    );
                    return Err(io::Error::other(format!(
                        "{} NJOY run(s) exceeded kinematic diagnostic limits; receipt was preserved",
                        result.receipt.rejected_run_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyExecution {
                receipt,
                execution_directory,
            } => {
                let document = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                document.verify_execution_root(&execution_directory)?;
                println!(
                    "verified NJOY execution artifacts at {}",
                    execution_directory.display()
                );
                println!("receipt SHA-256: {}", document.sha256);
                println!("nuclide runs: {}", document.receipt.runs.len());
                println!(
                    "rejected nuclide runs: {}",
                    document.receipt.rejected_run_count
                );
                println!(
                    "qualification: {}",
                    if document.receipt.rejected_run_count == 0 {
                        "execution_observed_unreviewed"
                    } else {
                        "execution_observed_diagnostics_failed"
                    }
                );
            }
            NjoyCommand::AssessExecution {
                receipt,
                execution_directory,
                output,
            } => {
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let report = NjoySuitabilityReport::assess(&execution, &execution_directory)?;
                let result = report.write_new(&output)?;
                println!("assessed transported-photon KERMA suitability");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!("nuclide runs: {}", result.report.runs.len());
                println!(
                    "rejected nuclide runs: {}",
                    result.report.rejected_run_count
                );
                println!(
                    "processor data findings: {} unique / {} occurrences",
                    result.report.processor_finding_count,
                    result.report.processor_finding_occurrence_count
                );
                println!(
                    "kinematic violations: {}",
                    result.report.kinematic_violation_count
                );
                if result.report.rejected_run_count == 0 {
                    println!("qualification: transported_photon_kerma_candidate_unreviewed");
                } else {
                    println!("qualification: transported_photon_kerma_rejected");
                    return Err(io::Error::other(format!(
                        "{} NJOY run(s) are unsuitable for transported-photon KERMA; report was preserved",
                        result.report.rejected_run_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifySuitability {
                receipt,
                execution_directory,
                suitability_report,
            } => {
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let report = NjoySuitabilityReportDocument::from_path(&suitability_report)?;
                report.verify_against_execution(&execution, &execution_directory)?;
                println!(
                    "verified transported-photon suitability report {}",
                    suitability_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!("nuclide runs: {}", report.report.runs.len());
                println!(
                    "rejected nuclide runs: {}",
                    report.report.rejected_run_count
                );
                println!(
                    "qualification: {}",
                    if report.report.rejected_run_count == 0 {
                        "transported_photon_kerma_candidate_unreviewed"
                    } else {
                        "transported_photon_kerma_rejected"
                    }
                );
            }
            NjoyCommand::AssessSourceAware {
                legacy_report,
                receipt,
                execution_directory,
                input_manifest,
                photon_inventory,
                output,
            } => {
                let legacy = NjoySuitabilityReportDocument::from_path(&legacy_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let input_manifest = fs::read(input_manifest)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report = NjoySourceAwareSuitabilityReport::assess(
                    &legacy,
                    &execution,
                    &execution_directory,
                    &input_manifest,
                    &inventory,
                )?;
                let result = report.write_new(&output)?;
                println!("assessed source-aware transported-photon KERMA suitability");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "rejected nuclide runs: {}",
                    result.report.rejected_run_count
                );
                println!(
                    "processor findings: {} rejecting / {} informational",
                    result.report.rejecting_processor_finding_count,
                    result.report.informational_processor_finding_count
                );
                println!(
                    "source format findings: {}",
                    result.report.source_format_finding_count
                );
                if result.report.rejected_run_count == 0 {
                    println!("qualification: transported_photon_kerma_candidate_unreviewed");
                } else {
                    println!("qualification: transported_photon_kerma_rejected");
                    return Err(io::Error::other(format!(
                        "{} NJOY run(s) remain unsuitable after source-aware interpretation; report was preserved",
                        result.report.rejected_run_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifySourceAware {
                legacy_report,
                receipt,
                execution_directory,
                input_manifest,
                photon_inventory,
                source_aware_report,
            } => {
                let legacy = NjoySuitabilityReportDocument::from_path(&legacy_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                let input_manifest = fs::read(input_manifest)?;
                let inventory =
                    EndfPhotonProductionInventoryDocument::from_path(&photon_inventory)?;
                let report =
                    NjoySourceAwareSuitabilityReportDocument::from_path(&source_aware_report)?;
                report.verify_against_evidence(
                    &legacy,
                    &execution,
                    &execution_directory,
                    &input_manifest,
                    &inventory,
                )?;
                println!(
                    "verified source-aware suitability report {}",
                    source_aware_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!(
                    "rejected nuclide runs: {}",
                    report.report.rejected_run_count
                );
                println!(
                    "informational File 13 findings: {}",
                    report.report.informational_processor_finding_count
                );
                println!(
                    "qualification: {}",
                    if report.report.rejected_run_count == 0 {
                        "transported_photon_kerma_candidate_unreviewed"
                    } else {
                        "transported_photon_kerma_rejected"
                    }
                );
            }
            NjoyCommand::AssessDomainAware {
                source_aware_report,
                legacy_report,
                receipt,
                execution_directory,
                input_manifest,
                nuclear_data_manifest,
                material,
                transport_domain,
                output,
            } => {
                let source_aware =
                    NjoySourceAwareSuitabilityReportDocument::from_path(&source_aware_report)?;
                let legacy = NjoySuitabilityReportDocument::from_path(&legacy_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                legacy.verify_against_execution(&execution, &execution_directory)?;
                let input_manifest = fs::read(input_manifest)?;
                let nuclear_data_manifest = fs::read(nuclear_data_manifest)?;
                let material = fs::read(material)?;
                let transport_domain =
                    OpenMcNeutronTransportDomainDocument::from_path(&transport_domain)?;
                let report = NjoyDomainAwareSuitabilityReport::assess(
                    &source_aware,
                    &legacy,
                    &execution,
                    &input_manifest,
                    &nuclear_data_manifest,
                    &material,
                    &transport_domain,
                )?;
                let result = report.write_new(&output)?;
                println!("assessed transport-domain-aware suitability");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "kinematic violations: {} full / {} in-domain / {} out-of-domain",
                    result.report.full_evaluation_kinematic_violation_count,
                    result.report.in_domain_kinematic_violation_count,
                    result.report.out_of_domain_kinematic_violation_count
                );
                println!(
                    "reclassified nuclide runs: {}",
                    result.report.reclassified_run_count
                );
                println!(
                    "rejected nuclide runs: {}",
                    result.report.rejected_run_count
                );
                if result.report.rejected_run_count == 0 {
                    println!("qualification: transported_photon_kerma_candidate_unreviewed");
                } else {
                    println!("qualification: transported_photon_kerma_rejected");
                    return Err(io::Error::other(format!(
                        "{} NJOY run(s) remain unsuitable in the bound transport domain; report was preserved",
                        result.report.rejected_run_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyDomainAware {
                source_aware_report,
                legacy_report,
                receipt,
                execution_directory,
                input_manifest,
                nuclear_data_manifest,
                material,
                transport_domain,
                domain_aware_report,
            } => {
                let source_aware =
                    NjoySourceAwareSuitabilityReportDocument::from_path(&source_aware_report)?;
                let legacy = NjoySuitabilityReportDocument::from_path(&legacy_report)?;
                let execution = NjoyExecutionReceiptDocument::from_path(&receipt)?;
                legacy.verify_against_execution(&execution, &execution_directory)?;
                let input_manifest = fs::read(input_manifest)?;
                let nuclear_data_manifest = fs::read(nuclear_data_manifest)?;
                let material = fs::read(material)?;
                let transport_domain =
                    OpenMcNeutronTransportDomainDocument::from_path(&transport_domain)?;
                let report =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                report.verify_against_evidence(
                    &source_aware,
                    &legacy,
                    &execution,
                    &input_manifest,
                    &nuclear_data_manifest,
                    &material,
                    &transport_domain,
                )?;
                println!(
                    "verified domain-aware suitability report {}",
                    domain_aware_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!(
                    "kinematic violations: {} full / {} in-domain / {} out-of-domain",
                    report.report.full_evaluation_kinematic_violation_count,
                    report.report.in_domain_kinematic_violation_count,
                    report.report.out_of_domain_kinematic_violation_count
                );
                println!(
                    "reclassified nuclide runs: {}",
                    report.report.reclassified_run_count
                );
                println!(
                    "qualification: {}",
                    if report.report.rejected_run_count == 0 {
                        "transported_photon_kerma_candidate_unreviewed"
                    } else {
                        "transported_photon_kerma_rejected"
                    }
                );
            }
            NjoyCommand::AssessEvidenceAware {
                domain_aware_report,
                law7_residual_report,
                law7_comparison_report,
                capture_balance_report,
                capture_comparison_report,
                output,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let law7_residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&law7_residual_report)?;
                let law7_comparison =
                    NjoyLaw7ImplicitResidualComparisonDocument::from_path(&law7_comparison_report)?;
                let capture_balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&capture_balance_report)?;
                let capture_comparison = NjoyCapturePhotonMomentComparisonDocument::from_path(
                    &capture_comparison_report,
                )?;
                let report = NjoyEvidenceAwareSuitabilityReport::assess(
                    &domain,
                    &law7_residual,
                    &law7_comparison,
                    &capture_balance,
                    &capture_comparison,
                )?;
                let result = report.write_new(&output)?;
                println!("assessed reaction-evidence-aware v0.4 suitability");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "kinematic violations: {} in-domain / {} approximation-attributed / {} remaining",
                    result.report.domain_in_scope_kinematic_violation_count,
                    result
                        .report
                        .approximation_attributed_in_domain_violation_count,
                    result.report.remaining_in_domain_kinematic_violation_count
                );
                println!(
                    "domain-status transitions: {} cleared / {} independently rejected",
                    result.report.reclassified_from_domain_run_count,
                    result.report.independently_rejected_from_domain_run_count
                );
                println!(
                    "rejected nuclide runs: {}",
                    result.report.rejected_run_count
                );
                if result.report.rejected_run_count == 0 {
                    println!("qualification: transported_photon_kerma_candidate_unreviewed");
                } else {
                    println!("qualification: transported_photon_kerma_rejected");
                    return Err(io::Error::other(format!(
                        "{} nuclide run(s) remain unsuitable after reaction-level evidence; report was preserved",
                        result.report.rejected_run_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyEvidenceAware {
                domain_aware_report,
                law7_residual_report,
                law7_comparison_report,
                capture_balance_report,
                capture_comparison_report,
                evidence_aware_report,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let law7_residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&law7_residual_report)?;
                let law7_comparison =
                    NjoyLaw7ImplicitResidualComparisonDocument::from_path(&law7_comparison_report)?;
                let capture_balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&capture_balance_report)?;
                let capture_comparison = NjoyCapturePhotonMomentComparisonDocument::from_path(
                    &capture_comparison_report,
                )?;
                let report =
                    NjoyEvidenceAwareSuitabilityReportDocument::from_path(&evidence_aware_report)?;
                report.verify_against_evidence(
                    &domain,
                    &law7_residual,
                    &law7_comparison,
                    &capture_balance,
                    &capture_comparison,
                )?;
                println!(
                    "verified reaction-evidence-aware v0.4 suitability {}",
                    evidence_aware_report.display()
                );
                println!("report SHA-256: {}", report.sha256);
                println!(
                    "kinematic violations: {} in-domain / {} approximation-attributed / {} remaining",
                    report.report.domain_in_scope_kinematic_violation_count,
                    report
                        .report
                        .approximation_attributed_in_domain_violation_count,
                    report.report.remaining_in_domain_kinematic_violation_count
                );
                println!(
                    "rejected nuclide runs: {}",
                    report.report.rejected_run_count
                );
                println!(
                    "qualification: {}",
                    if report.report.rejected_run_count == 0 {
                        "transported_photon_kerma_candidate_unreviewed"
                    } else {
                        "transported_photon_kerma_rejected"
                    }
                );
            }
            NjoyCommand::AssessDiagnosticTriage {
                evidence_aware_report,
                domain_aware_report,
                output,
            } => {
                let evidence =
                    NjoyEvidenceAwareSuitabilityReportDocument::from_path(&evidence_aware_report)?;
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let report = NjoyDiagnosticTriageReport::assess(&evidence, &domain)?;
                let result = report.write_new(&output)?;
                println!("triaged remaining in-domain NJOY diagnostics");
                println!("report: {}", result.report_path.display());
                println!("report SHA-256: {}", result.report_sha256);
                println!(
                    "remaining findings: {} original / {} source-data-blocked / {} requiring independent diagnostics",
                    result
                        .report
                        .original_remaining_in_domain_kinematic_violation_count,
                    result
                        .report
                        .source_data_blocked_in_domain_kinematic_violation_count,
                    result
                        .report
                        .independent_diagnostic_required_in_domain_kinematic_violation_count
                );
                if result
                    .report
                    .independent_diagnostic_required_in_domain_kinematic_violation_count
                    > 0
                {
                    return Err(io::Error::other(format!(
                        "{} in-domain finding(s) still require independent reaction diagnostics; triage report was preserved",
                        result
                            .report
                            .independent_diagnostic_required_in_domain_kinematic_violation_count
                    ))
                    .into());
                }
            }
            NjoyCommand::VerifyDiagnosticTriage {
                evidence_aware_report,
                domain_aware_report,
                triage_report,
            } => {
                let evidence =
                    NjoyEvidenceAwareSuitabilityReportDocument::from_path(&evidence_aware_report)?;
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let triage = NjoyDiagnosticTriageReportDocument::from_path(&triage_report)?;
                triage.verify_against_evidence(&evidence, &domain)?;
                println!(
                    "verified NJOY diagnostic triage {}",
                    triage_report.display()
                );
                println!("report SHA-256: {}", triage.sha256);
                println!(
                    "remaining findings: {} original / {} source-data-blocked / {} requiring independent diagnostics",
                    triage
                        .report
                        .original_remaining_in_domain_kinematic_violation_count,
                    triage
                        .report
                        .source_data_blocked_in_domain_kinematic_violation_count,
                    triage
                        .report
                        .independent_diagnostic_required_in_domain_kinematic_violation_count
                );
            }
            NjoyCommand::CheckDiagnosticTriage {
                domain_aware_report,
                law7_residual_report,
                law7_comparison_report,
                capture_balance_report,
                capture_comparison_report,
                evidence_aware_report,
                triage_report,
                output,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let law7_residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&law7_residual_report)?;
                let law7_comparison =
                    NjoyLaw7ImplicitResidualComparisonDocument::from_path(&law7_comparison_report)?;
                let capture_balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&capture_balance_report)?;
                let capture_comparison = NjoyCapturePhotonMomentComparisonDocument::from_path(
                    &capture_comparison_report,
                )?;
                let evidence =
                    NjoyEvidenceAwareSuitabilityReportDocument::from_path(&evidence_aware_report)?;
                let triage = NjoyDiagnosticTriageReportDocument::from_path(&triage_report)?;
                let result = NjoyDiagnosticTriageCheckResult::verify_and_build(
                    &triage,
                    &evidence,
                    &domain,
                    &law7_residual,
                    &law7_comparison,
                    &capture_balance,
                    &capture_comparison,
                )?;
                result.write_new(&output)?;
                println!("verified diagnostic-triage chain and wrote machine check");
                println!("result: {}", output.display());
                println!(
                    "response qualification: {}",
                    match result.response_qualification {
                        NjoySuitabilityQualification::TransportedPhotonKermaCandidateUnreviewed =>
                            "transported_photon_kerma_candidate_unreviewed",
                        NjoySuitabilityQualification::TransportedPhotonKermaRejected =>
                            "transported_photon_kerma_rejected",
                    }
                );
                println!(
                    "remaining findings: {} original / {} source-data-blocked / {} requiring independent diagnostics",
                    result.original_remaining_in_domain_kinematic_violation_count,
                    result.source_data_blocked_in_domain_kinematic_violation_count,
                    result.independent_diagnostic_required_in_domain_kinematic_violation_count
                );
            }
            NjoyCommand::CheckEvidenceAware {
                domain_aware_report,
                law7_residual_report,
                law7_comparison_report,
                capture_balance_report,
                capture_comparison_report,
                evidence_aware_report,
                output,
            } => {
                let domain =
                    NjoyDomainAwareSuitabilityReportDocument::from_path(&domain_aware_report)?;
                let law7_residual =
                    EndfMf6Law7ImplicitResidualReportDocument::from_path(&law7_residual_report)?;
                let law7_comparison =
                    NjoyLaw7ImplicitResidualComparisonDocument::from_path(&law7_comparison_report)?;
                let capture_balance =
                    EndfMf6CapturePhotonBalanceReportDocument::from_path(&capture_balance_report)?;
                let capture_comparison = NjoyCapturePhotonMomentComparisonDocument::from_path(
                    &capture_comparison_report,
                )?;
                let report =
                    NjoyEvidenceAwareSuitabilityReportDocument::from_path(&evidence_aware_report)?;
                let result = NjoyEvidenceAwareCheckResult::verify_and_build(
                    &report,
                    &domain,
                    &law7_residual,
                    &law7_comparison,
                    &capture_balance,
                    &capture_comparison,
                )?;
                result.write_new(&output)?;
                println!("verified evidence-aware suitability and wrote machine check");
                println!("result: {}", output.display());
                println!(
                    "qualification: {}",
                    match result.qualification {
                        NjoySuitabilityQualification::TransportedPhotonKermaCandidateUnreviewed =>
                            "transported_photon_kerma_candidate_unreviewed",
                        NjoySuitabilityQualification::TransportedPhotonKermaRejected =>
                            "transported_photon_kerma_rejected",
                    }
                );
                println!(
                    "remaining in-domain kinematic violations: {}",
                    result.remaining_in_domain_kinematic_violation_count
                );
            }
            NjoyCommand::CompareSuitability {
                baseline_report,
                candidate_report,
                output,
            } => {
                let baseline = NjoySuitabilityReportDocument::from_path(&baseline_report)?;
                let candidate = NjoySuitabilityReportDocument::from_path(&candidate_report)?;
                let comparison = NjoySuitabilityComparison::compare(&baseline, &candidate)?;
                let result = comparison.write_new(&output)?;
                println!("compared response-treatment candidate with rejected baseline");
                println!("comparison: {}", result.comparison_path.display());
                println!("comparison SHA-256: {}", result.comparison_sha256);
                println!(
                    "rejected nuclide runs: baseline={} candidate={}",
                    result.comparison.baseline_rejected_run_count,
                    result.comparison.candidate_rejected_run_count
                );
                println!(
                    "baseline rejections resolved: {}",
                    result.comparison.resolved_baseline_rejection_count
                );
                println!(
                    "new candidate rejections: {}",
                    result.comparison.introduced_rejection_count
                );
                println!(
                    "kinematic violations: baseline={} candidate={}",
                    result.comparison.baseline_kinematic_violation_count,
                    result.comparison.candidate_kinematic_violation_count
                );
                match result.comparison.qualification {
                    NjoySuitabilityComparisonQualification::CandidateRejected => {
                        println!("qualification: candidate_rejected");
                        return Err(io::Error::other(format!(
                            "candidate retains {} rejected nuclide run(s); comparison was preserved",
                            result.comparison.candidate_rejected_run_count
                        ))
                        .into());
                    }
                    NjoySuitabilityComparisonQualification::CandidateMechanicalGateClearUnreviewed => {
                        println!("qualification: candidate_mechanical_gate_clear_unreviewed");
                    }
                }
            }
            NjoyCommand::VerifyComparison {
                baseline_report,
                candidate_report,
                comparison_report,
            } => {
                let baseline = NjoySuitabilityReportDocument::from_path(&baseline_report)?;
                let candidate = NjoySuitabilityReportDocument::from_path(&candidate_report)?;
                let comparison = NjoySuitabilityComparisonDocument::from_path(&comparison_report)?;
                comparison.verify_against_reports(&baseline, &candidate)?;
                println!(
                    "verified response-treatment comparison {}",
                    comparison_report.display()
                );
                println!("comparison SHA-256: {}", comparison.sha256);
                println!(
                    "rejected nuclide runs: baseline={} candidate={}",
                    comparison.comparison.baseline_rejected_run_count,
                    comparison.comparison.candidate_rejected_run_count
                );
                println!(
                    "qualification: {}",
                    match comparison.comparison.qualification {
                        NjoySuitabilityComparisonQualification::CandidateRejected =>
                            "candidate_rejected",
                        NjoySuitabilityComparisonQualification::CandidateMechanicalGateClearUnreviewed =>
                            "candidate_mechanical_gate_clear_unreviewed",
                    }
                );
            }
            NjoyCommand::CheckCandidateComparison {
                baseline_report,
                candidate_report,
                comparison_report,
                output,
            } => {
                let baseline = NjoySuitabilityReportDocument::from_path(&baseline_report)?;
                let candidate = NjoySuitabilityReportDocument::from_path(&candidate_report)?;
                let comparison = NjoySuitabilityComparisonDocument::from_path(&comparison_report)?;
                let result = NjoyCandidateComparisonCheckResult::verify_and_build(
                    &comparison,
                    &baseline,
                    &candidate,
                )?;
                result.write_new(&output)?;
                println!("verified candidate comparison and wrote machine check");
                println!("result: {}", output.display());
                println!(
                    "candidate qualification: {}",
                    match result.candidate_qualification {
                        NjoySuitabilityComparisonQualification::CandidateRejected =>
                            "candidate_rejected",
                        NjoySuitabilityComparisonQualification::CandidateMechanicalGateClearUnreviewed =>
                            "candidate_mechanical_gate_clear_unreviewed",
                    }
                );
                println!(
                    "rejected nuclide runs: baseline={} candidate={} resolved={} introduced={}",
                    result.baseline_rejected_run_count,
                    result.candidate_rejected_run_count,
                    result.resolved_baseline_rejection_count,
                    result.introduced_rejection_count
                );
            }
        },
        Some(Command::Bio(args)) => match args.command {
            BioCommand::Apply {
                model,
                physical_bundle,
                region_masks,
                output,
            } => {
                let model_bytes = fs::read(&model)?;
                let model: BiologicalModel = serde_json::from_slice(&model_bytes)?;
                let bundle_bytes = fs::read(&physical_bundle)?;
                let physical: PhysicalDoseBundle = serde_json::from_slice(&bundle_bytes)?;
                let mut masks = Vec::new();
                for pair in &region_masks {
                    let (name, path) = pair.split_once('=').ok_or_else(|| {
                        io::Error::other(format!(
                            "region mask {pair:?} must be written as name=path"
                        ))
                    })?;
                    let mask: RegionMask = serde_json::from_slice(&fs::read(path)?)?;
                    if mask.name != name {
                        return Err(io::Error::other(format!(
                            "region mask {path} is named {:?}, expected {name:?}",
                            mask.name
                        ))
                        .into());
                    }
                    masks.push(mask);
                }
                let bundle = apply_biological_model(&model, &model_bytes, &physical, &masks)?;
                let json = serde_json::to_vec_pretty(&bundle)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output)?;
                file.write_all(&json)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                println!("biological dose bundle at {}", output.display());
                println!("model: {} sha256:{}", model.id, bundle.model.sha256);
                println!("physical provenance: {}", bundle.physical_bundle_provenance);
                println!("regions applied: {}", bundle.regions_applied.join(","));
                println!("qualification: {}", bundle.qualification);
            }
        },
        Some(Command::Dvh {
            dose,
            quantity,
            mask,
            bins,
            output,
        }) => {
            let dose_bytes = fs::read(&dose)?;
            let schema: serde_json::Value = serde_json::from_slice(&dose_bytes)?;
            let mask: RegionMask = serde_json::from_slice(&fs::read(&mask)?)?;
            let source = nctforge_core::ContentReference {
                id: dose.display().to_string(),
                sha256: nctforge_evidence::sha256_file(&dose)?,
            };
            let histogram = match schema
                .get("schema_version")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
            {
                nctforge_core::PHYSICAL_DOSE_BUNDLE_SCHEMA => {
                    let bundle: PhysicalDoseBundle = serde_json::from_slice(&dose_bytes)?;
                    let (values, unit) = dose_values(&bundle, &quantity)?;
                    let voxel_volume = bundle.geometry.spacing_mm.iter().product();
                    nctforge_evidence::DoseVolumeHistogram::compute(
                        &bundle.case_id,
                        &mask.name,
                        &quantity,
                        source,
                        unit,
                        values,
                        &mask.voxels,
                        voxel_volume,
                        bins,
                    )?
                }
                nctforge_bio::BIOLOGICAL_DOSE_BUNDLE_SCHEMA => {
                    let bundle: nctforge_bio::BiologicalDoseBundle =
                        serde_json::from_slice(&dose_bytes)?;
                    let (values, unit) = biological_dose_values(&bundle, &quantity)?;
                    let voxel_volume = bundle.geometry.spacing_mm.iter().product();
                    nctforge_evidence::DoseVolumeHistogram::compute(
                        &bundle.case_id,
                        &mask.name,
                        &quantity,
                        source,
                        unit,
                        values,
                        &mask.voxels,
                        voxel_volume,
                        bins,
                    )?
                }
                other => {
                    return Err(io::Error::other(format!(
                        "unsupported dose bundle schema {other:?}"
                    ))
                    .into());
                }
            };
            let json = serde_json::to_vec_pretty(&histogram)?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            file.write_all(&json)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            println!("dose-volume histogram at {}", output.display());
            println!(
                "region: {} ({} voxels)",
                histogram.region, histogram.region_voxel_count
            );
            println!("quantity: {} [{}]", histogram.quantity, histogram.unit);
        }
        Some(Command::Nifti(args)) => match args.command {
            NiftiCommand::Info { input } => {
                let image = read_nifti_file(&input)
                    .map_err(|error| io::Error::other(format!("nifti: {error}")))?;
                let g = &image.geometry;
                println!("shape: {} x {} x {}", g.shape[0], g.shape[1], g.shape[2]);
                println!(
                    "spacing_mm: [{}, {}, {}]",
                    g.spacing_mm[0], g.spacing_mm[1], g.spacing_mm[2]
                );
                println!(
                    "origin_mm: [{}, {}, {}]",
                    g.origin_mm[0], g.origin_mm[1], g.origin_mm[2]
                );
                println!("direction: {:?}", g.direction);
                println!("transform: {}", image.transform_source);
                println!(
                    "datatype: {} description: {:?}",
                    image.datatype, image.description
                );
            }
            NiftiCommand::ToMask {
                input,
                name,
                output,
            } => {
                let image = read_nifti_file(&input)
                    .map_err(|error| io::Error::other(format!("nifti: {error}")))?;
                let mask = nctforge_nifti::to_mask(&image, name);
                write_new_json(&output, &mask)?;
                println!(
                    "mask {} written ({} voxels included)",
                    mask.name,
                    mask.voxels.iter().filter(|v| **v).count()
                );
            }
            NiftiCommand::ExportDose {
                dose,
                quantity,
                output,
            } => {
                let bundle: PhysicalDoseBundle = serde_json::from_slice(&fs::read(&dose)?)?;
                let (values, _unit) = dose_values(&bundle, &quantity)?;
                let image = nctforge_nifti::NiftiImage {
                    geometry: bundle.geometry.clone(),
                    values: values.to_vec(),
                    datatype: 64,
                    transform_source: "sform",
                    description: format!("nctforge {} {}", bundle.case_id, quantity),
                    intent_name: String::new(),
                    units_declared_mm: true,
                };
                nctforge_nifti::write_nifti(&image, &output)?;
                println!("wrote {}", output.display());
            }
            NiftiCommand::Resample {
                input,
                target,
                interpolation,
                output,
            } => {
                let image = read_nifti_file(&input)
                    .map_err(|error| io::Error::other(format!("nifti: {error}")))?;
                let bundle: PhysicalDoseBundle = serde_json::from_slice(&fs::read(&target)?)?;
                let interpolation = match interpolation.as_str() {
                    "nearest" => nctforge_nifti::Interpolation::Nearest,
                    "trilinear" => nctforge_nifti::Interpolation::Trilinear,
                    other => {
                        return Err(io::Error::other(format!(
                            "unknown interpolation {other:?}; use nearest or trilinear"
                        ))
                        .into());
                    }
                };
                let resampled = nctforge_nifti::NiftiImage {
                    geometry: bundle.geometry.clone(),
                    values: nctforge_nifti::resample_to_grid(
                        &image,
                        &bundle.geometry,
                        interpolation,
                    ),
                    datatype: 64,
                    transform_source: "sform",
                    description: "nctforge resampled".into(),
                    intent_name: String::new(),
                    units_declared_mm: true,
                };
                nctforge_nifti::write_nifti(&resampled, &output)?;
                println!("wrote {}", output.display());
            }
        },
        Some(Command::Accumulate { plan, output }) => {
            let plan_bytes = fs::read(&plan)?;
            let plan_sha256 = nctforge_evidence::sha256_hex(&plan_bytes);
            let exposure_plan: ExposurePlan = serde_json::from_slice(&plan_bytes)?;
            exposure_plan
                .validate()
                .map_err(|error| io::Error::other(format!("exposure plan: {error}")))?;
            let plan_dir = plan.parent().unwrap_or(Path::new("."));
            let mut bundles = Vec::with_capacity(exposure_plan.exposures.len());
            for exposure in &exposure_plan.exposures {
                let path = plan_dir.join(&exposure.dose_bundle.path);
                let bytes = fs::read(&path)?;
                let actual = nctforge_evidence::sha256_hex(&bytes);
                if actual != exposure.dose_bundle.sha256 {
                    return Err(io::Error::other(format!(
                        "exposure {:?} bundle {} sha256 mismatch (plan {}, actual {})",
                        exposure.name,
                        path.display(),
                        exposure.dose_bundle.sha256,
                        actual
                    ))
                    .into());
                }
                let bundle: PhysicalDoseBundle =
                    serde_json::from_slice(&bytes).map_err(|error| {
                        io::Error::other(format!("exposure {:?} bundle: {error}", exposure.name))
                    })?;
                bundles.push(bundle);
            }
            let accumulated = accumulate_exposures(&exposure_plan, &plan_sha256, &bundles)
                .map_err(|error| io::Error::other(format!("accumulation: {error}")))?;
            let json = serde_json::to_vec_pretty(&accumulated)?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            file.write_all(&json)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            println!("accumulated dose bundle at {}", output.display());
            println!(
                "exposures: {} (covariance: independent)",
                exposure_plan.exposures.len()
            );
        }
        Some(Command::Evidence(args)) => match args.command {
            EvidenceCommand::Export {
                root,
                case_id,
                qualification,
                artifacts,
            } => {
                let qualification = match qualification.as_str() {
                    "synthetic_research_only" => {
                        nctforge_evidence::QualificationBoundary::SyntheticResearchOnly
                    }
                    "cross_code_research_only" => {
                        nctforge_evidence::QualificationBoundary::CrossCodeResearchOnly
                    }
                    "experimentally_validated_research_only" => {
                        nctforge_evidence::QualificationBoundary::ExperimentallyValidatedResearchOnly
                    }
                    other => {
                        return Err(io::Error::other(format!(
                            "unknown qualification boundary {other:?}"
                        ))
                        .into());
                    }
                };
                let mut inputs = Vec::new();
                for triple in &artifacts {
                    let (role, rest) = triple.split_once('=').ok_or_else(|| {
                        io::Error::other(format!(
                            "artifact {triple:?} must be written as role=SOURCE:DEST"
                        ))
                    })?;
                    let (source, dest) = rest.rsplit_once(':').ok_or_else(|| {
                        io::Error::other(format!(
                            "artifact {triple:?} must be written as role=SOURCE:DEST"
                        ))
                    })?;
                    inputs.push(nctforge_evidence::BundleInput {
                        role: role.into(),
                        source: PathBuf::from(source),
                        relative_path: dest.into(),
                        media_type: Some("application/json".into()),
                    });
                }
                let manifest = nctforge_evidence::export_evidence_bundle(
                    &root,
                    &case_id,
                    qualification,
                    &inputs,
                )?;
                println!("evidence bundle exported at {}", root.display());
                println!("artifacts: {}", manifest.artifacts.len());
            }
            EvidenceCommand::Verify { root } => {
                let manifest = nctforge_evidence::EvidenceBundleManifest::load_verified(&root)?;
                println!("evidence bundle verified at {}", root.display());
                println!("case: {}", manifest.case_id);
                println!("artifacts verified: {}", manifest.artifacts.len());
            }
        },
        Some(Command::Mask(args)) => match args.command {
            MaskCommand::Subtract {
                input,
                minus,
                name,
                output,
            } => {
                let mut mask = read_region_mask(&input)?;
                for path in &minus {
                    mask = mask
                        .subtract(&read_region_mask(path)?, name.clone())
                        .map_err(|error| io::Error::other(error.to_string()))?;
                }
                write_mask(&mask, &output)?;
            }
            MaskCommand::Union {
                inputs,
                name,
                output,
            } => {
                let mask = fold_region_masks(&inputs, &name, RegionMask::union)?;
                write_mask(&mask, &output)?;
            }
            MaskCommand::Intersect {
                inputs,
                name,
                output,
            } => {
                let mask = fold_region_masks(&inputs, &name, RegionMask::intersection)?;
                write_mask(&mask, &output)?;
            }
            MaskCommand::Threshold {
                ct_dir,
                min,
                max,
                name,
                output,
            } => {
                let mut slices = fs::read_dir(&ct_dir)?
                    .map(|entry| entry.map(|entry| entry.path()))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                slices.sort();
                let ct = nctforge_dicom::import_ct_series(&slices)
                    .map_err(|error| io::Error::other(error.to_string()))?;
                let mask = ct
                    .threshold_mask(name, min, max)
                    .map_err(|error| io::Error::other(error.to_string()))?;
                write_mask(&mask, &output)?;
            }
        },
        Some(Command::IrradiationTime {
            dose,
            quantity,
            source_strength,
            limits,
            masks,
            output,
        }) => {
            let dose_bytes = fs::read(&dose)?;
            let schema: serde_json::Value = serde_json::from_slice(&dose_bytes)?;
            let mut region_masks = Vec::with_capacity(masks.len());
            for binding in &masks {
                let (name, path) = binding
                    .split_once('=')
                    .ok_or_else(|| io::Error::other("--mask entries must be NAME=path"))?;
                let mask = read_region_mask(Path::new(path))?;
                if mask.name != name {
                    return Err(io::Error::other(format!(
                        "--mask {name}: mask file names itself {:?}",
                        mask.name
                    ))
                    .into());
                }
                region_masks.push(mask);
            }
            let mut organ_limits = Vec::with_capacity(limits.len());
            for entry in &limits {
                let (name, rest) = entry.split_once('=').ok_or_else(|| {
                    io::Error::other("--limit entries must be NAME=max|mean:LIMIT")
                })?;
                let (metric, value) = rest.split_once(':').ok_or_else(|| {
                    io::Error::other("--limit entries must be NAME=max|mean:LIMIT")
                })?;
                let metric = match metric {
                    "max" => nctforge_evidence::LimitMetric::Max,
                    "mean" => nctforge_evidence::LimitMetric::Mean,
                    other => {
                        return Err(io::Error::other(format!(
                            "--limit {name}: unknown metric {other:?} (max|mean)"
                        ))
                        .into());
                    }
                };
                let limit: f64 = value.parse().map_err(|_| {
                    io::Error::other(format!("--limit {name}: invalid limit {value:?}"))
                })?;
                organ_limits.push(nctforge_evidence::OrganLimit {
                    region: name.to_owned(),
                    metric,
                    limit,
                });
            }
            let source = nctforge_core::ContentReference {
                id: dose.display().to_string(),
                sha256: nctforge_evidence::sha256_file(&dose)?,
            };
            let report = match schema
                .get("schema_version")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
            {
                nctforge_core::PHYSICAL_DOSE_BUNDLE_SCHEMA => {
                    let bundle: PhysicalDoseBundle = serde_json::from_slice(&dose_bytes)?;
                    let (values, unit) = dose_values(&bundle, &quantity)?;
                    nctforge_evidence::IrradiationTimeReport::evaluate(
                        &bundle.case_id,
                        &quantity,
                        source,
                        unit,
                        values,
                        &region_masks,
                        &organ_limits,
                        source_strength,
                    )?
                }
                nctforge_bio::BIOLOGICAL_DOSE_BUNDLE_SCHEMA => {
                    let bundle: nctforge_bio::BiologicalDoseBundle =
                        serde_json::from_slice(&dose_bytes)?;
                    let (values, unit) = biological_dose_values(&bundle, &quantity)?;
                    nctforge_evidence::IrradiationTimeReport::evaluate(
                        &bundle.case_id,
                        &quantity,
                        source,
                        unit,
                        values,
                        &region_masks,
                        &organ_limits,
                        source_strength,
                    )?
                }
                other => {
                    return Err(io::Error::other(format!(
                        "unsupported dose bundle schema {other:?}"
                    ))
                    .into());
                }
            };
            write_new_json(&output, &report)?;
            println!("irradiation-time report at {}", output.display());
            for region in &report.regions {
                match (region.max_time_s, region.max_source_particles) {
                    (Some(time), Some(particles)) => println!(
                        "{} {:?} limit {}: {:.6e} endpoint/s -> max {:.6e} s ({:.6e} particles)",
                        region.region,
                        region.metric,
                        region.limit,
                        region.endpoint_rate_per_s,
                        time,
                        particles,
                    ),
                    _ => println!(
                        "{} {:?} limit {}: zero endpoint rate -> unbounded",
                        region.region, region.metric, region.limit,
                    ),
                }
            }
            match &report.limiting {
                Some(limiting) => println!(
                    "limiting structure: {} ({:?}), max {:.6e} s",
                    limiting.region, limiting.metric, limiting.max_time_s
                ),
                None => println!("no region bounds the irradiation (all endpoint rates zero)"),
            }
        }
        Some(Command::Position(args)) => match args.command {
            PositionCommand::Aim {
                case,
                source,
                mask,
                approach,
                direction,
                half_widths_cm,
                margin_cm,
                output_source,
                output_report,
            } => {
                let case: TransportCase = serde_json::from_slice(&fs::read(&case)?)?;
                let template: nctforge_transport::FixedSourceDefinition =
                    serde_json::from_slice(&fs::read(&source)?)?;
                let mask = read_region_mask(&mask)?;
                if half_widths_cm.len() != 2 {
                    return Err(
                        io::Error::other("--half-widths-cm must be HU,HV (two values)").into(),
                    );
                }
                let direction = if let Some(text) = &direction {
                    let parts: Vec<f64> = text
                        .split(',')
                        .map(|part| {
                            part.trim().parse().map_err(|_| {
                                io::Error::other(format!(
                                    "--direction component {part:?} is not a number"
                                ))
                            })
                        })
                        .collect::<Result<_, _>>()?;
                    if parts.len() != 3 {
                        return Err(io::Error::other(
                            "--direction must be dx,dy,dz (three components)",
                        )
                        .into());
                    }
                    [parts[0], parts[1], parts[2]]
                } else {
                    nctforge_transport::AxisApproach::parse(approach.as_deref().unwrap_or_default())
                        .map_err(|error| io::Error::other(error.to_string()))?
                        .unit_vector()
                };
                let (positioned, mut report) = nctforge_transport::aim_source_at_centroid(
                    &template,
                    &case.geometry,
                    &mask,
                    direction,
                    [half_widths_cm[0], half_widths_cm[1]],
                    margin_cm,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
                report.case_id = case.case_id.clone();
                write_new_json(&output_source, &positioned)?;
                write_new_json(&output_report, &report)?;
                println!("positioned source at {}", output_source.display());
                println!(
                    "target {:?} centroid LPS [{:.3}, {:.3}, {:.3}] mm",
                    report.target_region,
                    report.target_centroid_lps_mm[0],
                    report.target_centroid_lps_mm[1],
                    report.target_centroid_lps_mm[2]
                );
                println!(
                    "entry {:?}/{:?} at [{:.3}, {:.3}, {:.3}] mm, source-to-centroid {:.3} mm",
                    report.entry_axis,
                    report.entry_side,
                    report.entry_point_lps_mm[0],
                    report.entry_point_lps_mm[1],
                    report.entry_point_lps_mm[2],
                    report.source_to_centroid_mm
                );
            }
            PositionCommand::Rotate {
                source,
                axis,
                degrees,
                center_mm,
                output_source,
            } => {
                let source: nctforge_transport::FixedSourceDefinition =
                    serde_json::from_slice(&fs::read(&source)?)?;
                let axis = match axis.as_str() {
                    "x" => nctforge_transport::PlaneAxis::X,
                    "y" => nctforge_transport::PlaneAxis::Y,
                    "z" => nctforge_transport::PlaneAxis::Z,
                    other => {
                        return Err(io::Error::other(format!(
                            "--axis must be x|y|z, got {other:?}"
                        ))
                        .into());
                    }
                };
                let rotated = nctforge_transport::rotate_source(
                    &source,
                    [center_mm[0], center_mm[1], center_mm[2]],
                    axis,
                    degrees,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
                write_new_json(&output_source, &rotated)?;
                println!("rotated source at {}", output_source.display());
            }
        },
        None => {
            println!("NCTForge research scaffold");
            println!("Not commissioned or certified for clinical use.");
        }
    }
    Ok(())
}

fn write_new_json<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()
}

fn read_region_mask(path: &Path) -> Result<RegionMask, io::Error> {
    serde_json::from_slice(&fs::read(path)?)
        .map_err(|error| io::Error::other(format!("{}: {error}", path.display())))
}

fn write_mask(mask: &RegionMask, output: &Path) -> io::Result<()> {
    write_new_json(output, mask)?;
    println!(
        "mask {} written ({} voxels included)",
        mask.name,
        mask.included_voxel_count()
    );
    Ok(())
}

fn fold_region_masks(
    inputs: &[PathBuf],
    name: &str,
    op: fn(&RegionMask, &RegionMask, String) -> Result<RegionMask, nctforge_core::ValidationError>,
) -> Result<RegionMask, io::Error> {
    if inputs.len() < 2 {
        return Err(io::Error::other("at least two --inputs are required"));
    }
    let mut mask = read_region_mask(&inputs[0])?;
    for path in &inputs[1..] {
        mask = op(&mask, &read_region_mask(path)?, name.to_owned())
            .map_err(|error| io::Error::other(error.to_string()))?;
    }
    Ok(mask)
}

fn dose_values<'a>(
    bundle: &'a PhysicalDoseBundle,
    quantity: &str,
) -> Result<(&'a [f64], &'a str), Box<dyn Error>> {
    if let Some(name) = quantity.strip_prefix("component:") {
        let component = match name {
            "boron" => nctforge_core::DoseComponent::Boron,
            "nitrogen" => nctforge_core::DoseComponent::Nitrogen,
            "hydrogen" => nctforge_core::DoseComponent::Hydrogen,
            "photon" => nctforge_core::DoseComponent::Photon,
            other => {
                return Err(io::Error::other(format!("unknown dose component {other:?}")).into());
            }
        };
        let volume = bundle
            .components
            .iter()
            .find(|v| v.component == component)
            .ok_or_else(|| io::Error::other(format!("bundle lacks component {name}")))?;
        let unit = match volume.unit {
            nctforge_core::DoseUnit::Gray => "gray",
            nctforge_core::DoseUnit::GrayPerSourceParticle => "gray_per_source_particle",
        };
        return Ok((&volume.values, unit));
    }
    if quantity == "physical_total" {
        let unit = match bundle.physical_total.unit {
            nctforge_core::DoseUnit::Gray => "gray",
            nctforge_core::DoseUnit::GrayPerSourceParticle => "gray_per_source_particle",
        };
        return Ok((&bundle.physical_total.values, unit));
    }
    Err(io::Error::other(format!("unknown physical quantity {quantity:?}")).into())
}

fn biological_dose_values<'a>(
    bundle: &'a nctforge_bio::BiologicalDoseBundle,
    quantity: &str,
) -> Result<(&'a [f64], &'a str), Box<dyn Error>> {
    if let Some(name) = quantity.strip_prefix("component:") {
        let component = match name {
            "boron" => nctforge_core::DoseComponent::Boron,
            "nitrogen" => nctforge_core::DoseComponent::Nitrogen,
            "hydrogen" => nctforge_core::DoseComponent::Hydrogen,
            "photon" => nctforge_core::DoseComponent::Photon,
            other => {
                return Err(io::Error::other(format!("unknown dose component {other:?}")).into());
            }
        };
        let volume = bundle
            .components
            .iter()
            .find(|v| v.component == component)
            .ok_or_else(|| io::Error::other(format!("bundle lacks component {name}")))?;
        return Ok((&volume.values, &volume.unit));
    }
    if quantity == "biological_total" {
        return Ok((&bundle.total.values, &bundle.total.unit));
    }
    Err(io::Error::other(format!("unknown biological quantity {quantity:?}")).into())
}

fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / 1024.0_f64.powi(3)
}

fn qualification_name(qualification: EndfMf6CapturePhotonBalanceQualification) -> &'static str {
    match qualification {
        EndfMf6CapturePhotonBalanceQualification::MissingCapturePhotonDataRejected => {
            "missing_capture_photon_data_rejected"
        }
        EndfMf6CapturePhotonBalanceQualification::SpectrumNormalizationRejected => {
            "spectrum_normalization_rejected"
        }
        EndfMf6CapturePhotonBalanceQualification::CapturePhotonEnergyBalanceRejected => {
            "capture_photon_energy_balance_rejected"
        }
        EndfMf6CapturePhotonBalanceQualification::CapturePhotonEnergyBalanceCheckedUnreviewed => {
            "capture_photon_energy_balance_checked_unreviewed"
        }
    }
}

fn law7_qualification_name(
    qualification: EndfMf6Law7ImplicitResidualQualification,
) -> &'static str {
    match qualification {
        EndfMf6Law7ImplicitResidualQualification::SpectrumNormalizationRejected => {
            "spectrum_normalization_rejected"
        }
        EndfMf6Law7ImplicitResidualQualification::NegativeImplicitResidualEnergyRejected => {
            "negative_implicit_residual_energy_rejected"
        }
        EndfMf6Law7ImplicitResidualQualification::SpectrumNormalizationAndResidualEnergyRejected => {
            "spectrum_normalization_and_residual_energy_rejected"
        }
        EndfMf6Law7ImplicitResidualQualification::ImplicitResidualEnergyCheckedUnreviewed => {
            "implicit_residual_energy_checked_unreviewed"
        }
    }
}

fn law7_comparison_qualification_name(
    qualification: NjoyLaw7ImplicitResidualComparisonQualification,
) -> &'static str {
    match qualification {
        NjoyLaw7ImplicitResidualComparisonQualification::
            ProcessorApproximationFullyAttributedUnreviewed => {
                "processor_approximation_fully_attributed_unreviewed"
            }
        NjoyLaw7ImplicitResidualComparisonQualification::ProcessorAttributionRejected => {
            "processor_attribution_rejected"
        }
    }
}

fn energy_balance_attribution_qualification_name(
    qualification: NjoyEnergyBalanceAttributionQualification,
) -> &'static str {
    match qualification {
        NjoyEnergyBalanceAttributionQualification::
            ProcessorAccountingMechanismAttributedPhysicalValidationRequired => {
                "processor_accounting_mechanism_attributed_physical_validation_required"
            }
        NjoyEnergyBalanceAttributionQualification::ProcessorAccountingAttributionMismatch => {
            "processor_accounting_attribution_mismatch"
        }
    }
}

fn reaction_balance_qualification_name(
    qualification: EndfReactionBalanceQualification,
) -> &'static str {
    match qualification {
        EndfReactionBalanceQualification::SourceRemaindersComputedUnreviewed => {
            "source_remainders_computed_unreviewed"
        }
        EndfReactionBalanceQualification::SourceRemaindersPartiallyComputable => {
            "source_remainders_partially_computable"
        }
    }
}

/// Owned artifacts backing `NjoyResponseTableInputs`; the borrowed view is
/// built by `inputs()` once everything is loaded.
struct ResponseTableArtifacts {
    material: MaterialDefinition,
    material_bytes: Vec<u8>,
    component_profile: ComponentDefinitionProfile,
    component_profile_bytes: Vec<u8>,
    method: ResponseGenerationMethod,
    method_bytes: Vec<u8>,
    nuclear_data: NuclearDataManifest,
    nuclear_data_bytes: Vec<u8>,
    transport_domain: OpenMcNeutronTransportDomain,
    transport_domain_bytes: Vec<u8>,
    selection: EvaluatedNeutronSourceSelectionDocument,
    domain_aware: NjoyDomainAwareSuitabilityReportDocument,
    execution: NjoyExecutionReceiptDocument,
    execution_directory: PathBuf,
}

impl ResponseTableArtifacts {
    #[allow(clippy::too_many_arguments)]
    fn load(
        material: &Path,
        component_profile: &Path,
        generation_method: &Path,
        nuclear_data_manifest: &Path,
        transport_domain: &Path,
        selection: &Path,
        domain_aware_report: &Path,
        receipt: &Path,
        execution_directory: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let material_bytes = fs::read(material)?;
        let material: MaterialDefinition = serde_json::from_slice(&material_bytes)?;
        let component_profile_bytes = fs::read(component_profile)?;
        let component_profile: ComponentDefinitionProfile =
            serde_json::from_slice(&component_profile_bytes)?;
        let method_bytes = fs::read(generation_method)?;
        let method: ResponseGenerationMethod = serde_json::from_slice(&method_bytes)?;
        let nuclear_data_bytes = fs::read(nuclear_data_manifest)?;
        let nuclear_data: NuclearDataManifest = serde_json::from_slice(&nuclear_data_bytes)?;
        let transport_domain_bytes = fs::read(transport_domain)?;
        let transport_domain: OpenMcNeutronTransportDomain =
            serde_json::from_slice(&transport_domain_bytes)?;
        Ok(Self {
            material,
            material_bytes,
            component_profile,
            component_profile_bytes,
            method,
            method_bytes,
            nuclear_data,
            nuclear_data_bytes,
            transport_domain,
            transport_domain_bytes,
            selection: EvaluatedNeutronSourceSelectionDocument::from_path(selection)?,
            domain_aware: NjoyDomainAwareSuitabilityReportDocument::from_path(domain_aware_report)?,
            execution: NjoyExecutionReceiptDocument::from_path(receipt)?,
            execution_directory,
        })
    }

    fn inputs(&self) -> NjoyResponseTableInputs<'_> {
        NjoyResponseTableInputs {
            material: &self.material,
            material_bytes: &self.material_bytes,
            component_profile: &self.component_profile,
            component_profile_bytes: &self.component_profile_bytes,
            method: &self.method,
            method_bytes: &self.method_bytes,
            nuclear_data: &self.nuclear_data,
            nuclear_data_bytes: &self.nuclear_data_bytes,
            transport_domain: &self.transport_domain,
            transport_domain_bytes: &self.transport_domain_bytes,
            selection: &self.selection,
            domain_aware: &self.domain_aware,
            execution: &self.execution,
            execution_directory: &self.execution_directory,
        }
    }
}
