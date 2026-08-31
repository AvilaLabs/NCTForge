# ADR 0015: Python and Native Distribution Boundary

**Status:** Accepted; packaging implementation pending

**Date:** 2026-08-31

## Context

BNCT research workflows are predominantly assembled in Python, while NCTForge's
validation, geometry, evidence, and transport-neutral contracts are implemented
in Rust. Requiring every scientific user to adopt Rust would unnecessarily limit
use. Reimplementing those contracts in Python, however, would create two
scientific authorities that could disagree on units, validation, hashes, or
qualification state.

The desktop workbench, command-line interface, and future notebooks also have
different distribution needs. A Python wheel is the natural entry point for a
notebook user; a native desktop application should not depend on a Python
environment; and a Rust developer should be able to use Cargo directly.

## Decision

NCTForge will use one authoritative Rust implementation with three supported
access layers:

1. `pip install nctforge` will be the primary scientific-user entry point once
   the first public Python release passes its gates. A mixed Python/Rust package
   will use PyO3 and maturin. Its compiled private extension will call the same
   Rust crates used by the CLI and GUI, while a thin Python package supplies
   ergonomic names, type information, and notebook-oriented helpers.
2. Cargo will remain the native developer and source-build path. The CLI package
   is `nctforge-cli` and installs the `nctforge` executable. Crates.io publication
   remains disabled during the early research phase; a stable Cargo publication
   requires an explicit API, dependency, and license review.
3. The egui workbench will be distributed as native release artifacts. It may
   later be launchable from the Python package, but PyPI will not be the only way
   to obtain the desktop application.

The Python layer must not implement an independent dose engine, geometry
transform, evidence verifier, or qualification state machine. Python-facing
objects serialize to or wrap the versioned Rust contracts. Cross-language tests
must prove equivalent acceptance, rejection, canonical serialization, and
content identity for shared benchmark inputs.

Prebuilt wheels are required for the supported CPython, operating-system, and
architecture matrix so ordinary `pip` users do not need a local Rust toolchain.
Source distributions may still require Rust. The exact stable-ABI strategy will
be selected when the binding surface is known; it will not be assumed early,
especially because free-threaded CPython has separate wheel considerations.

The first Python surface should expose mature, bounded capabilities such as
case verification, geometry inspection, normalized model construction, and
evidence reading. It must not expose a transport action as available before the
same Rust backend capability and evidence gates used by the CLI and GUI pass.
Installing NCTForge will not bundle OpenMC, MCNP, PHITS, nuclear data, or another
external transport system.

## Release gates

The first PyPI release requires:

- a PyO3 crate and mixed-package layout under `bindings/python`;
- a typed public Python API with `py.typed` and generated or checked stubs;
- wheel builds and clean-environment import tests for the supported matrix;
- TestPyPI installation and benchmark smoke tests before production upload;
- parity tests showing Python and Rust produce the same validation outcomes and
  artifact identities; and
- documentation that distinguishes installed NCTForge capabilities from
  separately installed transport backends and nuclear data.

The first crates.io release additionally requires removal of the workspace
`publish = false` boundary through a reviewed change and verification that every
published dependency crate has a stable public surface.

## Consequences

- Python users get a conventional scientific workflow without a second physics
  implementation.
- Rust remains an implementation choice rather than an adoption requirement.
- Native CLI and GUI users do not inherit a Python runtime dependency.
- Release engineering must produce and test several platform wheels and native
  application artifacts.
- `pip install nctforge` and crates.io installation are goals, not claims about
  the current unreleased repository.

## References

- [Maturin user guide](https://www.maturin.rs/)
- [Maturin mixed-project layout](https://www.maturin.rs/project_layout.html)
- [PyO3 building and distribution](https://pyo3.rs/main/building-and-distribution)
