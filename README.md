# OpenBNCT

[![CI](https://github.com/AvilaLabs/OpenBNCT/actions/workflows/ci.yml/badge.svg)](https://github.com/AvilaLabs/OpenBNCT/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status: early research](https://img.shields.io/badge/status-early_research-orange.svg)](ROADMAP.md)
[![Clinical use: not validated](https://img.shields.io/badge/clinical_use-not_validated-red.svg)](DISCLAIMER.md)

**English** | [日本語](README.ja.md)

OpenBNCT is an open, transport-neutral research workbench for boron neutron
capture therapy (BNCT) dosimetry and independent verification. It is built in
Rust around one idea: a dose result should carry its provenance, uncertainty,
and qualification with it — and a claim should only be as strong as the
evidence bound to it.

*(Formerly NCTForge — internal crate names, the `nctforge` CLI/Python package,
and `nctforge.*` schema identifiers retain the original namespace.)*

OpenMC is the first transport backend behind a transport-neutral boundary.
MCNP, PHITS, and other external results import through a published
component-dose interchange contract — nothing about the platform requires
bundling those systems.

> **OpenBNCT is research software.** It is not a medical device, not a dose
> calculator for clinical use, and not commissioned for any treatment
> facility. See [DISCLAIMER.md](DISCLAIMER.md).

## What it does today

- **Frozen synthetic benchmark** — `NF-BNCT-001`: synthetic DICOM CT +
  RTSTRUCT, transport-neutral `case.json`, explicit material and source
  contracts, all inputs bound by SHA-256.
- **OpenMC backend** — deterministic deck generation, controlled execution,
  statepoint collection into versioned physical-dose bundles
  (`nctforge openmc generate|run|collect|evaluate`).
- **Verified candidate reference** — three frozen-seed OpenMC runs at 600M
  histories each passed every predeclared acceptance gate: ROI precision,
  per-voxel precision (photon median RSE ≈ 2.7%), estimator comparisons
  (291), and chi-square seed consistency (378). Reports:
  [`openmc-acceptance-report-300M.json`](benchmarks/synthetic/nf-bnct-001/transport/openmc-acceptance-report-300M.json)
  (failed photon gate — kept in the record) and
  [`openmc-acceptance-report-600M.json`](benchmarks/synthetic/nf-bnct-001/transport/openmc-acceptance-report-600M.json)
  (passed).
- **Component dosimetry** — four-component physical dose (boron, nitrogen,
  hydrogen, photon) with absolute voxel uncertainty; DVH, `D_x`/`V_x`/EUD
  region metrics, mask operations, CT-threshold regions, NIfTI I/O.
- **Biological interpretation** — versioned model families (weighted,
  photon-isoeffective, fractionation), sensitivity sweeps, endpoint models
  (logistic/probit TCP/NTCP, voxel-Poisson, UTCP), BED/EQD2 conversion —
  always a distinct layer from physical dose.
- **Transport neutrality** — `nctforge.component-dose-interchange/0.1.0`
  import contract; MCNP meshtal and PHITS output adapters; MCNP input-deck
  export; external-dose import and combined-treatment evaluation; a
  `nctforge compare` cross-code comparison record.
- **Facility beam descriptions** — versioned `nctforge.beam-description/0.1.0`
  documents (spectrum, divergence, aperture, normalization, cited provenance)
  that bind onto a transport case (`nctforge beam info|list|bind`); the
  `beams/` registry ships the FiR 1 K63 literature beam.
- **Beam quality characterization** — `nctforge beam qa` emits versioned
  `nctforge.beam-quality/0.1.0` reports: TECDOC-1223-style in-air group
  fluences and current-to-fluence ratio computed exactly from the declared
  source, optional in-phantom advantage-depth/ratio and peak therapeutic
  ratio from a dose bundle, and per-metric comparison against published
  reference values.
- **Three surfaces, one implementation** — CLI (`nctforge`), a native egui
  workbench (integrity-gated tri-planar viewer, dose wash, DVH/metrics,
  plan workspace, source positioning), and a bounded Python package —
  all calling the same Rust contracts.

## Quick start

```text
cargo build --workspace                  # CLI + libraries
cargo test --workspace                   # full suite incl. conformance
cargo run --bin nctforge-gui             # desktop workbench
```

The Python package builds one `abi3` wheel per platform (Python ≥ 3.10):

```text
pip install 'maturin>=1.7,<2'
maturin build --manifest-path bindings/python/Cargo.toml
```

## Where the evidence lives

- [`docs/USAGE.md`](docs/USAGE.md) — detailed command and workflow reference
- [`ROADMAP.md`](ROADMAP.md) — evidence-gated status, per-milestone detail
- [`benchmarks/synthetic/nf-bnct-001/SPECIFICATION.md`](benchmarks/synthetic/nf-bnct-001/SPECIFICATION.md)
  — the frozen case and its predeclared acceptance gates
- [`conformance/`](conformance/) — public fixture suites: interchange,
  biological models, endpoints, and MCNP/PHITS adapters
- [`docs/adr/`](docs/adr/) — architecture decision records
- [`docs/research/TECHNICAL_BASELINE.md`](docs/research/TECHNICAL_BASELINE.md)
  — scientific rationale

## Honest status

- The candidate reference passed all **statistical** acceptance gates. Under
  the case specification it is not promoted to a *reference output* until a
  separately implemented transport path (Geant4, or licensed MCNP/PHITS
  produced by a licensed user) reproduces the frozen case.
- The MCNP/PHITS adapters are verified against documented-format fixtures;
  real-engine acceptance remains an open gate.
- Nothing here claims clinical qualification, clinical equivalence,
  commissioning, or regulatory suitability.

## License and use boundary

Code is licensed under Apache-2.0. Synthetic benchmark data will receive an
explicit data license before its first release.

The repository must not implement Avify Dose patent subject matter without a
documented intellectual-property review. See
[docs/IP_BOUNDARY.md](docs/IP_BOUNDARY.md).
