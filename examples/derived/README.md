# Derived-case example: structure-driven material assignment

Demonstrates `nctforge benchmark derive-materials`: ROI masks rasterized from
the verified NF-BNCT-001 RT Structure Set become axis-aligned voxel-box
material regions. Every mapped ROI must equal its bounding box exactly —
non-box ROIs are rejected rather than approximated.

`material-map.json` assigns `CORE` a boron-free tissue variant
(`material-core-unloaded.json`): the benchmark composition with B-10 removed
and its mass fraction folded into N-14. The compensation must stay on a
response-covered nuclide (B-10 or N-14) — changing any uncovered nuclide is
rejected at deck generation because the folded response tables assume base
atom densities. The base material remains the frozen boron-loaded benchmark
material, so the response-set material binding still holds and the derived
case is a physically distinct demonstration — boron dose inside `CORE`
vanishes while the surrounding phantom keeps the nominal B-10 loading.

```text
nctforge benchmark generate /tmp/nf-bnct-001
nctforge benchmark derive-materials \
  --case-root /tmp/nf-bnct-001 \
  --case benchmarks/synthetic/nf-bnct-001/transport/case.json \
  --base-material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --map examples/derived/material-map.json \
  --output-assignment /tmp/nf-derived-assignment.json \
  --output-case /tmp/nf-derived-case.json
```

Then generate or run a deck with `--assignment /tmp/nf-derived-assignment.json`
and the derived case/material pair. Region densities must equal the base
material density until per-region voxel-mass collection lands.

Research only: illustrative composition, not a clinical material model.
