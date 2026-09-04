# ADR 0026: O-17 processor energy-balance attribution

- Status: Accepted
- Date: 2026-09-03
- Decision owners: NCTForge maintainers
- Scope: NF-BNCT-001 JEFF-4.0 O-17 transported-photon KERMA diagnostics

## Context

ADR 0025 leaves one active reaction-diagnostic queue: 43 in-domain O-17 high
MT=301 findings. The exact O-17 evaluation supplies transported-photon records
and has no separate source-format or rejecting processor-data finding, so the
C-13/O-18 missing-source disposition does not apply.

An initial attempt to reuse the MF=6/MT=102 capture-balance calculator failed
with `invalid ENDF tabulation`. O-17's capture representation contains 100
discrete photon lines followed by a continuum, while its other contributing
reactions include product distributions and reference-frame handling outside
that calculator's intentionally narrow contract. Extending that source-level
calculation across every contributing reaction would be a new scientific
implementation, not a safe parser adjustment.

The pinned NJOY2016.78 source nevertheless exposes a testable processor
accounting mechanism. In commit
`71a76bc6345fa15f36bacc816ae7900714345d97`, `src/heatr.f90`:

- calculates `ebal6` for the final File 6 product of applicable reactions
  (lines 1458-1461);
- adds `h + ebal6` to total and requested partial heating factors (lines
  1470-1471 and 1506-1512); and
- computes the kinematic bounds separately from `h`, beginning at line 1515.

That suggests the final MT=301 excess can be compared with the sum of the
reaction-level `ebal` values printed by the same controlled run. Such a check
can explain processor behavior, but cannot validate the evaluated reaction
physics independently.

## Decision

Add `nctforge.njoy-energy-balance-attribution/0.1.0` and two CLI boundaries:

- `njoy attribute-energy-balance`; and
- `njoy verify-energy-balance-attribution`.

The attribution must:

1. verify the complete execution directory against its external receipt;
2. bind the exact domain-aware report and its closed 20 MeV interval;
3. reproduce the full final-table violation sequence exactly;
4. collect every printed File 6 `ebal` contribution at each in-domain receipt
   energy;
5. compare their sum with `MT301 - kinematic maximum` and independently check
   that MT=443 equals that maximum within a declared print tolerance; and
6. preserve every finding as requiring independent physical validation.

The default relative tolerance is `2e-3`. NJOY prints these diagnostic values
to five significant digits, and sums can include cancellation across reactions.
The tolerance is part of the report, bounded to at most `5e-3`, and reapplied
during regeneration.

A successful qualification is named
`processor_accounting_mechanism_attributed_physical_validation_required`.
Its evidence scope is `processor_internal_print_accounting_only`, its finding
disposition is `retained_for_independent_physical_validation`, and its waived
count must be zero. These are schema invariants rather than narrative caveats.

The immutable evidence-aware v0.4 suitability and ADR 0025 triage are not
changed. Processor attribution alone cannot clear their independent diagnostic
queue.

## Frozen result

The JEFF-4.0 O-17 artifact binds the 701,234-byte processor report with SHA-256
`e7ec927a4d7faeb49a2f6c7cc89c0a22c322818252763899b950e41d39f38e61`.

| Observation | Result |
| --- | ---: |
| Full-evaluation O-17 findings | 45 |
| In-domain findings | 43 |
| Above-domain findings | 2 |
| In-domain findings attributed | 43 |
| Failed attribution samples | 0 |
| Remainder-bearing processor tables | 22 |
| Reaction MTs contributing in domain | 15 |
| Maximum remainder/excess relative difference | `3.721276397804447e-4` |
| Maximum MT=443/kinematic-maximum relative difference | `0` |
| Findings still requiring physical validation | 43 |
| Findings waived | 0 |

At thermal energy, MT=107 alone supplies the printed remainder: `9.3472e6`
eV-barns versus a final MT=301 excess of `9.3480e6` eV-barns. At 10 MeV, six
positive and negative reaction contributions sum to `1.2252915e6` eV-barns
versus a final excess of `1.2253e6` eV-barns. The all-energy result therefore
reflects reaction accounting rather than a single constant offset.

The frozen attribution JSON has SHA-256
`1c38d1e5fb6a6b26e5d99fc1505bd3aa15b25a2b01116e47aed5566381e093d8`.

## Continuation decision

Pause autonomous work on the JEFF-4.0 response-treatment qualification after
this slice. The processor mechanism is now reproducible, but the scientific
blocker is not removed:

- O-17 still needs an independently reviewed reaction-level energy-balance
  calculation or an independent trusted processing comparison;
- C-13 and O-18 still lack a supported transported-photon production source;
  and
- N-15 remains rejected by its independent capture-balance gate.

Resume this path when there is either a controlled alternative evaluated-data
profile that addresses those source conditions, or sufficient nuclear-data
expertise to specify and review the broader O-17 integrator. Generating response
tables before then would convert unexplained evidence into false progress.

This pauses one R2 evidence path; it does not imply that NCTForge's geometry,
workflow, or interoperability work is invalid.

## Consequences

- O-17's 43 findings now have a deterministic processor-level causal trace.
- No response-table status changes and Core should continue to fail the
  existing 43/zero and rejected/candidate requirements.
- Future physical work can inspect exact per-energy, per-reaction contributions
  instead of restarting from a monolithic final-table warning.
- The failed capture-parser reuse is retained as evidence that a broader
  source-level implementation needs an explicit scientific scope.

## Related decisions

- [ADR 0019: Independent MF=6 capture photon balance](0019-independent-mf6-capture-photon-balance.md)
- [ADR 0020: Content-bound transport-domain suitability](0020-content-bound-transport-domain-suitability.md)
- [ADR 0023: Reaction-evidence-aware transported-photon suitability](0023-reaction-evidence-aware-suitability.md)
- [ADR 0024: Avila Core evidence loop](0024-avila-core-evidence-loop.md)
- [ADR 0025: Diagnostic triage of remaining NJOY findings](0025-diagnostic-triage-of-remaining-njoy-findings.md)
