# Synthetic benchmark corpus

Only deliberately generated, non-patient data may be committed here.

Every benchmark must eventually include:

- generator source and version;
- coordinate-frame and unit definitions;
- expected physical components and uncertainty;
- reference method and tolerances;
- provenance hashes;
- explicit data license;
- known limitations and qualification boundary.

Binary benchmark data are generated on demand and are not committed by default.
The generator, frozen specification, independent oracle, and rejection tests
are source-controlled.

## Specified cases

- [`NF-BNCT-001`](nf-bnct-001/SPECIFICATION.md) freezes the first synthetic
  DICOM geometry, macroscopic material, source, dose semantics, uncertainty
  rules, and preregistered acceptance gates. Geometry and source are ready for
  implementation; KERMA response tables and reference outputs are intentionally
  not yet qualified.

Generate and verify its DICOM geometry inputs with:

```text
cargo run --bin nctforge -- benchmark generate /tmp/nf-bnct-001
cargo run --bin nctforge -- benchmark verify /tmp/nf-bnct-001
```
