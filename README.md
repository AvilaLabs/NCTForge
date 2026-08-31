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
  names, content-hashed profile and response-set identities, absolute voxel
  uncertainty, and a separately estimated physical total;
- a transport-backend trait;
- validated, backend-neutral material and fixed-source contracts with frozen
  machine inputs for `NF-BNCT-001`;
- a machine-validated four-component contributor ledger and an unqualified,
  reproducible NJOY partial-KERMA response-generation method;
- a fail-closed neutron response-set contract binding material, nuclear data,
  generation method, pointwise closure, and independent-review evidence;
- a case-scoped OpenMC nuclear-data inspector and preflight that bind table
  hashes and reject missing temperatures, energy coverage, reactions, heating,
  or photon data;
- a frozen official OpenMC ENDF/B-VIII.1 selection and a reproducible,
  pointwise MT 301 comparison against the controlled NJOY outputs;
- a Rust nuclear-data acquisition path with HTTPS redirect confinement, exact
  byte-range resume, explicit large-transfer confirmation, publisher-digest
  checks when available, and content-addressed receipts;
- a case-scoped ENDF/B-VIII.1 evaluated-neutron candidate selection that binds
  the current NNDC archive, acquisition receipt, frozen material, and all ten
  selected evaluation files by SHA-256;
- a standalone, no-overwrite NJOY2016.78 input generator that verifies every
  source and content binding, emits ten deterministic partial-KERMA decks, and
  freezes their `input_preparation_only` manifest;
- a controlled NJOY2016.78 runner that clears inherited environment state,
  binds the processor and declared runtime artifacts, requires exact output
  sections, parses kinematic findings, and emits an independently verifiable
  receipt even when scientific qualification fails;
- a deterministic transported-photon KERMA suitability gate that structures
  NJOY's photon-data fallback/incompleteness messages, combines them with
  kinematic findings, and can be independently regenerated from the raw logs;
- a byte-stable OpenMC 0.16 input-deck generator that verifies content
  bindings and selected nuclear-data files before emitting the complete tally
  ledger;
- an OpenMC adapter whose capability flags remain intentionally disabled until
  a real smoke run and statepoint import pass;
- a strict DICOM CT geometry and RT Structure Set import boundary;
- a deterministic generator and independent verifier for `NF-BNCT-001`;
- a backend-neutral `case.json` binding geometry, structure truth values, DICOM
  identifiers, and SHA-256 artifact integrity;
- an egui-independent, orientation-tested tri-planar view model with linked
  voxel crosshairs and explicit patient-side labels;
- content-hash and run-manifest primitives;
- a CLI and a native, evidence-aware egui workbench shell with overview,
  geometry, transport, component-dose, and evidence workspaces;
- an evidence-gated development roadmap;
- an explicit research and intellectual-property boundary;
- a researched technical baseline and frozen first synthetic conformance-case
  specification.

The R1 geometry milestone is implemented. CT slices are ordered from DICOM
patient-space geometry rather than filenames or Instance Number; affine, frame,
native pixel, and rescale invariants are validated; and the frozen RTSTRUCT is
rasterized to exact masks. The desktop workbench exposes the intended workflow
while keeping unfinished capabilities visibly blocked. Its geometry workspace
opens only an integrity-verified `NF-BNCT-001` case and provides linked axial,
coronal, and sagittal views, window/level, structure overlays, patient-edge
labels, and an LPS cursor readout. CI also requires all 41 generated DICOM
instances to pass independent IOD and cross-instance consistency validation
without errors or warnings.
Material and source inputs are now explicit, validated, and transport-neutral.
The first identity-oriented synthetic geometry can be translated into
deterministic OpenMC XML. Its exact evaluated-neutron source files and the
official processed OpenMC case selection are frozen. All ten generated MT 301
curves agree pointwise with the official processed tables within `4.9e-7`, but
the comparison also confirms effective local-photon fallback for O-17 and O-18.
Reviewed response tables therefore remain blocked; the project does not hide or
zero those contributions. Material mapping from general DICOM cases, particle
execution, statepoint import, biological modeling, and dose calculation are not
implemented yet. Transport capability flags remain false until their acceptance
gates pass.

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
crates/nctforge-openmc/     OpenMC preflight and deterministic input generator
crates/nctforge-njoy/       deterministic NJOY preparation, execution, and evidence
crates/nctforge-cli/        headless entry point
crates/nctforge-gui/        native egui application shell
bindings/python/            future scientific extension surface
benchmarks/synthetic/       public, non-patient validation corpus
profiles/                   reviewed external-data acquisition profiles
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

Without a case argument, the GUI opens on a research-readiness overview. Passing
a verified case opens its geometry workspace directly. Use the left navigation
to see the current OpenMC capability gates, the four-component dose workspace,
and the evidence ledger. Dose and transport actions are disabled because this
build has no qualified response bundle or executable backend; the interface
does not show placeholder dose values. See [ADR
0014](docs/adr/0014-evidence-aware-workbench-shell.md).

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

### Nuclear-data acquisition

NCTForge will not download multi-gigabyte nuclear data as a hidden build step.
First make a one-byte probe of the frozen official OpenMC profile:

```text
cargo run --bin nctforge -- openmc data probe \
  --profile profiles/openmc/openmc-endfb81-official-library.json
```

Acquisition requires the exact reported byte count and an existing output
directory. It writes a resumable `.part` file and a JSON receipt without
overwriting completed output. The official processed archive currently has no
published digest, so its receipt deliberately remains `acquisition_only`; a
locally calculated SHA-256 is byte identity, not scientific qualification. See
[ADR 0010](docs/adr/0010-verifiable-nuclear-data-acquisition.md).

After selective extraction, independently verify the checked manifest and the
material-specific capabilities with:

```text
cargo run --bin nctforge -- openmc data verify-manifest \
  --manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --data-root PATH-TO-SELECTED-OPENMC-DATA \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json
```

### NJOY input preparation

After acquiring and extracting the exact evaluated-neutron selection, generate
a new reviewable bundle with:

```text
cargo run --bin nctforge -- njoy prepare \
  --selection benchmarks/synthetic/nf-bnct-001/transport/evaluated-neutron-source-selection.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --generation-method benchmarks/synthetic/nf-bnct-001/transport/response-generation-method.json \
  --profile profiles/openmc/endfb81-neutron-evaluations.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/endfb81-neutron-acquisition-receipt.json \
  --evaluations-directory PATH-TO-EXACT-SELECTION \
  --output NEW-OUTPUT-DIRECTORY
```

The command executes no external processor and refuses an existing output
directory. The frozen benchmark copy is under
`benchmarks/synthetic/nf-bnct-001/transport/njoy/`; see [ADR
0011](docs/adr/0011-deterministic-njoy-input-preparation.md).

### Controlled NJOY execution evidence

`nctforge njoy execute` requires the same five content-bound source documents,
the exact prepared bundle, a real NJOY executable, explicitly declared runtime
support artifacts, and a new output directory. Run `nctforge njoy execute
--help` for the complete argument contract. It preserves a receipt before
returning a failure when NJOY reports a kinematic violation.

An execution directory can be checked later against an external receipt:

```text
cargo run --bin nctforge -- njoy verify-execution \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH-TO-COMPLETE-EXECUTION-DIRECTORY
```

The first canonical receipt is intentionally
`execution_observed_diagnostics_failed`, not a response table or reference
result. See [ADR 0012](docs/adr/0012-controlled-njoy-execution-evidence.md) and
the [structured finding summary](docs/research/NJOY2016_78_KINEMATIC_FINDINGS.md).

Derive the separately versioned data-suitability gate from a verified root:

```text
cargo run --bin nctforge -- njoy assess-execution \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH-TO-COMPLETE-EXECUTION-DIRECTORY \
  --output NEW-SUITABILITY-REPORT.json
```

The canonical assessment is `transported_photon_kerma_rejected`: O-17 and O-18
have no photon-production files, N-15 lacks File 12, and O-16 has a potentially
incomplete discrete photon sequence. See [ADR
0013](docs/adr/0013-transported-photon-kerma-suitability.md).

## License and use boundary

Code is licensed under Apache-2.0. Synthetic benchmark data will receive an
explicit data license before its first release.

NCTForge is research software. It is not a medical device, has not been
commissioned for any treatment facility, and must not be represented as
clinically qualified. See [DISCLAIMER.md](DISCLAIMER.md).

The repository must not implement Avify Dose patent subject matter without a
documented intellectual-property review. See [docs/IP_BOUNDARY.md](docs/IP_BOUNDARY.md).
