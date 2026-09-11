# ADR 0029: TENDL-2025 mixed-source candidate — executed and rejected

**Status:** Accepted; candidate executed and rejected

**Date:** 2026-09-11

## Context

ADR 0026 paused response-treatment qualification pending either a controlled
alternative evaluated-data profile or a reviewed independent O-17
calculation. ADR 0028 added the `0.3.0` mixed-selection contract so the first
alternative profile could be assessed. This ADR records the executed outcome.

The candidate pairs the six clean ENDF/B-VIII.1 evaluations with the
TENDL-2025 evaluations for the four nuclides that reject under both
ENDF/B-VIII.1 and JEFF-4.0 (N-15, O-16, O-17, O-18). TENDL was the only
remaining credible published alternative: its informal screening appeared to
clear those four nuclides, and it is the most current general-purpose
evaluation release.

## Decision

Run the full frozen evidence chain on the mixed selection and record the
result as evidence, including gates that could not produce reports.

## Outcome

- All six ENDF/B-VIII.1-selected nuclides reproduce the baseline exactly:
  zero violations, candidate-unreviewed. Per-nuclide substitution behaves as
  ADR 0028 intends.
- All four TENDL-2025-selected nuclides fail the same MT 301 kinematic
  gate as under ENDF/B-VIII.1: N-15 6 violations, O-16 16, O-17 45, O-18 10
  (77 total; 70 in-domain after v0.3 scoping). The earlier informal screening
  reading that suggested a clean TENDL result was an analysis error; the
  screening tapes contain the same violation markers.
- All ten evaluations carry complete photon-production records (315
  inventoried sections, zero format findings), so the failures are
  source-physics findings, not missing-data artifacts.
- The four TENDL evaluations carry duplicated energy-grid points in nearly
  every MF=3 section — a tabulation style NJOY2016.78 tolerates but the
  strict source parsers reject. The independent capture-balance, continuum
  photon-moment, evidence-aware (v0.4), and diagnostic-triage gates are
  therefore not computable for this candidate. This uncomputability is a
  source-format finding, preserved as evidence rather than worked around.
- The H-2 LAW=7 implicit-residual gate does not apply to ENDF/B-VIII.1's
  H-2, which records both (n,2n) products explicitly (NK=2).
- The receipt-bound O-17 attribution reproduces all 43 in-domain findings
  from NJOY's printed energy-balance remainders; they remain queued for
  independent physical validation.
- The verified baseline comparison: 4 baseline rejections, 4 candidate
  rejections, 0 resolved, 0 introduced; `candidate_rejected`.

## Consequences

- No published evaluated neutron library currently satisfies the
  transported-photon KERMA chain for this material. ENDF/B-VIII.1, JEFF-4.0,
  and TENDL-2025 are each rejected with distinct evidence.
- Response qualification remains paused per ADR 0026; the only remaining
  resume condition is a reviewed independent O-17 reaction calculation.
- The candidate-comparison check (`njoy check-candidate-comparison`) and the
  `njoy-candidate-comparison` Avila Core package generalize the external
  evidence loop to any future candidate without requiring the deeper chain
  to be computable.
- The strict tabulation parsers are deliberately unchanged: relaxing them to
  accommodate TENDL's grid style would silently weaken the independent
  checks the chain exists to provide.
- Rejected evidence, the uncomputable gates, and the mixed-selection binding
  are all preserved in the repository record.
