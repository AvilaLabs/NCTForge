# Component-dose interchange example

`phits-synthetic-dose.json` is a `nctforge.component-dose-interchange/0.1.0`
document: a transport-neutral record an external transport pipeline (MCNP,
PHITS, Geant4, or a custom tool) emits so NCTForge can import a
component-resolved voxel dose without losing meaning.

This fixture is **synthetic** — analytic stand-in values on the NF-BNCT-001
grid labeled `system: phits` for demonstration; it is not PHITS output.

```text
nctforge import interchange \
  --file examples/interchange/phits-synthetic-dose.json \
  --output imported-dose.json
```

The resulting `nctforge.physical-dose-bundle/0.2.0` flows through `dvh`,
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
any NCTForge dose — no clinical or commissioning claim is implied.
