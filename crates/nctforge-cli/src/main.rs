// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use nctforge_dicom::synthetic::generate_nf_bnct_001;
use nctforge_dicom::verify_nf_bnct_001;
use nctforge_njoy::{
    DEFAULT_CAPTURE_ENERGY_BALANCE_RELATIVE_TOLERANCE,
    DEFAULT_LAW7_BREAKUP_NORMALIZATION_TOLERANCE, DEFAULT_LAW7_BREAKUP_RELATIVE_ENERGY_TOLERANCE,
    DEFAULT_NJOY_CAPTURE_PRINT_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_ENERGY_BALANCE_PRINT_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_LAW7_PRINT_RELATIVE_TOLERANCE, DEFAULT_NJOY_LAW7_SOURCE_RELATIVE_TOLERANCE,
    DEFAULT_NJOY_PRINT_RELATIVE_TOLERANCE, DEFAULT_NJOY_TIMEOUT_SECONDS,
    DEFAULT_SPECTRUM_NORMALIZATION_TOLERANCE, EndfContinuumPhotonMomentReport,
    EndfContinuumPhotonMomentReportDocument, EndfMf6CapturePhotonBalanceQualification,
    EndfMf6CapturePhotonBalanceReport, EndfMf6CapturePhotonBalanceReportDocument,
    EndfMf6Law7ImplicitResidualQualification, EndfMf6Law7ImplicitResidualReport,
    EndfMf6Law7ImplicitResidualReportDocument, EndfPhotonProductionInventory,
    EndfPhotonProductionInventoryDocument, NjoyCapturePhotonMomentComparison,
    NjoyCapturePhotonMomentComparisonDocument, NjoyDiagnosticTriageCheckResult,
    NjoyDiagnosticTriageReport, NjoyDiagnosticTriageReportDocument,
    NjoyDomainAwareSuitabilityReport, NjoyDomainAwareSuitabilityReportDocument,
    NjoyEnergyBalanceAttribution, NjoyEnergyBalanceAttributionDocument,
    NjoyEnergyBalanceAttributionQualification, NjoyEvidenceAwareCheckResult,
    NjoyEvidenceAwareSuitabilityReport, NjoyEvidenceAwareSuitabilityReportDocument,
    NjoyExecutionOptions, NjoyExecutionReceipt, NjoyExecutionReceiptDocument, NjoyInputArtifacts,
    NjoyInputBundle, NjoyLaw7ImplicitResidualComparison,
    NjoyLaw7ImplicitResidualComparisonDocument, NjoyLaw7ImplicitResidualComparisonQualification,
    NjoyPhotonMomentComparison, NjoyPhotonMomentComparisonDocument,
    NjoySourceAwareSuitabilityReport, NjoySourceAwareSuitabilityReportDocument,
    NjoySuitabilityComparison, NjoySuitabilityComparisonDocument,
    NjoySuitabilityComparisonQualification, NjoySuitabilityQualification, NjoySuitabilityReport,
    NjoySuitabilityReportDocument,
};
use nctforge_openmc::{
    DataAcquisitionClient, DataAcquisitionProfileDocument, DataAcquisitionReceiptDocument,
    EvaluatedNeutronSourceSelectionDocument, EvaluatedSourceQualification, NuclearDataManifest,
    OpenMcBackend, OpenMcNeutronTransportDomain, OpenMcNeutronTransportDomainDocument,
};
use nctforge_transport::{MaterialDefinition, TransportBackend};

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
}

#[derive(Debug, Args)]
struct OpenMcDataArgs {
    #[command(subcommand)]
    command: OpenMcDataCommand,
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
        /// Reviewed acquisition profile bound by the source selection.
        #[arg(long)]
        profile: PathBuf,
        /// Publisher-matched acquisition receipt bound by the source selection.
        #[arg(long)]
        receipt: PathBuf,
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
        /// Reviewed acquisition profile bound by the source selection.
        #[arg(long)]
        profile: PathBuf,
        /// Publisher-matched acquisition receipt bound by the source selection.
        #[arg(long)]
        receipt: PathBuf,
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
        /// Reviewed acquisition profile bound by the receipt.
        #[arg(long)]
        profile: PathBuf,
        /// Acquisition receipt checked into the case provenance.
        #[arg(long)]
        receipt: PathBuf,
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
                    let profile = DataAcquisitionProfileDocument::from_path(&profile)?;
                    let receipt = DataAcquisitionReceiptDocument::from_path(&receipt)?;

                    selection
                        .selection
                        .validate_for_material(&material, &material_bytes)?;
                    selection
                        .selection
                        .validate_acquisition(&profile, &receipt)?;
                    selection.selection.verify_files(&evaluations_directory)?;

                    println!("selection: {}", selection.selection.id);
                    println!("selection SHA-256: {}", selection.sha256);
                    println!(
                        "verified evaluations: {}",
                        selection.selection.evaluations.len()
                    );
                    println!(
                        "archive SHA-256: {}",
                        selection.selection.acquisition.archive_sha256
                    );
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
                let acquisition_profile_json = fs::read(profile)?;
                let acquisition_receipt_json = fs::read(receipt)?;
                let bundle = NjoyInputBundle::generate(
                    &evaluations_directory,
                    NjoyInputArtifacts {
                        evaluated_source_selection_json: &selection_json,
                        material_json: &material_json,
                        generation_method_json: &generation_method_json,
                        acquisition_profile_json: &acquisition_profile_json,
                        acquisition_receipt_json: &acquisition_receipt_json,
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
                let acquisition_profile_json = fs::read(profile)?;
                let acquisition_receipt_json = fs::read(receipt)?;
                let bundle = NjoyInputBundle::generate(
                    &evaluations_directory,
                    NjoyInputArtifacts {
                        evaluated_source_selection_json: &selection_json,
                        material_json: &material_json,
                        generation_method_json: &generation_method_json,
                        acquisition_profile_json: &acquisition_profile_json,
                        acquisition_receipt_json: &acquisition_receipt_json,
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
        },
        None => {
            println!("NCTForge research scaffold");
            println!("Not commissioned or certified for clinical use.");
        }
    }
    Ok(())
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
