# NCTForge

NCTForge is a proposed transport-neutral, DICOM-native research and independent
verification workbench for boron neutron capture therapy (BNCT).

The project is being built from scratch in Rust. OpenMC is the first planned
calculation backend, not a permanent architectural dependency. Normalized
physical-dose contracts are intended to allow imported MCNP, PHITS, OpenPINT,
and other externally calculated results without bundling those systems.

## Current standing

This repository is an early research implementation, not a dose calculator. It
contains:

- a Rust workspace divided at the transport boundary;
- a validated four-component physical-dose data model with canonical component
  names, content-hashed profile identity, absolute voxel uncertainty, and a
  separately estimated physical total;
- a transport-backend trait;
- an intentionally non-functional OpenMC adapter;
- a strict DICOM CT geometry and RT Structure Set import boundary;
- a deterministic generator and independent verifier for `NF-BNCT-001`;
- a backend-neutral `case.json` binding geometry, structure truth values, DICOM
  identifiers, and SHA-256 artifact integrity;
- an egui-independent, orientation-tested tri-planar view model with linked
  voxel crosshairs and explicit patient-side labels;
- content-hash and run-manifest primitives;
- a CLI and a native egui geometry viewer for the verified synthetic case;
- an evidence-gated development roadmap;
- an explicit research and intellectual-property boundary;
- a researched technical baseline and frozen first synthetic conformance-case
  specification.

The R1 geometry milestone is implemented. CT slices are ordered from DICOM
patient-space geometry rather than filenames or Instance Number; affine, frame,
native pixel, and rescale invariants are validated; and the frozen RTSTRUCT is
rasterized to exact masks. The desktop viewer opens only an integrity-verified
`NF-BNCT-001` case and provides linked axial, coronal, and sagittal views,
window/level, structure overlays, patient-edge labels, and an LPS cursor
readout. CI also requires all 41 generated DICOM instances to pass independent
IOD and cross-instance consistency validation without errors or warnings.
Material mapping, particle transport, biological modeling, and dose calculation
are not implemented yet. Transport capability flags remain false until their
acceptance gates pass.

The first implementation target is
[`NF-BNCT-001`](benchmarks/synthetic/nf-bnct-001/SPECIFICATION.md). Its geometry,
material, and source are frozen before results exist; OpenMC results will not be
called reference values until independent evidence is available. The scientific
rationale is recorded in
[`docs/research/TECHNICAL_BASELINE.md`](docs/research/TECHNICAL_BASELINE.md).

## Intended invariant

```text
DICOM and case inputs
        |
backend-neutral case model
        |
transport adapter (OpenMC first)
        |
four physical dose components + uncertainty
        |
versioned biological interpretation
        |
QA, comparison, visualization, and evidence bundle
```

Physical transport, boron distribution, biological weighting, and uncertainty
must remain separable and independently inspectable.

## Workspace

```text
crates/nctforge-core/       geometry and component-dose contracts
crates/nctforge-dicom/      strict DICOM import and synthetic geometry benchmark
crates/nctforge-view/       patient-aligned tri-planar view geometry
crates/nctforge-transport/  backend interface and normalized run lifecycle
crates/nctforge-evidence/   hashes, manifests, and qualification boundary
crates/nctforge-openmc/     OpenMC adapter (scaffold)
crates/nctforge-cli/        headless entry point
crates/nctforge-gui/        native egui application shell
bindings/python/            future scientific extension surface
benchmarks/synthetic/       public, non-patient validation corpus
schemas/                    versioned interchange schemas
docs/                       architecture, decisions, and qualification records
```

## Build

The workspace pins Rust 1.95, the minimum required by eframe 0.36.1. Once the
toolchain is installed:

```text
cargo test --workspace --all-targets
cargo run --bin nctforge
cargo run --bin nctforge-gui
```

Generate and independently verify the first synthetic DICOM case:

```text
cargo run --bin nctforge -- benchmark generate /tmp/nf-bnct-001
cargo run --bin nctforge -- benchmark verify /tmp/nf-bnct-001
cargo run --bin nctforge-gui -- /tmp/nf-bnct-001
```

Generation refuses to overwrite an existing destination. Generated DICOM files
are ignored by default and contain visibly synthetic identity values only.

### Independent DICOM validation

CI pins Ubuntu 24.04's `dicom3tools` snapshot `20240118131615` and runs both
`dciodvfy` and `dcentvfy` against a newly generated case. With those tools on
your path, the same strict gate is:

```text
scripts/validate-dicom-iod.sh /tmp/nf-bnct-001
```

The gate rejects validator warnings as well as errors. Passing these tools is
useful interoperability evidence, not a DICOM certification or a guarantee of
clinical fitness.

## License and use boundary

Code is licensed under Apache-2.0. Synthetic benchmark data will receive an
explicit data license before its first release.

NCTForge is research software. It is not a medical device, has not been
commissioned for any treatment facility, and must not be represented as
clinically qualified. See [DISCLAIMER.md](DISCLAIMER.md).

The repository must not implement Avify Dose patent subject matter without a
documented intellectual-property review. See [docs/IP_BOUNDARY.md](docs/IP_BOUNDARY.md).
