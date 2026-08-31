// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use nctforge_dicom::synthetic::generate_nf_bnct_001;
use nctforge_dicom::verify_nf_bnct_001;
use nctforge_openmc::{DataAcquisitionClient, DataAcquisitionProfileDocument, OpenMcBackend};
use nctforge_transport::TransportBackend;

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
            },
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
