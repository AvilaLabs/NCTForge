# Component-dose interchange example

`phits-synthetic-dose.json` is a `openbnct.component-dose-interchange/0.1.0`
document: a transport-neutral record an external transport pipeline (MCNP,
PHITS, Geant4, or a custom tool) emits so OpenBNCT can import a
component-resolved voxel dose without losing meaning.

This fixture is **synthetic** — analytic stand-in values on the NF-BNCT-001
grid labeled `system: phits` for demonstration; it is not PHITS output.

```text
openbnct import interchange \
  --file examples/interchange/phits-synthetic-dose.json \
  --output imported-dose.json
```

The resulting `openbnct.physical-dose-bundle/0.2.0` flows through `dvh`,
`metrics`, `bio apply`, `irradiation-time`, evidence bundles, and the GUI
exactly like an OpenMC-collected bundle.

## Document semantics

- `geometry` is the transport-neutral grid (shape, spacing, origin,
  direction); values are in grid order `i + nx·j + nx·ny·k`.
- `producer` names the external system, its version, and how tallies were
  normalized/folded.
- `components` must carry exactly the four physical components (`boron`,
  `nitrogen`, `hydrogen`, `photon`) once each, in a consistent unit
  (`gray_per_source_particle` or `gray`), with optional per-voxel
  1-sigma absolute uncertainties.
- `total.mode`:
  - `dedicated` — the producer tallied the physical total itself; optional
    sigmas import with `dedicated_estimator` provenance;
  - `component_sum` — the importer sums components and records
    `unavailable` total uncertainty, because component covariance is the
    producer's domain knowledge and cannot be claimed on import.
- `component_profile`/`response_set` are producer-declared content
  references; when absent the import binds the document's own SHA-256 so
  provenance never dangles. The bundle's `provenance_id` records
  `interchange:<system>:sha256:<document hash>`.

Research only: imported results carry the same qualification boundaries as
any OpenBNCT dose — no clinical or commissioning claim is implied.

## External-dose interchange (single-dose course)

`photon-course-60gy.json` is a `openbnct.external-dose/0.1.0` document: one
absolute absorbed-dose field plus the fractionation it was delivered in —
the shape a photon or hadron course contributes to a combined-treatment
research evaluation. This fixture is **synthetic** (60 Gy uniform over the
NF-BNCT-001 grid, 30 fractions) and stands in for a real external course.

```text
openbnct import dose \
  --file examples/interchange/photon-course-60gy.json \
  --output external-course.json
```

The imported bundle converts to a biological quantity and combines with a
photon-isoeffective BNCT result:

```text
# BED or EQD2 (default eqd2); region α/β overrides take matching masks.
openbnct bio bed --dose external-course.json --alpha-beta 3.0 \
  --output external-eqd2.json

# Adds the external EQD2 field to a weighted_eqd2 biological bundle —
# the only compatible combination; everything else is rejected.
openbnct bio combine \
  --primary biological-eqd2.json --external external-eqd2.json \
  --assumption "full-repair additive EQD2; independent courses" \
  --output combined-eqd2.json
```

When the external course sits on a different (axis-aligned) grid,
`--resample trilinear` on `bio combine` co-registers it onto the primary
grid by trilinear interpolation at voxel centers; a target outside the
external extent rejects rather than silently scoring zero. The combined
record is `openbnct.combined-dose/0.1.0` with both input content hashes,
both provenance chains, the resampling declaration, and the declared
additivity assumption — no clinical or equivalence claim.
