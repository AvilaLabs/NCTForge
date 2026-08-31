# ADR 0023: Reaction-Evidence-Aware Transported-Photon Suitability

**Status:** Accepted and implemented; H-2 reclassified, candidate still rejected

**Date:** 2026-08-31

## Context

The immutable JEFF-4.0 domain-aware v0.3 report rejects C-13, H-2, O-17, and
O-18 after limiting NJOY's complete evaluation diagnostics to the exact common
20 MeV OpenMC transport domain. It does not consume either of the later
reaction-level investigations:

- ADR 0021 and ADR 0022 independently establish that all H-2 MF=6/MT=16
  LAW=7 source nodes conserve mean energy and that every H-2 NJOY finding is
  caused by the pinned processor's implicit-residual approximation; and
- ADR 0019 independently rejects N-15 because 33 of 37 MF=6/MT=102 source
  nodes miss a conservative one-percent capture-energy balance screen, even
  though the narrower processor check accepts the run.

Changing v0.3 in place would erase the history of what each gate knew. Simply
waiving all kinematic findings after explaining H-2 would be broader than the
evidence. The project needs a new suitability layer that applies each
reaction-level result only to the exact nuclide, source, processor execution,
and transport domain it supports.

## Decision

NCTForge adds
`nctforge.njoy-transported-photon-suitability/0.4.0`. Assessment and
verification require five immutable input reports:

1. the domain-aware v0.3 suitability report;
2. the independent H-2 LAW=7 implicit-residual report;
3. the receipt-bound H-2 processor-attribution comparison;
4. the independent N-15 MF=6 capture-balance report; and
5. the receipt-bound N-15 capture-moment comparison.

The v0.4 gate requires the two processor comparisons to bind their supplied
independent reports, the H-2 and N-15 processor outputs to match the
corresponding v0.3 runs, and both comparisons to bind the exact execution
receipt used by v0.3. The source-level reports must bind the same evaluated
source selection and photon-production inventory. Case identities and the
content-derived transport domain must also remain unchanged.

The only supported kinematic disposition is
`law7_implicit_residual_processor_approximation`, and validation permits it
only for H-2 when all H-2 violations are exactly attributed. The only
supported independent rejection is
`mf6_capture_photon_energy_balance_rejected`, and validation permits it only
for N-15. Neither disposition can be copied to another nuclide.

The report retains the full-evaluation, in-domain, and out-of-domain counts.
Attributed H-2 findings remain counted and are separately partitioned; only
unattributed in-domain findings remain rejecting. Rejecting source-format or
processor-data findings remain rejecting regardless of the kinematic
disposition. N-15 remains rejected through its independent gate even though it
has no v0.3 kinematic or nonkinematic rejection.

As in earlier suitability versions, `candidate_unreviewed` is not response
table approval, transport qualification, or clinical validation. Prior report
versions remain immutable and independently reproducible.

## Controlled result

The evidence-aware assessment produces:

| Measure | Result |
| --- | ---: |
| Full-evaluation kinematic findings retained | 120 |
| Findings inside the bound transport domain | 114 |
| H-2 findings attributed, full / in-domain / out-of-domain | 15 / 12 / 3 |
| Remaining in-domain findings | 102 |
| N-15 failed independent capture-balance nodes | 33 |
| v0.3 rejected to v0.4 candidate transitions | 1 |
| v0.3 candidate to v0.4 rejected transitions | 1 |
| Rejected v0.4 runs | 4 |

The run-level result is:

| Nuclide | v0.3 | v0.4 | Reason |
| --- | --- | --- | --- |
| H-2 | rejected | candidate unreviewed | all 15 findings exactly attributed to the LAW=7 implicit-residual processor approximation |
| N-15 | candidate unreviewed | rejected | independent MF=6 capture-energy balance fails at 33 nodes |
| C-13 | rejected | rejected | 32 in-domain findings and local-photon fallback remain |
| O-17 | rejected | rejected | 43 in-domain findings remain |
| O-18 | rejected | rejected | 27 in-domain findings and local-photon fallback remain |

All other nuclides remain `candidate_unreviewed`. The overall qualification is
still `transported_photon_kerma_rejected`.

The frozen v0.4 report SHA-256 is
`68b22afd510d477eb997fd514a37bcca9c45730e7fab22fd7ad9186d37f2baa0`.

## Consequences

- H-2 is no longer an honest blocker for this exact JEFF-4.0 evaluation,
  NJOY2016.78 execution, and 20 MeV transport domain. The result does not
  generalize to another reaction, source evaluation, processor, or domain.
- The project does not turn a processor warning into a blanket waiver. Every
  original finding remains visible and content-bound to its causal evidence.
- N-15's independently observed conservation failure is part of the integrated
  decision instead of being left beside a narrower passing processor gate.
- The remaining response-treatment investigation is C-13, O-17, and O-18,
  with 102 in-domain findings. A reviewed response table remains blocked.

## Related decisions and primary sources

- [ADR 0019: Independent MF=6 capture-photon balance](0019-independent-mf6-capture-photon-balance.md)
- [ADR 0020: Content-bound transport-domain suitability](0020-content-bound-transport-domain-suitability.md)
- [ADR 0021: Independent LAW=7 implicit-residual balance](0021-independent-law7-implicit-residual-balance.md)
- [ADR 0022: LAW=7 processor-approximation attribution](0022-law7-processor-attribution.md)
- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
