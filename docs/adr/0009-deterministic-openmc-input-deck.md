# ADR 0009: Deterministic OpenMC 0.16 Input Deck

**Status:** Accepted and implemented for input generation

**Date:** 2026-08-31

## Context

The first NCTForge OpenMC path must translate the transport-neutral case into
backend input without weakening the content bindings established for material,
source, component semantics, neutron responses, and nuclear data. OpenMC's
`EnergyFunctionFilter` evaluates to zero outside its supplied energy domain, so
checking only the source energy would permit a plausible but silently
under-scored component result.

Input generation is also a provenance boundary. Relying on mutable defaults,
implicit isotope expansion, unrecorded particles-per-batch, or a Python runtime
would make two nominally identical runs harder to compare and audit.

## Decision

NCTForge writes `geometry.xml`, `materials.xml`, `settings.xml`, and
`tallies.xml` directly in Rust for OpenMC `0.16.0` at source commit
`617d35a5063c57796b43428bc401e627d2011046`. The writer uses stable IDs,
ordering, float formatting, indentation, and terminal newlines. The OpenMC
Python package is not a runtime dependency.

Generation accepts the exact JSON bytes for the component profile, explicit
material, fixed source, reviewed response set, nuclear-data manifest, and
execution profile. It hashes those bytes before parsing and rejects any
response-set reference that does not match the observed artifact. It then
verifies every case-selected HDF5 file and the `cross_sections.xml` mapping
against the nuclear-data manifest before emitting XML.

The nuclear-data inspector records the first and last incident-neutron energy
at every available temperature. For the selected material temperature,
NCTForge reproduces OpenMC's common transport interval as the maximum lower
bound and minimum upper bound across all selected neutron tables. The reviewed
response set must cover that entire interval. This prevents the zero-outside-
domain behavior of `EnergyFunctionFilter` from becoming an accepted dose bias.

The first geometry profile is deliberately narrow:

- identity-oriented DICOM LPS axes, converted from millimetres to centimetres;
- one homogeneous material cell bounded by six vacuum planes;
- a regular scoring mesh exactly coincident with the voxel boundaries; and
- a uniform source plane represented by an OpenMC box with equal lower and
  upper z coordinates, strictly inside the geometry.

The frozen smoke settings use fixed-source, continuous-energy, history-based
transport; five active batches; an explicit seed and OpenMC's `152917` default
stride; coupled photon transport; atomic relaxation; local electron energy
deposition (`led`); probability tables; nearest-temperature selection within
`0.5 K`; no survival biasing; no temperature multipole treatment; a final
statepoint; and no sourcepoint or ASCII tally file.

The tally ledger is fixed by ID:

| IDs | Quantity | Estimator | Collection rule |
| --- | --- | --- | --- |
| 1–3 | B, N, and residual-neutron response folds | track length | divide `Gy cm3/source` by voxel volume |
| 4 | neutron-only heating audit | track length | convert eV to J and divide by voxel mass |
| 5 | photon component heating | collision | convert eV to J and divide by voxel mass |
| 6 | coupled physical-total heating | collision | convert eV to J and divide by voxel mass |
| 7–8 | B-10 MT=107 and N-14 MT=103 reaction rates | track length | retain reactions/source |
| 9–10 | neutron and photon diagnostic fluence | track length | divide track length by voxel volume |
| 11–12 | neutron and photon surface current | analog | retain particle current/source |

The generated `nctforge-input-manifest.json` binds all input identities, run
controls, scoring bounds, voxel volume and mass, tally meanings and collection
normalizations, and SHA-256 for every OpenMC XML file.

## Acceptance boundary

The standalone generator is implemented and byte-regression tested. All four
XML documents were also parsed successfully with the exact OpenMC `0.16.0`
Python package during development. This does not enable the transport adapter's
preparation, execution, or import capability flags.

Those flags remain false until an official ENDF/B-VIII.1 case manifest and an
independently reviewed response table exist, a real OpenMC executable passes a
controlled smoke run, and statepoint collection verifies every tally shape,
unit conversion, batch count, and identity recorded here.

## Consequences

- A caller cannot generate an accepted deck from a merely plausible manifest;
  the selected data files and cross-section mappings must exist and hash-match.
- The first backend profile fails closed on oblique or permuted geometry rather
  than silently changing patient axes.
- The OpenMC adapter remains honest about its current capabilities while its
  deterministic preparation core can be reviewed independently.
- Later geometry, source, physics, or OpenMC-version profiles require explicit
  versioned changes and new byte fixtures.

## Primary sources

- [OpenMC 0.16.0 settings format](https://docs.openmc.org/en/v0.16.0/io_formats/settings.html)
- [OpenMC 0.16.0 tallies format](https://docs.openmc.org/en/v0.16.0/io_formats/tallies.html)
- [OpenMC 0.16.0 geometry format](https://docs.openmc.org/en/v0.16.0/io_formats/geometry.html)
- [OpenMC 0.16.0 materials format](https://docs.openmc.org/en/v0.16.0/io_formats/materials.html)
- [OpenMC energy-function filter](https://docs.openmc.org/en/v0.16.0/pythonapi/generated/openmc.EnergyFunctionFilter.html)
- [OpenMC 0.16.0 selected-table energy bounds](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/simulation.cpp)
- [OpenMC 0.16.0 estimator selection](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/tallies/tally.cpp)
