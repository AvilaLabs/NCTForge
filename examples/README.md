# Examples

End-to-end examples are added only when they are executable in CI and use
the reviewed synthetic benchmark corpus.

## `biological/`

- `fixed-component-weights-model-v1.json` — a demonstration
  `nctforge.biological-model/0.1.0` with fixed per-component effectiveness
  weights and a tumor-like boron override inside the `core` region. The
  weights are illustrative research values only; they carry no clinical CBE
  or RBE claim and are not part of the frozen NF-BNCT-001 benchmark outputs.
- `core-region-mask.json` — the 512-voxel mask selecting the
  acceptance-contract `CORE` box (voxel centers within ±20 mm on every axis)
  on the benchmark's 40×40×40, 5 mm grid.

Usage is documented under "Biological interpretation and dose-volume
histograms" in the repository README.
