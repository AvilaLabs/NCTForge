# Cross-code reproduction recipe — `nf-bnct-001`

The benchmark's reference-output promotion gate requires a separately
implemented transport path to reproduce the frozen case and pass its
predeclared acceptance gates. This document is the operator recipe for
producing that independent result with licensed MCNP (or PHITS via the
analogous `import phits` path), ending in a hash-bound dose-comparison
record.

OpenBNCT ships no MCNP or PHITS executable and makes no claim about them.
Everything below runs on artifacts already in the repository; the licensed
engine is the operator's step.

## 1. Export the transport deck

```text
openbnct export mcnp \
  --case benchmarks/synthetic/nf-bnct-001/transport/case.json \
  --xs-suffix 80c \
  --seed 42 \
  --output nf-bnct-001.i
```

The deck carries the case grid as an `RPP` box, `M` cards from the
declared nuclide mass fractions, the plane source as `SDEF`, and `FMESH`
neutron/photon flux tallies on the 40³ scoring mesh. `--xs-suffix` names
the operator's own data tables — OpenBNCT never invents a library; the
choice is recorded in the deck header. Raise `NPS` for production runs
(the emitted default is a smoke size).

Component-dose folding is deliberately absent from the deck — see step 3.

## 2. Run MCNP

Execute the deck with a licensed MCNP installation and your `xsdir`
library. The meshtal output carries the per-voxel flux tallies.

## 3. Fold components

The four physical components are not native MCNP tallies — each is a
declared folding of flux against the published response set
(`benchmarks/synthetic/nf-bnct-001/transport/provenance/neutron-response-set.json`):

- **boron** — B-10(n,α) response
- **nitrogen** — N-14(n,p) response
- **hydrogen** — recoil-proton response
- **photon** — coupled photon heating

Fold each response into a meshtal column per component (one `Result` /
`Rel Error` block per tally, all four on the shared 40³ mesh). This
folding is the external pipeline's declared step — the record you produce
must state it in `--normalization`.

## 4. Import into the interchange

```text
openbnct import mcnp \
  --case-id nf-bnct-001 \
  --unit gray_per_source_particle \
  --normalization "per source neutron; response-set fold rX" \
  --component boron=meshtal:4 \
  --component nitrogen=meshtal:14 \
  --component hydrogen=meshtal:24 \
  --component photon=meshtal:34 \
  --output mcnp-dose.json
```

Relative errors import as absolute per-voxel sigmas. The resulting bundle
is an ordinary `openbnct.physical-dose-bundle/0.2.0` whose provenance
binds the meshtal content hash.

## 5. Compare against the OpenMC candidate

```text
openbnct compare \
  --reference openmc-dose.json \
  --candidate mcnp-dose.json \
  --sigma-level 2 --output dose-comparison.json
```

The record reports max/mean/RMS absolute difference, a normalized
difference anchored to the reference maximum, and the within-sigma voxel
fraction per component — a measured agreement statement, not an
equivalence claim.

## 6. PHITS variant

The same loop exists for PHITS `[t-deposit]` xyz-mesh output (ANGEL
format). The operator runs their own PHITS input with an `axis = xy`
deposit tally per component on the shared mesh; each output file plus
its `FILE_err.out` sibling carries values and relative errors:

```text
openbnct import phits \
  --case-id nf-bnct-001 \
  --unit gray_per_source_particle \
  --normalization "per-source-particle deposit; no response folding" \
  --producer-version "PHITS-3.33" \
  --component boron=dep-boron.out \
  --component nitrogen=dep-nitrogen.out \
  --component hydrogen=dep-hydrogen.out \
  --component photon=dep-photon.out \
  --output phits-dose.json
```

Two format details worth noting for deck authors: the tally echo
declares bin *edges* (`xmin`/`xmax`/`nx`), while the bundle's
`origin_mm` stores voxel *centers* — a PHITS mesh of −10…+10 cm with
`nx = 40` reproduces the benchmark's −97.5 mm origin exactly. Each
z-slice is one `#newpage:` page of x-fastest `(x-lo, x-hi, y-lo, y-hi,
value)` rows.

## Verified state

As of the recorded commit, the import-and-compare half of this recipe is
verified end-to-end at benchmark scale for **both** external formats: a
component meshtal and a four-file PHITS `t-deposit` set were each
synthesized from a completed 140M-history OpenMC run (64,000 voxels,
four components), re-imported, and compared with 100% of voxels within
2σ (max |Δ| ≈ 5e-20 Gy/source, physical total ≈ 1.6e-16). The unverified
step is the licensed-engine run itself — that remains the open
acceptance gate this recipe exists to satisfy.
