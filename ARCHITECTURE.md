# Architecture

**Status:** Initial direction, 2026-08-31

## Objective

Build a neutral BNCT research and verification platform whose calculation
inputs, physical-dose components, biological assumptions, uncertainty, and
provenance remain independently inspectable.

## One authoritative Rust model

The Rust domain model is authoritative for production interchange, validation,
QA, and evidence generation. GUI, CLI, Python, and future hosted interfaces must
invoke the same implementation rather than reproduce scientific logic.

## Backend neutrality

The application must not pass OpenMC-specific objects beyond
`nctforge-openmc`. Transport adapters consume a `TransportCase` and produce a
`PhysicalDoseBundle`. Backends may prepare and execute a calculation, import an
external result, or both.

Initial adapters are expected to be:

1. analytic synthetic benchmarks;
2. OpenMC preparation, controlled execution, and statepoint collection;
3. generic component-dose import;
4. MCNP and PHITS import when licensing and validation permit.

No adapter may advertise a capability before a committed acceptance case
demonstrates it.

## Scientific layers

1. **Geometry:** patient coordinate frame, voxel affine, structures, materials.
2. **Transport:** neutron and photon histories and reaction estimators.
3. **Physical dose:** macroscopic boron (`D_B`), nitrogen (`D_N`), conventional
   hydrogen/neutron (`D_H`), and photon (`D_gamma`) components under a named
   semantic profile. Contributor reactions remain inspectable.
4. **Boron model:** measured or assumed concentration and spatial distribution.
5. **Biology:** separately versioned CBE, RBE, or isoeffective interpretation.
6. **QA:** DVH, gamma, sensitivities, cross-code and measurement comparisons.
7. **Evidence:** immutable manifest binding every consequential artifact.

A derived biological dose must retain references to the exact physical bundle,
boron model, biological model, and parameter set used to produce it.

## DICOM geometry boundary

`nctforge-dicom` is the sole DICOM-to-domain boundary. It uses a pinned
`dicom-rs` release for Part 10 parsing, then applies NCTForge's own fail-closed
semantic checks. CT order comes from projected Image Position (Patient), not
file order or Instance Number. The core grid uses `[column, row, slice]`, stores
the first voxel centre as its origin, and retains a right-handed orthonormal LPS
direction matrix.

R1 deliberately supports only native signed 16-bit Explicit VR Little Endian CT
with a uniform, unsheared stack and one `CLOSED_PLANAR` polygon per ROI per image
plane. Compressed pixels, enhanced multiframe CT, tilted stacks, and ambiguous
multi-polygon topology fail explicitly until separately specified and tested.
See `docs/adr/0003-dicom-geometry-boundary.md`.

The initial component semantics are fixed by
`docs/adr/0002-macroscopic-dose-semantics.md`. In particular, `D_H` is not
limited to H-1 elastic scattering when that would leave neutron energy
unclassified. Physical-total uncertainty must not assume that component tallies
from shared particle histories are independent.

## GUI process model

egui/eframe is the initial native interface. The GUI thread must never execute
transport. It submits immutable jobs to a worker process or remote executor and
observes structured progress events. A calculation crash must not corrupt the
open case or terminate the viewer.

The first GUI milestone contains linked axial, sagittal, and coronal views for
a synthetic case, window/level controls, RT structure overlays, component-dose
selection, and provenance inspection. Contour editing is explicitly deferred.

Anatomical mapping lives in `nctforge-view`, not egui callbacks. R1 labels views
as axial, coronal, or sagittal only when grid direction is aligned to canonical
DICOM LPS axes. The viewer rejects oblique or permuted grids until their
resampling and labeling conventions have dedicated tests. Screen-edge mappings
and click-to-voxel round trips are tested independently for all three planes.
See `docs/adr/0004-anatomical-view-conventions.md`.

## Data handling

Patient data is denied by default during early development. The repository
ignores common medical-image and transport-output formats. Only deliberately
reviewed synthetic artifacts may enter the public benchmark corpus.

## Qualification

Every result declares one of the bounded qualification states defined by
`nctforge-evidence`. The software must never infer clinical fitness from a
successful calculation or benchmark.

The R1 case manifest binds its coordinate system, grid, DICOM identifiers,
structure truth values, material/source model identifiers, and every CT or
RTSTRUCT input by relative path and SHA-256. Artifact verification rejects path
traversal, symlink escape from the case root, missing files, and hash changes.
The manifest intentionally does not hash itself; archival releases bind the
manifest from a higher-level release or run manifest.
