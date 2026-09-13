# NCTForge Development Roadmap

**Adopted:** 2026-08-31

**Status:** R1 complete; R2 response generation and smoke execution implemented; reference acceptance remains open. R3/R4 capability parity requirements expanded on 2026-09-12.

**Style:** Evidence-gated, not feature-count or calendar driven

## Goal

Establish an open, transport-neutral BNCT research and independent-verification
platform that makes geometry, component dosimetry, biological interpretation,
uncertainty, and provenance comparable between codes and institutions.


## OpenPINT capability parity and demonstrated superiority

**Adopted:** 2026-09-12, at the project owner's direction.

NCTForge's completion target is a full BNCT research workbench that matches
every verified OpenPINT research capability and demonstrates advantages in
accuracy, usability, performance, interoperability, and reproducibility.
Independent verification is an integrated capability, not a reason to omit
research planning or analysis functions. A remaining OpenPINT advantage is a
tracked gap to close, not an accepted permanent product boundary.

This is a development objective, not a claim of present superiority.
OpenPINT's age or adoption does not establish its technical quality. Completion
requires evidence from implemented workflows and comparable benchmarks.

### Audited baseline and source evidence

The initial comparison is against OpenPINT commit
[`7d035fbfc1b764cab0b112e45acee39e6b909efb`](https://github.com/ipostuma/OpenPINT/tree/7d035fbfc1b764cab0b112e45acee39e6b909efb),
reviewed on 2026-09-12. Its README and relevant implementation surfaces were
inspected; OpenPINT was not executed during this roadmap review. Thus
"present" below means visible implementation, not independently validated
scientific correctness or usability.

Source paths at that revision:

- [README and workflow inventory](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/README.md)
- [Treatment evaluation and combined treatments](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/OpenPINT/treatment.py)
- [Biological model functions](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/OpenPINT/dose/Model_db/models.py)
- [Component import, uncertainty, and fraction aggregation](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/OpenPINT/dose/bnct.py)
- [Patient positioning](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/OpenPINT/mcgenerator/patient_positioning.py)
- [MCNP deck generation](https://github.com/ipostuma/OpenPINT/blob/7d035fbfc1b764cab0b112e45acee39e6b909efb/OpenPINT/mcgenerator/mcnp.py)

The prior R3/R4 wording did not guarantee this breadth. In particular,
NCTForge's fixed-component biological weights do not establish parity with
photon-isoeffective models, fractionation, TCP/NTCP, or combined-treatment
analysis. An abstract backend interface does not establish working MCNP/PHITS
support. Each row below is required and remains open until its evidence passes.

### Required capability matrix

| ID / milestone | OpenPINT baseline | Required NCTForge deliverable | Acceptance evidence |
| --- | --- | --- | --- |
| OP-01 / R3 | NIfTI dose and mask workflows; CT-aligned mesh conversion | NIfTI import/export alongside DICOM; explicit LPS/RAS, affine, units, masks, interpolation and resampling semantics | Synthetic oblique, translated and anisotropic cases; landmark and volume checks; component round-trips; grid-resolution convergence with tolerances declared before execution |
| OP-02 / R3 | Synthetic patient masks, bounding boxes, material lattice generation | General heterogeneous synthetic anatomy and segmented-volume material/density mapping, including explicit tissue boron inputs | Multi-material head and cylindrical cases; analytic region volumes and densities; ambiguous mappings rejected; reproducible decks |
| OP-03 / R3 | Patient rotation, tumor-centroid and skin-entry positioning helpers | Interactive and scriptable research positioning, beam-entry geometry and source-to-anatomy transforms | Independent transform/landmark calculations, round-trips and out-of-volume rejection; matched GUI/CLI/Python outputs |
| OP-04 / R4 | MCNP and PHITS mesh readers; component uncertainty handling | Working MCNP and PHITS import adapters with explicit version/format support, component meaning, normalization and available statistical uncertainty | Independently created parser fixtures plus authorized real-engine examples; preservation of dose and uncertainty; no silent substitution of missing information |
| OP-05 / R4 | MCNP material/lattice and transform deck generation | MCNP input export for supported research geometry and sources, alongside native OpenMC preparation/execution | Generated decks executed by an authorized MCNP user; matched-case geometry and dose comparisons; transport programs remain external dependencies |
| OP-06 / R3 | Weighted irradiation-fraction aggregation | Multiple fields/exposures and fractions with explicit duration, source strength, boron assumptions and physical accumulation semantics | Hand-computable unequal-weight cases; dose-rate versus dose checks; alignment checks; covariance assumptions recorded; biological evaluation respects schedules rather than blindly summing weighted maps |
| OP-07 / R3 | Organ-limited irradiation time using physical, weighted and isoeffective endpoints | Research scenario evaluation under explicit maximum/mean organ limits, reporting the limiting structure/voxel and assumptions | Analytic scaling cases, multiple competing limits, zero-rate and infeasible cases, and independent checks for nonlinear biological endpoints |
| OP-08 / R3 | Weighted dose, photon-isoeffective tumor/healthy-tissue models and fractionation | Source-attributed model registry extending fixed weights to these model families, with explicit parameters, units, validity domains and fraction schedules | Independent equation-based fixtures and published reference cases for each model; parameter sensitivity; physical and biological results kept distinct |
| OP-09 / R3 | TCP, NTCP and UTCP functions; treatment metrics and DVHs | Tumor-control and normal-tissue-complication research estimates, combined endpoint where justified, standard dose-volume metrics and overlays | Probability bounds and limiting cases, analytic DVHs, explicit tissue/model provenance; distinguish low-level function availability from a working end-to-end workflow |
| OP-10 / R4 | Hadron-dose resampling, BED conversion and BNCT/hadron combination | Imported external photon/hadron dose, fraction-aware BED analysis and scientifically specified combined-treatment research evaluation | Analytic BED fixtures; co-registration checks; explicit compatible endpoint/model assumptions; incompatible biological quantities cannot be silently added |
| OP-11 / R3 | Structured plan configuration, Excel input and CLI/Python workflows | Validated research plan schema, CSV/XLSX interchange and complete GUI/CLI/Python paths for the covered tasks | Clean-install end-to-end examples; identical numerical results across surfaces; useful diagnostics for malformed plans; persistence and rerun of saved scenarios |
| OP-12 / R3 | Mask subtraction and limiting-organ construction | Region subtraction, exclusion masks and configurable CT-threshold region construction | Exact synthetic mask tests, frame checks, empty/overlapping region handling and recorded threshold choices; full segmentation remains a separate feature |

### Delivery sequence and completion gates

1. Finish the R2 physical benchmark and restore passing CI. Retain the
   predeclared scientific acceptance gates.
2. Complete OP-01/02/06 first so heterogeneous cases, interchange and exposure
   semantics are sound. Build OP-03/07/08/09/11/12 into the R3 research alpha.
   Simple fixed weights and DVH plumbing alone do not close these tasks.
3. Complete OP-04/05/10 in R4 and publish a reproducible cross-code case suite.
   One external importer alone no longer satisfies the full R4 target.
4. Carry all parity evidence into R5 external validation. An earlier alpha may
   ship with explicit gaps, but the full research platform is not complete
   while a verified baseline capability remains uncovered.
5. Refresh the pinned OpenPINT capability inventory at each release candidate.
   New implemented capabilities or measured advantages become named roadmap
   tasks. Track upstream plans separately from demonstrated functions.

For each OP task, maintain an evidence record containing implementation status,
NCTForge and comparator revisions, test inputs, expected outputs or independent
reference, acceptance tolerances, actual results, limitations and artifact
locations. Record unsupported and untested workflows as open.

### Demonstrating that the complete workbench is better

Freeze a comparison protocol before collecting results. Include homogeneous
and heterogeneous phantoms, oblique geometry, boundary-sensitive DVHs, multiple
exposures, biological model evaluation, and imported MCNP/PHITS outputs.
Use independently generated synthetic data and published methods.

- **Accuracy:** compare against analytic, independent numerical and available
  measured references. Agreement with OpenPINT alone is not truth. Report
  component/region error, statistical uncertainty, grid convergence and model
  assumptions; any material disadvantage gets a corrective roadmap task.
- **Performance:** compare identical analysis workloads on declared hardware;
  report wall time and peak memory. Separate NCTForge processing overhead from
  transport-engine time. Compare transport at equivalent precision, not merely
  equal particle histories. Rust alone is not evidence of greater speed.
- **Usability:** measure clean installation, time to first result, completion
  time, manual steps and errors for the same research tasks with external users.
  A GUI screenshot does not establish better usability.
- **Interoperability:** demonstrate actual DICOM and NIfTI round-trips, MCNP and
  PHITS imports, OpenMC execution and MCNP deck export on frozen cases.
- **Reproducibility:** recreate results from saved plans/evidence on a clean
  environment, retain model/data versions, and reject altered or incompatible
  artifacts.

The release comparison must show coverage of every OP row and report each
dimension honestly. The objective is parity or better across these dimensions
and measurable advantages beyond parity. Every measured regression remains a
tracked improvement task; no aggregate score may hide a material deficit.

Implement independently from scientific publications and public format
specifications with attribution. Do not copy OpenPINT implementation code.
The existing research-use and Avify Dose IP boundaries continue to apply.
Conventional scenario evaluation and the named parity tasks do not authorize
patent-sensitive optimization or clinical deployment.

## R0 — Architecture and risk register

Exit evidence:

- backend-neutral case and physical-dose contracts;
- OpenMC isolated behind a transport interface;
- Apache-2.0 and contribution policy;
- research and Avify Dose IP boundaries;
- synthetic-data-only repository policy;
- documented feasibility risks and acceptance gates.

## R1 — Geometry truth case

Deliver one synthetic DICOM CT and RTSTRUCT case with linked egui views.

Exit evidence:

- DICOM frame of reference and voxel affine preserved;
- axial, sagittal, and coronal orientation independently checked;
- structures round-trip within predeclared geometric tolerances;
- patient identifiers absent by construction;
- malformed or ambiguous geometry is rejected.

The frozen starting case is
[`NF-BNCT-001`](benchmarks/synthetic/nf-bnct-001/SPECIFICATION.md). R1 implements
its synthetic CT, RTSTRUCT, expected masks, and backend-neutral case manifest;
it does not wait for a transport engine.

Implementation status:

- complete: deterministic CT/RTSTRUCT generation;
- complete: patient-space CT ordering and affine validation;
- complete: exact frozen ROI masks, volumes, and LPS centroids;
- complete: malformed frame, plane, spacing, and orientation rejection tests;
- complete: one-command CLI generation and independent verification;
- complete: backend-neutral `case.json` with traversal-safe SHA-256 artifact
  verification;
- complete: UI-independent axial, coronal, and sagittal mappings with linked
  crosshair and independent edge-orientation tests;
- complete: integrity-gated egui viewer with linked axial, sagittal, and coronal
  views, LPS cursor, window/level controls, and RT structure overlays;
- complete: warning-free external CT/RT Structure Set IOD validation with
  `dciodvfy`, plus cross-instance consistency validation with `dcentvfy`, in CI.

## R2 — Physical component truth case

Calculate the four physical BNCT dose components for a simple analytic phantom.

Exit evidence:

- OpenMC version, commit, nuclear data, settings, seed, and inputs recorded;
- B-10, N-14, hydrogen/recoil, and photon definitions cited and tested;
- estimator limitations documented;
- analytic or independently calculated reference tolerances passed;
- statistical uncertainty retained at voxel level.

R2 also requires the response-generation and classification ledger specified by
`NF-BNCT-001`, plus an independent estimator comparison. OpenMC output alone is
not promoted to a reference result.

Implementation status:

- complete: OpenMC 0.16.0 estimator boundary and reaction-filter limitation
  recorded in ADR 0005;
- complete: canonical four-component physical-dose bundle with content-hashed
  profile identity, absolute voxel uncertainty, and independently derived
  physical-total uncertainty;
- complete: versioned explicit-nuclide material and unit-weight fixed-source
  contracts, including frozen machine inputs for `NF-BNCT-001`;
- complete: machine-validated contributor ledger and NJOY partial-KERMA
  generation method, explicitly held at `method_frozen_tables_pending`;
- complete: fail-closed response-set envelope and physical-dose-bundle binding,
  including pointwise neutron-KERMA closure and review-state enforcement;
- complete: case-scoped OpenMC nuclear-data manifest, HDF5 capability
  inspection, artifact verification, and cross-sections mapping preflight;
- complete: acquire the official 9.66 GB OpenMC ENDF/B-VIII.1 distribution,
  freeze its case-scoped acquisition receipt and 16 selected artifacts, and
  pass the material-specific transport capability preflight;
- complete: resumable, no-overwrite nuclear-data acquisition with frozen
  publisher profiles, redirect confinement, size and publisher-digest checks,
  and receipts bound into manifest schema `0.3.0`;
- complete: acquire the current publisher-matched NNDC neutron archive and
  freeze the ten `NF-BNCT-001` evaluations by path, size, and SHA-256 as an
  unqualified candidate, preserving the different OpenMC-recipe archive digest
  and unresolved equivalence state;
- complete: validate every selected ENDF material identity and generate the ten
  byte-stable NJOY2016.78 production/diagnostic decks with a content-bound,
  no-overwrite `input_preparation_only` manifest;
- complete: byte-stable OpenMC 0.16 input generation with complete scoring and
  audit tally ledgers;
- complete: controlled, no-overwrite NJOY2016.78 execution with exact
  input/output file sets, processor/runtime hashes, structured kinematic
  diagnostics, preserved rejected receipts, and independent artifact
  verification;
- complete: record the first ten-nuclide execution as rejected evidence after
  72 MT 301 violations across N-15, O-16, O-17, and O-18, without clipping or
  silently dropping an isotope;
- complete: derive and independently regenerate a transported-photon KERMA
  suitability report that rejects the same four nuclides on explicit NJOY
  missing/incomplete photon-data findings;
- complete: compare all ten production MT 301 tables pointwise against the
  official processed OpenMC tables with corresponding grids and no
  interpolation; the maximum relative difference is `4.892060e-7`;
- complete: establish that the official processed tables retain effective
  local-photon fallback for O-17 and O-18, so official-library acquisition does
  not by itself resolve the transported-photon KERMA blocker;
- complete: expose the intended workflow in an evidence-aware egui shell with
  overview, geometry, transport, four-component dose, and evidence workspaces;
  unfinished physics and backend actions remain explicitly disabled;
- complete: add official Avila Labs branding, contextual offline help, and
  guided dim-and-spotlight tours over live workflow controls;
- complete: add a separately versioned alternate-library selection and
  deterministic comparison contract, then run the first complete candidate
  gate with publisher-matched JEFF-4.0; it is rejected with six affected
  nuclides, 120 kinematic violations, no baseline rejection resolved, and two
  new rejections;
- complete: inventory exact MF=6/12/13/14/15 source records for both
  ten-nuclide selections and add source-aware suitability schema `0.2.0`; this
  corrects NJOY's File 12 message to informational when File 13 is valid,
  clearing JEFF N-15 while leaving five full-evaluation candidate rejections;
- complete: independently integrate all eight supported N-15 File 13/File 15
  continuum photon-energy moments for both selections and reproduce 58 shared-
  node NJOY diagnostics within their five-digit print precision;
- complete: independently reconstruct JEFF N-15 MF=6/MT=102 photon first and
  second moments, photon-momentum recoil, and Q-value balance; 33 of 37 source
  nodes fail a conservative 1% screen even though all spectra normalize and
  NJOY's self-bounded kinematic check reports zero violations;
- complete: derive and content-bind the common OpenMC neutron transport domain
  from the exact processed-data manifest and material, then apply it
  symmetrically through domain-aware suitability schema `0.3.0`; all 72
  baseline violations remain in domain, while six of 120 JEFF findings are
  above 20 MeV and O-16 alone clears with zero in-domain violations;
- complete: add evidence-aware suitability schema `0.4.0`, bind the exact H-2
  source and processor-attribution reports to the immutable domain assessment,
  clear all 15 attributed H-2 findings without deleting them, and integrate
  N-15's independent capture-balance rejection;
- complete: expose that evidence-aware gate through Avila Core's hash-bound
  external-checker protocol, preserving the categorical qualification and
  exact remaining-finding count as separate claims while NCTForge retains all
  domain logic;
- complete: partition the 102 remaining in-domain JEFF-4.0 findings into 59
  source-data-blocked C-13/O-18 findings and a 43-finding O-17 reaction queue;
- complete: reproduce all 43 O-17 MT 301 excesses from NJOY's printed
  per-reaction File 6 energy-balance remainders, while retaining all 43 for
  independent physical validation and waiving none;
- complete: add mixed-source selection schema `0.3.0`, NJOY input-manifest
  `0.2.0`, and repeatable profile/receipt CLI binding, then execute the
  ENDF/B-VIII.1 + TENDL-2025 candidate under the frozen method; the six shared
  nuclides reproduce the baseline exactly while all four TENDL substitutions
  remain rejected with 77 kinematic violations (70 in domain), no baseline
  rejection resolved, and none introduced;
- complete: attribute all 43 in-domain TENDL O-17 findings to NJOY's printed
  energy-balance remainders and expose the verified candidate comparison to
  Avila Core through `check-candidate-comparison`; TENDL's duplicated File 3
  grid points make the deeper independent gates uncomputable, which is
  preserved as a source-format finding;
- complete: compute independent File 3/File 6 reaction-level energy-balance
  remainders for JEFF-4.0 O-17 without NJOY; all 43 in-domain samples compute,
  and the independent remainder sum reproduces NJOY's printed `ebal` sums and
  the MT=301 excess within 1.7% at every finding energy, with the MT=91
  processor-internal convention and residual per-product `ebar` differences
  preserved as unreviewed evidence (ADR 0030);
- resolved: the O-17 queue is dispositioned as explained — the non-conservation
  is a documented property of the evaluated File 6 accounting, independently
  reproduced; the baseline photon-coverage findings are data-coverage findings
  (ADR 0031); all findings remain carried in provenance, none waived;
- complete: generate the first response tables under the frozen method —
  `generate-response-tables` folds receipt-bound production HEATR PENDF
  sections (MT=301 total, MT=443 kinematic, MT=407 B-10, MT=403 N-14 partial
  KERMA) onto a 7,526-knot union grid with exact B+N+H closure at every knot,
  and `verify-response-tables` reproduces both artifacts byte-exactly and
  seals the set as independently reviewed under the in-house deterministic
  verification path (ADR 0031); all 72 in-domain kinematic findings are
  carried into the generation report, none waived;
- complete: smoke execution under OpenMC 0.16.0 at the frozen commit —
  `openmc generate` materializes the deterministic deck bound to the reviewed
  response set, the run produces a five-batch statepoint honoring all twelve
  tally contracts, and `compare-openmc-smoke-estimators.py` freezes the ADR
  0007 correlated-diagnostic evidence: executed energy-function tables bitwise
  identical to the sealed set, coupled-heating closure within combined
  uncertainty, component response sum versus the dedicated neutron-heating
  estimator at the 1e-9 relative level, and B-10/N-14 reaction-rate audits
  reproducing the evaluated mean deposited energies within 2e-4;
- complete: statepoint import into the platform result model — `openmc
  collect` reads the newest statepoint through a pure-Rust HDF5 path, binds
  the run header, recorded OpenMC version, and tally contracts to the input
  manifest, normalizes the component tallies into gray per source neutron
  with per-voxel 1-sigma uncertainties, takes the coupled-heating tally as
  the dedicated physical total, and emits a validated
  `nctforge.physical-dose-bundle/0.2.0` whose provenance id binds the
  input-manifest and statepoint SHA-256 digests. The smoke bundle is
  execution evidence only; reference results still require a
  reference-statistics execution under the predeclared acceptance gates.

## R3 — End-to-end research alpha

Run a synthetic head case from DICOM through physical dose, a separately
versioned biological model, visualization, DVH, and evidence export.

Exit evidence:

- one-command reproducible run;
- GUI and CLI consume identical engine results;
- an installable Python alpha wraps the same Rust validation and model contracts
  without duplicating scientific logic;
- physical and biological layers can be inspected separately;
- deterministic manifest binds inputs and outputs;
- independent verifier rejects modified artifacts;
- all R3 tasks in the OpenPINT capability matrix pass their acceptance evidence.

Implementation status:

- complete: biological interpretation layer — `nctforge-bio` applies a
  separately versioned `nctforge.biological-model/0.2.0` artifact to a
  physical dose bundle, producing `nctforge.biological-dose-bundle/0.2.0`
  whose `weighted_gray*`/`weighted_eqd2` units and `synthetic_research_only`
  qualification can never alias physical dose;
- complete (OP-08, first slice): biological model families — models now
  declare `fixed_per_component` or `photon_isoeffective` weight semantics
  (the latter requiring every photon weight to be exactly 1.0), an optional
  free-text `validity_domain`, and an optional linear-quadratic
  `fractionation` block (fraction count, source particles per fraction,
  default + per-region α/β). A fractionated model transforms the weighted
  per-particle total to `weighted_eqd2` — `n·d·(1+d/(α/β))/(1+2/(α/β))` —
  with first-order sigma propagation, while components stay linear
  weighted values; both schema bumps are 0.2.0 because
  `deny_unknown_fields` makes the new fields forward-incompatible.
  Published reference cases now live in `conformance/bio/0.2.0/` — one
  byte-fixed golden bundle per model family plus stable rejection tokens
  (`cargo test -p nctforge-bio --test bio_conformance`). Parameter
  sensitivity is covered by `nctforge bio sweep` / Python
  `sweep_biological_model` — a `nctforge.bio-sensitivity-sweep/0.1.0`
  record sweeping component weights, region overrides, or fractionation
  parameters with per-point re-validation and region-masked
  min/mean/max;
- complete: deterministic `nctforge.dose-volume-histogram/0.1.0` over named
  voxel masks for any bundle component or total;
- complete: evidence-bundle export — `evidence export` collects inputs,
  deck files, logs, statepoints, and dose bundles into a
  `nctforge.evidence-bundle-manifest/0.1.0` directory; `evidence verify`
  re-checks every artifact hash and rejects tampering;
- complete: one-command `openmc run` chaining prepare, execute, and
  collect, with optional `--evidence-root` export;
- complete: GUI parity — the dose workspace loads validated physical and
  biological bundles (component statistics, totals, region-mask DVH plot)
  and the evidence workspace verifies bundle manifests in place; no
  placeholder values and no GUI-side scientific logic;
- complete: Python parity — the bounded API now exposes dose-bundle
  loading, biological-model application, and DVH computation over the same
  Rust contracts, with an extended wheel-level parity suite;
- complete (OP-02, first slice): DICOM-derived material assignment —
  `benchmark derive-materials` emits a transport-neutral
  `nctforge.material-assignment/0.2.0` from verified ROI masks. Box-exact
  masks become CSG cell regions; arbitrary masks become `voxel_set` regions
  realized as a rectilinear material lattice (one universe per distinct
  material, one element per voxel). Region densities may differ from the
  base — collection normalizes heating by per-voxel mass and rescales
  folded components by the atom-density ratio (density × mass fraction),
  residual folds by the density ratio alone. Rotated grids, overlaps,
  out-of-grid indices, and duplicated voxels are all rejected;
- in progress: candidate-reference execution — three frozen-seed 300M-history
  runs are executing under the predeclared acceptance contract; the report
  and reference-status decision follow when the runs complete;
- complete (OP-02, continued): `benchmark derive-materials --mask NAME=path`
  binds map keys to external `RegionMask` JSONs (e.g. `nifti to-mask`
  output) instead of RT Structure Set ROIs, with name/voxel-count checks —
  segmented NIfTI volumes now drive material assignment end-to-end;
- complete (OP-12, first slice): mask operations — `nctforge mask`
  subtract/union/intersect `RegionMask` volumes with shared-frame and
  non-empty-result checks, and `mask threshold` builds regions from a DICOM
  CT modality (HU) window for limiting-organ construction;
- complete (OP-07, first slice): organ-limited irradiation time —
  `nctforge irradiation-time` evaluates per-source-particle endpoints
  (physical or biological weighted) under per-region max/mean limits,
  emitting `nctforge.irradiation-time-report/0.1.0` with the limiting
  structure, per-region time and particle budgets, and declared
  assumptions; zero-statistic regions are unbounded and absolute-unit
  endpoints are rejected;
- complete (OP-03, first slice): positioning helpers — `nctforge position
  aim` derives a `UniformAxisPlane` source whose beam axis passes through a
  mask centroid (axis approaches and oblique directions), reporting the
  entry face/point and source-to-centroid distance; `position rotate`
  applies quarter-turns about a patient axis and rejects
  non-axis-aligned results. Python parity (`aim_source`,
  `rotate_source`, `PositionReport`) and a GUI positioning panel in the
  transport workspace (ROI or mask-file target, approach axis, quarter-turn
  rotate, report/source export) now match the CLI — all three surfaces run
  the same `nctforge_transport` functions;
- in progress (OP-01): NIfTI imaging I/O — `nctforge-nifti` reads and writes
  NIfTI-1 `.nii`/`.nii.gz` 3-D scalar volumes (`u8`–`f64`), prefers sform
  over qform, converts RAS+ to patient LPS with transform provenance,
  accepts explicit-mm or unspecified units, and rejects unsupported
  dimensions/datatypes/transforms/non-mm units. `nctforge nifti` provides
  `info`, `to-mask`, `export-dose`, and `resample` (nearest/trilinear);
  affine handling is regression-tested against independent `nibabel`
  output including an oblique sform. The GUI dose workspace surfaces the
  same four operations — volume inspection with transform provenance,
  mask writing, dose export, and nearest/trilinear resampling onto a
  bundle grid. Grid-resolution convergence evidence is pinned by an
  analytic test: a smooth field sampled at 2.0/1.0/0.5 mm and resampled
  onto a fixed incommensurate target must show interior L∞ error ratios
  below the predeclared 0.35 second-order bound per halving. CT-aligned
  mesh conversion is covered: `nifti resample --target case.json` (and
  the GUI panel) map external volumes onto the transport case's
  CT-aligned `GridGeometry` through the shared `read_target_geometry`
  path, which also accepts dose bundles and refuses unknown schemas;
- complete (OP-06): weighted exposure/fraction aggregation — the
  `nctforge.exposure-plan/0.1.0` contract binds each exposure's dose bundle
  by SHA-256 with an explicit weight, weight basis, duration, and boron
  assumption; `nctforge accumulate` sums weighted doses with quadrature
  1-sigma propagation under declared inter-exposure independence, emitting
  an ordinary physical dose bundle that flows through DVH, biological
  models, and the GUI;
- complete (OP-09, first slice): dose-volume metrics and endpoint models —
  `nctforge metrics` computes exact `D_x`/`V_x`/min/mean/max/EUD readings
  over a region mask (`nctforge.dose-metrics/0.1.0`), and `nctforge
  endpoint` scores `nctforge.endpoint-model/0.1.0` artifacts —
  `voxel_poisson_tcp` (voxel-level LQ Poisson TCP over per-particle dose),
  `logistic`, and `probit` (Lyman) functions over mean/min/max/EUD
  statistics — emitting `nctforge.endpoint-evaluation/0.1.0`, with `endpoint
  utcp` combining TCP+NTCP under `p_plus`/`difference` after matching
  case/region/quantity/source checks. Python exposes the same functions,
  and the GUI dose workspace overlays region metrics through the shared
  `RegionDoseMetrics` path with metrics-artifact export. Public
  reference fixtures live in `conformance/endpoints/0.1.0/` covering all
  three function families and both UTCP combinations;
- complete (OP-11, first slice): plan tables and diagnostics — the
  `nctforge-plan` crate round-trips `nctforge.exposure-plan/0.1.0` through
  CSV (`# key:` metadata) and XLSX (`plan`+`exposures` sheets), reporting
  every malformed row with its row number and hashing bundles on import
  when `sha256` cells are blank; `nctforge plan import|export|validate`,
  `ExposurePlan::validate_diagnostics`, and
  `nctforge_plan::accumulate_plan_file` serve CLI, Python
  (`load_exposure_plan`, `exposure_plan_diagnostics`,
  `accumulate_exposures`, `plan_table_read`, `plan_table_write`), and the
  GUI Plan workspace identically;
- complete: optional GUI slice-overlay of loaded dose on the patient grid —
  the geometry workspace loads a physical or biological dose bundle gated on
  matching `case_id` and equivalent grid geometry, then blends a hot-ramp
  dose wash (component or total, opacity and %of-max threshold controls)
  into the linked axial/sagittal/coronal renders, with the dose value
  surfaced at the linked voxel;

## R4 — Transport-neutral reference platform

Add generic component-dose import and cross-code comparison cases.

Exit evidence:

- published interchange schema;
- at least one result produced outside OpenMC imported without loss of meaning;
- OpenMC and one independent transport path compared on frozen cases;
- public conformance suite and versioned reference outputs;
- Python API supports external biological-model experiments without duplicating
  production evaluation logic;
- all R4 tasks in the OpenPINT capability matrix pass, including both MCNP and
  PHITS import, MCNP deck export, and external-dose research analysis.

Implementation status:

- complete (first slice): published interchange schema + generic importer —
  `nctforge.component-dose-interchange/0.1.0` records the producing system,
  version, and normalization, the transport-neutral grid, and the four
  physical component volumes with optional per-voxel sigmas. `nctforge
  import interchange` and Python `import_component_dose` share one Rust
  path validating it into `nctforge.physical-dose-bundle/0.2.0`.
  `dedicated` totals keep their estimator sigmas; `component_sum` totals
  record `unavailable` uncertainty rather than claiming a dedicated
  estimator. Bundle provenance binds the document SHA-256
  (`interchange:<system>:sha256:<hash>`) and threads through
  `dvh`/`metrics`/`bio apply` — verified end-to-end on a synthetic
  PHITS-labeled fixture in `examples/interchange/` (analytic stand-in
  values, not PHITS output);
- complete (first slice): public conformance suite —
  `conformance/interchange/0.1.0/` ships a manifest-driven fixture set (4
  valid documents with byte-fixed golden bundles, 15 rejection cases with
  stable error tokens) exercised by
  `cargo test -p nctforge-core --test interchange_conformance`;
  `conformance/adapters/0.1.0/` extends coverage end-to-end — documented
  MCNP meshtal and PHITS `xyz`-mesh inputs replay through each adapter into
  byte-fixed interchange documents and bundles
  (`cargo test -p nctforge-mcnp|nctforge-phits --test adapter_conformance`,
  same `NCTFORGE_UPDATE_CONFORMANCE` regeneration convention);
- complete (documented-format slice): MCNP/PHITS import adapters — the
  `nctforge-mcnp` crate parses ASCII `meshtal` files (tally + optional
  energy-bin selection, `Rel Error` → absolute sigmas) and the
  `nctforge-phits` crate parses `xyz`-mesh `.out` files (echo-parsed grid,
  `#newpage` axis pages, inline `r.err` preferred over `_err` siblings,
  incomplete error coverage rejected). Both emit the interchange document
  and validate through the shared importer; `nctforge import mcnp|phits`
  and Python `import_mcnp_meshtal`/`import_phits` are parity surfaces with
  document-hash provenance. Parser fixtures are authored to documented
  formats — real-engine acceptance remains the open gate;
- complete (documented-format slice): MCNP deck export — `nctforge export
  mcnp` and Python `export_mcnp_deck` emit a deterministic deck from a
  transport case: grid `RPP` box, `voxel_box` regions as carved CSG cells,
  `voxel_set` regions via a `LAT=1` lattice `FILL` array (one universe per
  distinct material), `M` cards from declared mass fractions (operator's
  `--xs-suffix` or xsdir defaults — never an invented library), `SDEF`
  plane source, `NPS`, and `FMESH` flux tallies on the case mesh. The deck
  deliberately carries no component folding — that stays the external
  pipeline's declared step before `import mcnp` re-ingests the meshtal.
  Source planes outside the grid are rejected. Execution against real MCNP
  remains the open acceptance gate;
- complete (first slice): OP-10 external-dose/BED combined analysis —
  `nctforge.external-dose/0.1.0` imports one absolute-dose course with
  declared fractionation (uniform or explicit, hash-bound provenance);
  `nctforge bio bed` converts it to a BED/EQD2 field under declared
  (optionally per-region) α/β with first-order sigma propagation; `nctforge
  bio combine` adds an external `eqd2` field to a photon-isoeffective
  `weighted_eqd2` bundle — the only admitted combination — under a required
  operator-stated additivity assumption, with strict trilinear
  co-registration (`--resample trilinear`) when grids differ and rejection
  of every incompatible-quantity or uncovered-target path. The
  `nctforge.combined-dose/0.1.0` record binds both input content hashes,
  both provenance chains, and the declared assumption. CLI/Python parity
  (`import_external_dose`, `bed_from_external_dose`,
  `combine_biological_doses`) with analytic BED fixtures in unit and parity
  tests; `examples/interchange/photon-course-60gy.json` ships a synthetic
  60 Gy/30-fx demonstration course;
- pending: committed real-engine parser fixtures per producing system for
  OP-04;
- complete (infrastructure slice): cross-code frozen-case comparison —
  `nctforge compare` and Python `compare_dose_bundles` measure voxelwise
  agreement between two physical dose bundles on the same case (equal
  `case_id`, equivalent grid, same components/unit — all enforced) and emit
  `nctforge.dose-comparison/0.1.0`: per-component and total max/mean/RMS
  absolute difference, normalized difference anchored to the reference
  maximum, and a combined-sigma coverage fraction when both sides state
  uncertainties, with both content hashes and provenance chains bound into
  the record;
- pending: a real OpenMC-vs-independent-code comparison on a frozen case —
  the comparison record exists; it still needs an actual MCNP/PHITS-produced
  candidate bundle (the real-engine acceptance gate).

## Cross-cutting distribution

The accepted distribution boundary is recorded in [ADR
0015](docs/adr/0015-python-and-native-distribution.md): PyPI is the planned
primary scientific-user entry point, Cargo remains the native developer path,
and desktop builds ship as native release artifacts. All three surfaces call the
same Rust contracts.

Implementation status:

- complete: choose the PyO3/maturin mixed-package architecture and prohibit a
  parallel Python dose, geometry, evidence, or qualification engine;
- complete: implement the first bounded Python API over the selected,
  versioned Rust contracts — case generation, verification, and gated loading;
  geometry and ROI inspection; schema-validated manifest and transport-contract
  readers with canonical `to_json` serialization; the honest response-set
  folding gate and backend capability flags — with a 12-test cross-language
  parity suite run against a wheel installed into a clean environment in CI;
- pending: build and smoke-test the supported wheel matrix through TestPyPI;
- pending: review public crate surfaces before enabling crates.io publication;
- pending: produce signed native desktop release artifacts.

## R5 — External validation and adoption

Exit evidence:

- independent reproduction by a researcher outside Avila Labs;
- review by a BNCT physicist;
- comparison with measured phantom or commissioned beam data under a written
  collaboration agreement;
- at least two institutions execute the conformance suite;
- methods manuscript and archival software/data release.

## Deferred beyond the research platform

- patient-specific clinical decisions;
- treatment delivery instructions;
- automated segmentation or contour editing;
- facility commissioning claims;
- regulatory submission;
- optimization involving Avify Dose patent subject matter;
- any claim of clinical equivalence to a certified TPS.
