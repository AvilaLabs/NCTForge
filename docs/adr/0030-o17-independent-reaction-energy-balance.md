# ADR 0030: O-17 independent reaction energy-balance integrator

**Status:** Accepted; evidence produced, unreviewed

**Date:** 2026-09-12

## Context

ADR 0026 paused JEFF-4.0 response qualification pending either a controlled
alternative evaluated-data profile or a reviewed independent O-17 reaction
calculation. ADR 0029 exhausted the first condition: ENDF/B-VIII.1, JEFF-4.0,
and TENDL-2025 are all rejected under the controlled chain, and TENDL's
duplicate File 3 grid points make the deeper independent gates uncomputable
for it. The processor survey
(`docs/research/INDEPENDENT_KERMA_PROCESSOR_SURVEY.md`) established that no
off-the-shelf processor computes KERMA independently of NJOY's HEATR.

The remaining path is an in-house calculation that integrates the source
evaluation's File 3/File 6 data directly. JEFF-4.0 O-17's representation is
bounded and uniform: 28 shared File 3/File 6 MTs, all `LCT=3`, every product
`LAW=1`, with `LANG` 1 or 2 and `LEP` 1 or 2 only.

## Decision

Add schema `openbnct.endf-reaction-energy-balance/0.1.0` and two CLI
boundaries:

- `njoy calculate-reaction-energy-balance`; and
- `njoy verify-reaction-energy-balance`.

The calculator parses the selected evaluation's File 3 cross sections and
File 6 product distributions without calling NJOY or reading processed
heating data, and forms per-reaction remainders

```text
h_MT(E) = sigma_MT(E) * [ (E + QM_MT) - sum_p yield_p(E) * ebar_p,lab(E) ]
```

at each in-domain finding energy. Binding requirements mirror the existing
gates: the selection, attribution, domain-aware report, and execution receipt
must share one evidence chain; the evaluation bytes are re-hashed; the
execution directory is re-verified against its external receipt; and the
printed `ebal`/`ebar` values are re-parsed from the receipt-bound processor
report.

The physics conventions implemented are enumerated in
`docs/research/O17_REACTION_ENERGY_BALANCE_INTEGRATOR.md` — mass-difference
Q selection, the `LCT=3` frame rule (Kalbach-Mann products are CM-frame,
isotropic products are lab-frame), the ENDF-6 Kalbach-Mann slope
systematics for `NA=1`, union-grid incident spectrum interpolation, and the
right-continuous duplicate-grid-point rule. Each is recorded so a reviewer
can accept or dispute it explicitly.

The report's only qualifications are
`source_remainders_computed_unreviewed` and
`source_remainders_partially_computable`. Its evidence scope is fixed at
`independent_source_calculation_unreviewed` and its finding disposition at
`retained_for_independent_physical_validation` as schema invariants.

## Frozen result

Artifact `jeff40-o17-endf-reaction-energy-balance.json` binds the same
execution receipt chain as the ADR 0026 attribution and carries SHA-256
`9a102394d11a25a90928b0cebd6b5aee6475704dbbe8ef68c6f52701e0711a55`.

- 43/43 in-domain samples computed; none partially computable.
- The independent remainder sum reproduces NJOY's printed `ebal` sum and
  the final `MT301` excess within `1.7e-2` at every finding energy; 17 of
  43 samples match within the default `5e-3` tolerance.
- Two difference classes are preserved as evidence rather than resolved:
  NJOY's processor-internal MT=91 continuum-inelastic neutron convention
  (product `ebar` differences up to 51%) and its internal `ebar`
  reconstruction conventions (per-product differences up to ~1% in either
  direction, amplified to ~10% on reactions whose remainder is a small
  fraction of available energy).

The result indicates the MT=301 energy non-conservation is present in the
evaluated File 6 product accounting itself — it is not introduced by NJOY's
MT=301 assembly. That is a measured agreement, not a validation.

## Consequences

- The independent calculation ADR 0026 contemplated now exists as runnable,
  verifiable evidence; the review requirement is unchanged and the
  response-treatment path stays paused until the conventions and the
  residual difference classes are reviewed.
- The artifact gives reviewers a concrete, per-reaction, per-energy
  decomposition of the O-17 blocker instead of a monolithic final-table
  warning.
- `verify-reaction-energy-balance` makes the artifact self-checking: any
  checkout with the bound inputs regenerates it exactly.
- No suitability verdicts, ADR 0025/0026 queues, or response-table states
  change.

## Related decisions

- [ADR 0026: O-17 processor energy-balance attribution](0026-o17-processor-energy-balance-attribution.md)
- [ADR 0028: Mixed evaluated neutron source selections](0028-mixed-evaluated-neutron-source-selections.md)
- [ADR 0029: TENDL-2025 mixed-source candidate](0029-tendl2025-mixed-source-candidate.md)
