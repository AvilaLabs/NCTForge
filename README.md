# NCTForge

[![CI](https://github.com/AvilaLabs/NCTForge/actions/workflows/ci.yml/badge.svg)](https://github.com/AvilaLabs/NCTForge/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status: early research](https://img.shields.io/badge/status-early_research-orange.svg)](ROADMAP.md)
[![Clinical use: not validated](https://img.shields.io/badge/clinical_use-not_validated-red.svg)](DISCLAIMER.md)

**English** | [日本語](README.ja.md)

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
- an OpenMC adapter that now imports a completed run's statepoint into the
  normalized physical dose bundle while its prepare/execute capability flags
  remain intentionally disabled;
- a strict DICOM CT geometry and RT Structure Set import boundary;
- a deterministic generator and independent verifier for `NF-BNCT-001`;
- a backend-neutral `case.json` binding geometry, structure truth values, DICOM
  identifiers, and SHA-256 artifact integrity;
- an egui-independent, orientation-tested tri-planar view model with linked
  voxel crosshairs and explicit patient-side labels;
- content-hash and run-manifest primitives;
- a CLI and a native, evidence-aware egui workbench shell with overview,
  geometry, transport, component-dose, and evidence workspaces, official Avila
  Labs branding, contextual help, and guided spotlight tours;
- a bounded Python package (`bindings/python`, PyO3/maturin) whose compiled
  extension calls the same Rust contracts for case generation, verification,
  geometry and structure inspection, manifest and contract reading, and the
  honest capability and response-set review gates — checked by a
  cross-language parity suite against a clean-environment wheel install;
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
Those energy-accounting findings are dispositioned as explained and carried in
provenance (ADR 0031): the first component response tables are generated from
receipt-bound production HEATR PENDF output with exact B+N+H closure at each
of 7,526 union-grid knots and sealed `independently_reviewed` by deterministic
in-house regeneration, and `openmc collect` imports a completed run's
statepoint into the platform physical dose bundle. Material mapping from
general DICOM cases, backend-driven particle execution, biological modeling,
and dose calculation are not implemented yet. Transport capability flags
beyond import remain false until their acceptance gates pass.

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
bindings/python/            bounded PyO3/maturin scientific package
benchmarks/synthetic/       public, non-patient validation corpus
profiles/                   reviewed external-data acquisition profiles
schemas/                    versioned interchange schemas
docs/                       architecture, decisions, and qualification records
integrations/               bounded external orchestration specimens
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
to see the current OpenMC capability gates, the dose workspace, and the evidence
ledger. The dose workspace loads any validated physical or biological bundle
file — component statistics, totals, and a region-mask DVH plot — while keeping
the two layers visually distinct. The evidence workspace can verify an exported
`artifact-manifest.json` in place. Select the `?` button or press `F1` for
contextual guidance, bundled offline answers, and guided tours that dim the
application and spotlight live workflow controls. Interactive transport actions
stay disabled until the upstream response gates are qualified, and the interface
never shows placeholder dose values. See [ADR
0014](docs/adr/0014-evidence-aware-workbench-shell.md).

`pip install nctforge` is the planned primary distribution path for scientific
users, backed by the same Rust implementation through PyO3 and maturin. The
first bounded API is implemented under `bindings/python` and exercised by a
cross-language parity suite, but no PyPI release is published yet. Cargo
remains the native source/developer path, while desktop releases will ship as
native artifacts. See [ADR
0015](docs/adr/0015-python-and-native-distribution.md) and [ADR
0027](docs/adr/0027-first-bounded-python-api.md).

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

That immutable v0.1 report records the processor messages conservatively.
Source-aware v0.2 evidence subsequently recognizes valid File 13 alternatives,
and domain-aware v0.3 evidence scopes kinematic findings to a material- and
manifest-bound OpenMC interval without deleting full-range diagnostics. For
the JEFF-4.0 investigation this clears O-16's sole 30 MeV finding from the
bounded 20 MeV decision. An independent H-2 LAW=7 calculation confirms
normalized source distributions and positive mean energy for the implicit
proton, and a receipt-bound comparison attributes all 15 H-2 findings to
NJOY's excluded File 6 energy-balance remainder. Evidence-aware v0.4 consumes
that exact evidence and reclassifies only H-2 while independently restoring
N-15's capture-balance rejection. C-13, O-17, and O-18 retain 102 in-domain
findings. Diagnostic triage preserves all 102 while separating 59 findings on
the source-data-blocked C-13/O-18 runs from 43 O-17 findings. All 43 O-17
excesses now reproduce NJOY's printed per-reaction energy-balance accounting,
but that same-processor attribution waives none of them and cannot replace an
independent physical validation; the candidate remains rejected. See [ADR
0017](docs/adr/0017-source-aware-photon-production-suitability.md), [ADR
0019](docs/adr/0019-independent-mf6-capture-photon-balance.md), and [ADR
0020](docs/adr/0020-content-bound-transport-domain-suitability.md), followed by
[ADR 0021](docs/adr/0021-independent-law7-implicit-residual-balance.md).
The processor attribution is frozen in [ADR
0022](docs/adr/0022-law7-processor-attribution.md), and the integrated decision
is specified by [ADR
0023](docs/adr/0023-reaction-evidence-aware-suitability.md), and the bounded
work queue by [ADR
0025](docs/adr/0025-diagnostic-triage-of-remaining-njoy-findings.md), and the
O-17 attribution and response-path pause by [ADR
0026](docs/adr/0026-o17-processor-energy-balance-attribution.md); that pause is
superseded by [ADR
0031](docs/adr/0031-o17-diagnostic-queue-dispositioned.md), under which the
first component response tables are generated from the receipt-bound
production HEATR output and sealed `independently_reviewed` by deterministic
in-house regeneration.

### OpenMC input generation

With the sealed response set in place, generate the deterministic OpenMC deck
for the frozen smoke profile:

```text
cargo run --bin nctforge -- openmc generate \
  --case benchmarks/synthetic/nf-bnct-001/transport/case.json \
  --component-profile benchmarks/synthetic/nf-bnct-001/transport/component-profile.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --source benchmarks/synthetic/nf-bnct-001/transport/source.json \
  --response-set benchmarks/synthetic/nf-bnct-001/transport/provenance/neutron-response-set.json \
  --nuclear-data-manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --execution-profile benchmarks/synthetic/nf-bnct-001/transport/openmc-smoke-profile.json \
  --nuclear-data-root PATH-TO-SELECTED-ENDFB81-HDF5-ROOT \
  --output NEW-DECK-DIRECTORY
```

The generator verifies every content binding, requires the response set to
pass `validate_for_folding` (`independently_reviewed` under the ADR 0031
in-house deterministic-verification path), confirms the response energy range
covers the selected data, and refuses an existing output directory. The deck
executes under OpenMC 0.16.0 at commit
`617d35a5063c57796b43428bc401e627d2011046` with `OPENMC_CROSS_SECTIONS`
pointed at the manifest's `cross_sections.xml`.

After execution, `scripts/compare-openmc-smoke-estimators.py` binds the
statepoint to its input manifest and sealed response set, checks the tally
contract and executed energy-function tables, and freezes the ADR 0007
correlated-diagnostic estimator comparisons (coupled-heating closure,
component sum versus dedicated neutron heating, and reaction-rate times
evaluated mean deposited energy for B-10 and N-14) as a content-hashed report:

```text
python3 scripts/compare-openmc-smoke-estimators.py \
  --statepoint DECK-DIRECTORY/statepoint.5.h5 \
  --input-manifest DECK-DIRECTORY/nctforge-input-manifest.json \
  --response-set benchmarks/synthetic/nf-bnct-001/transport/provenance/neutron-response-set.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --execution-profile benchmarks/synthetic/nf-bnct-001/transport/openmc-smoke-profile.json \
  --execution-root PATH-TO-NJOY-EXECUTION-ROOT \
  --execution-receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json \
  --report-id nctforge.nf-bnct-001.openmc-smoke-estimator-comparison.v1 \
  --output NEW-COMPARISON-REPORT.json
```

`openmc collect` then imports the completed run into the platform result
model. The collector reads the newest `statepoint.N.h5` with a pure-Rust HDF5
path, refuses a nonzero exit code or an existing output, binds the run header
(batches, particles per batch, seed, stride, and the statepoint's recorded
OpenMC version) and every tally contract to the deck's input manifest, and
normalizes each component tally under its manifest-declared semantics into
gray per source neutron. The coupled-heating tally — no component, no
particle filter — supplies the dedicated physical total; particle-filtered
audit heating stays out of the bundle. The emitted
`nctforge.physical-dose-bundle/0.2.0` carries per-voxel 1-sigma uncertainties
and a provenance id binding both the input-manifest and statepoint SHA-256:

```text
nctforge openmc collect \
  --working-directory DECK-DIRECTORY \
  --exit-code 0 \
  --output NEW-DOSE-BUNDLE.json
```

The collected smoke bundle is execution evidence only — a five-batch,
thousand-history run cannot produce reference values, and the bundle makes no
clinical or qualification claim.

### DICOM-derived material assignment

`benchmark derive-materials` turns verified RT Structure Set masks into a
transport-neutral `nctforge.material-assignment/0.1.0` artifact: named,
non-overlapping, axis-aligned voxel boxes that each carry a
`MaterialDefinition`. Derivation is honest about its limits — every selected
ROI mask must equal its bounding box exactly, so the emitted CSG
decomposition is an exact representation, not an approximation of an
arbitrary mask — and the artifact binds the source `case.json` by SHA-256
provenance:

```text
nctforge benchmark derive-materials \
  --case-root CASE-ROOT \
  --case transport/case.json \
  --base-material transport/material.json \
  --map examples/derived/material-map.json \
  --output-assignment NEW-ASSIGNMENT.json \
  --output-case NEW-DERIVED-CASE.json
```

`--map` is a JSON object `{"regions": {"ROI_NAME": "material-file.json"}}`
whose paths resolve relative to the map file. The derived transport case
reuses the verified DICOM geometry and is written alongside the assignment.
`examples/derived/` ships a runnable demonstration that unloads boron from
the `CORE` box.

`openmc generate --assignment` then builds a multi-cell deck: one OpenMC
material per distinct region material, one CSG cell per region box, and the
base cell carved with the region complements. Generation gates keep the
result scientifically meaningful — the assignment's base material must equal
the bound material artifact byte-for-byte, region density and temperature
must match (collection still assumes one voxel mass), regions may not
introduce nuclides absent from the base material, and only nuclides covered
by `njoy_partial_kerma_fluence_fold` component estimators may change
fraction; uncovered nuclides must match the base exactly so the residual
response tables stay valid.

At collection the folded-response tallies still encode the base material's
atom densities, so `openmc collect` applies a per-voxel region/base
mass-fraction ratio to each covered component's values and 1-sigma
uncertainties, leaving residual, photon, and native-heating components
untouched. The emitted assignment and component profile are copied into the
deck directory and hash-verified against the manifest before any correction
is applied.

### Candidate-reference runs and acceptance evaluation

Candidate-reference execution profiles (`purpose: candidate_reference`,
profile schema `0.2.0`) must bind a predeclared acceptance contract via
`openmc generate --acceptance`; smoke profiles may not bind one. The contract
(`nctforge.acceptance-contract/0.1.0`) declares the acceptance regions — each
realized as its own OpenMC mesh so region sums carry proper batch statistics —
the evaluated mean deposited energies for the reaction-rate audits, the
precision and estimator-comparison gate tolerances, the frozen seed set, and
the minimum batch count. The generator emits one mesh plus nine ROI-scoped
tallies per region, binds the contract hash into the input manifest, and
writes the contract JSON into the deck directory.

`OpenMcBackend` can now drive a run itself: `prepare` generates the deck from
the configured artifact set, and `execute` launches the configured binary in
the run directory, captures stdout/stderr, and freezes an
`nctforge.openmc-run-receipt/0.1.0` recording the executable hash, environment
overlay, timestamps, exit code, and content hashes of every log and
statepoint artifact.

`openmc evaluate` reads each run directory's manifest, contract, and
statepoint; verifies seed registration and uniqueness, run-header and
tally-contract bindings, and the manifest's acceptance binding; then applies
the predeclared gates — ROI precision, per-voxel precision at or above 20% of
each component's maximum, and the estimator comparisons — plus reduced
chi-square consistency across independent seeds. It emits a content-hashed
`nctforge.openmc-acceptance-report/0.1.0`:

```text
nctforge openmc evaluate \
  --run RUN-DIRECTORY-SEED-A --run RUN-DIRECTORY-SEED-B --run RUN-DIRECTORY-SEED-C \
  --output NEW-ACCEPTANCE-REPORT.json
```

These are conformance thresholds for the synthetic benchmark, not clinical
commissioning tolerances; a passing report earns reference-result status for
the run set only within the case's declared qualification ceiling.

### Biological interpretation and dose-volume histograms

`nctforge-bio` is a separately versioned interpretation layer. A
`nctforge.biological-model/0.1.0` artifact assigns dimensionless
effectiveness weights to the four physical dose components, with optional
per-region overrides; `bio apply` produces a
`nctforge.biological-dose-bundle/0.1.0` whose weighted values never alias
physical dose (`weighted_gray*` unit labels, a `synthetic_research_only`
qualification, and content hashes binding the model and physical bundle).
The biological total's uncertainty is the fully-correlated sum of the
weighted component sigmas, since all components share transport histories.
The NF-BNCT-001 specification's exclusion of CBE/RBE/Gy-Eq claims is
preserved: biological bundles exist only when a model artifact is supplied,
and a demonstration model plus a core-region mask live under
`examples/biological/`:

```text
nctforge bio apply \
  --model examples/biological/fixed-component-weights-model-v1.json \
  --physical-bundle DOSE-BUNDLE.json \
  --region-mask core=examples/biological/core-region-mask.json \
  --output NEW-BIO-BUNDLE.json
```

### Multi-exposure accumulation

`nctforge accumulate` implements weighted irradiation-fraction and
multi-field aggregation under an `nctforge.exposure-plan/0.1.0` contract.
Each exposure binds a physical dose bundle by SHA-256 plus an explicit
delivery weight, weight basis, optional duration, and a boron-assumption
record. Accumulation sums `weight * dose` and propagates 1-sigma
uncertainties in quadrature — the only supported covariance model is
statistical independence between exposures; within each exposure the
dedicated physical-total estimator already accounts for component
covariance, so the accumulated total sums exposure totals rather than
recombining components. Every bundle must share the grid, component
profile, component set, and dose unit; the output is an ordinary
`nctforge.physical-dose-bundle/0.2.0` usable by `dvh`, `bio apply`, and the
GUI:

```text
nctforge accumulate \
  --plan examples/exposure/two-field-plan.json \
  --output accumulated-dose.json
```

`examples/exposure/` ships a two-field demonstration plan.

`nctforge dvh` computes a deterministic `nctforge.dose-volume-histogram/0.1.0`
over a named voxel mask for any component or total in a physical or
biological bundle — equal-width dose bins, differential volume fractions that
sum to one, and a cumulative `V(d)` curve:

```text
nctforge dvh \
  --dose DOSE-BUNDLE.json --quantity component:boron \
  --mask examples/biological/core-region-mask.json \
  --bins 100 --output NEW-DVH.json
```

### One-command runs and evidence bundles

`openmc run` chains `prepare` (deterministic deck), `execute` (run receipt),
and `collect` (normalized dose bundle) in one invocation; `--evidence-root`
additionally exports a `nctforge.evidence-bundle-manifest/0.1.0` directory
that binds every input, deck file, log, statepoint, and the dose bundle by
SHA-256 under a declared qualification boundary:

```text
nctforge openmc run \
  --case transport/case.json --component-profile transport/component-profile.json \
  --material transport/material.json --source transport/source.json \
  --response-set transport/provenance/neutron-response-set.json \
  --nuclear-data-manifest transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --execution-profile transport/openmc-smoke-profile.json \
  --nuclear-data-root DATA-ROOT --openmc /path/to/openmc \
  --env LD_LIBRARY_PATH=/path/to/libopenmc \
  --env OPENMC_CROSS_SECTIONS=DATA-ROOT/cross_sections.xml \
  --working-directory NEW-RUN-DIR --dose-output NEW-DOSE.json \
  --evidence-root NEW-BUNDLE-DIR

nctforge evidence verify --root BUNDLE-DIR
```

`evidence export`/`verify` are also available standalone for assembling
arbitrary hash-bound artifact sets. The exported layout follows the
evidence-bundle list predeclared in the NF-BNCT-001 specification.

A controlled ENDF/B-VIII.1 + TENDL-2025 mixed-source candidate was then
executed under selection schema `0.3.0`. The six shared nuclides reproduce the
baseline exactly, but all four TENDL-2025 substitutions remain rejected with
77 kinematic violations, and TENDL's duplicated File 3 grid points leave the
deeper independent gates uncomputable — a preserved source-format finding. All
three published evaluated libraries are now rejected under the controlled
chain, so the response path resumes only with a reviewed independent O-17
calculation. See
[ADR 0028](docs/adr/0028-mixed-evaluated-neutron-source-selections.md) and
[ADR 0029](docs/adr/0029-tendl2025-mixed-source-candidate.md).

The derived diagnostic-triage gate is also a live dogfood case for Avila Core.
NCTForge keeps the domain verification and emits a deterministic machine
result; Core binds the exact executable and inputs, records the run, evaluates
the 43-finding queue, and independently enforces the closed response category.
A second integration binds the candidate-comparison check so a rejected
candidate is a verified result rather than a process failure. See the
[integration case](integrations/avila-core/njoy-evidence-aware/README.md), the
[candidate-comparison case](integrations/avila-core/njoy-candidate-comparison/README.md),
and [ADR 0024](docs/adr/0024-avila-core-evidence-loop.md).

## License and use boundary

Code is licensed under Apache-2.0. Synthetic benchmark data will receive an
explicit data license before its first release.

NCTForge is research software. It is not a medical device, has not been
commissioned for any treatment facility, and must not be represented as
clinically qualified. See [DISCLAIMER.md](DISCLAIMER.md).

The repository must not implement Avify Dose patent subject matter without a
documented intellectual-property review. See [docs/IP_BOUNDARY.md](docs/IP_BOUNDARY.md).
