# ADR 0025: Diagnostic triage of remaining NJOY findings

- Status: Accepted
- Date: 2026-09-03
- Decision owners: NCTForge maintainers
- Scope: NF-BNCT-001 JEFF-4.0 transported-photon KERMA investigation

## Context

The evidence-aware v0.4 assessment preserves 102 in-domain NJOY kinematic
findings across C-13, O-17, and O-18. Treating that number as one undifferentiated
queue obscures an important distinction already established by the bound
source inventory:

- C-13 and O-18 use `local_deposition_fallback` because their exact evaluations
  contain no supported MF=6/12/13/14/15 photon-production source. Their runs
  are already rejected on that independent source-data condition.
- O-17 has transported-photon source records and no separate nonkinematic
  rejection in this layer. Its 43 findings still require reaction-level
  diagnosis.

A triage layer can identify the next useful work without claiming a numerical
cause for any finding or weakening the existing rejection.

## Decision

Add `nctforge.njoy-diagnostic-triage/0.1.0` as a derived, immutable report over
the exact evidence-aware v0.4 and domain-aware v0.3 reports.

A run is `blocked_by_missing_photon_production_source` only when all of the
following are true:

1. HEATR used `local_deposition_fallback`;
2. the source-format finding list is empty;
3. exactly one rejecting nonkinematic finding exists; and
4. that finding is `no_photon_production_local_fallback`.

Findings on such a run are counted as `source_data_blocked`. Findings on any
other run remain `independent_diagnostic_required`. The original count must
equal the sum of those two partitions. No finding is deleted, cleared, waived,
or described as numerically explained.

The report qualification is
`independent_reaction_diagnostic_queue_clear_unreviewed` only when the
independent queue is zero. This status is separate from the response
qualification: an empty diagnostic queue cannot turn an already rejected
response candidate into an acceptable one.

NCTForge also exposes `njoy check-diagnostic-triage`, which re-verifies the
complete seven-report evidence chain and emits a compact deterministic result
for external orchestration.

## Frozen result

| Partition | Findings | Runs |
| --- | ---: | ---: |
| Original in-domain findings | 102 | 3 |
| Source-data-blocked | 59 | 2 (C-13, O-18) |
| Independent diagnostics required | 43 | 1 (O-17) |

The response remains `transported_photon_kerma_rejected`, and the triage state
is `independent_reaction_diagnostics_required`.

Avila Core revision `51e2b59` binds the closed response vocabulary and applies
an explicit categorical requirement. A fresh run over NCTForge revision
`0629ba8` verifies all 8 artifacts and 10 evidence records, reproduces the
three committed claims, and returns:

- `FAIL / bounded.le.exceeds` for 43 findings against a limit of zero; and
- `FAIL / categorical.equals.mismatch` for observed
  `transported_photon_kerma_rejected` against required
  `transported_photon_kerma_candidate_unreviewed`.

The checker exits successfully because scientific rejection is a valid result,
not an execution failure.

## Consequences

- The next bounded physics task is the 43-finding O-17 reaction investigation.
- C-13 and O-18 remain visible and rejected, but further kinematic attribution
  is not on the immediate critical path unless a different photon source is
  supplied.
- The Core gate now checks both the diagnostic count and response category;
  neither can silently stand in for the other.
- This reduces the active investigation queue by 59 findings without claiming
  that those findings were solved.

## Evidence

- Diagnostic-triage report SHA-256:
  `6ba92bce735cf290dd3dbe3e068ceff1e25cbc1869b21d5ecd64db8b8d206020`
- Compact checker result SHA-256:
  `aff141e8786f8c7cd4729e6a0a7f29ecb5c0db5bdabab7636d633db789b3cdf0`
- Core compiled snapshot SHA-256:
  `7c44c22e634c6e65349ba075c97c206a838b7a7040b25a9f7954e102235624a3`
- Core campaign SHA-256:
  `5feff819d1abe6abeb6ffdd9e691bc3193f9cc6d45be690267a2133f97a7c417`

## Related decisions

- [ADR 0017: Source-aware photon-production suitability](0017-source-aware-photon-production-suitability.md)
- [ADR 0020: Content-bound transport-domain suitability](0020-content-bound-transport-domain-suitability.md)
- [ADR 0023: Reaction-evidence-aware transported-photon suitability](0023-reaction-evidence-aware-suitability.md)
- [ADR 0024: Avila Core evidence loop](0024-avila-core-evidence-loop.md)
