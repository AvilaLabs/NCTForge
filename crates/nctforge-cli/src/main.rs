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
    DEFAULT_NJOY_TIMEOUT_SECONDS, NjoyExecutionOptions, NjoyExecutionReceipt,
    NjoyExecutionReceiptDocument, NjoyInputArtifacts, NjoyInputBundle, NjoySuitabilityComparison,
    NjoySuitabilityComparisonDocument, NjoySuitabilityComparisonQualification,
    NjoySuitabilityReport, NjoySuitabilityReportDocument,
};
use nctforge_openmc::{
    DataAcquisitionClient, DataAcquisitionProfileDocument, DataAcquisitionReceiptDocument,
    EvaluatedNeutronSourceSelectionDocument, EvaluatedSourceQualification, NuclearDataManifest,
    OpenMcBackend,
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
