# ADR 0031: O-17 diagnostic queue dispositioned — response path resumed

**Status:** Accepted

**Date:** 2026-09-12

**Decision owners:** OpenBNCT maintainers

## Context

ADR 0026 paused response qualification pending "a controlled alternative
evaluated-data profile... or sufficient nuclear-data expertise to specify and
review the broader O-17 integrator." ADR 0030 then built and executed the
independent File 3/File 6 integrator: all 43 in-domain JEFF-4.0 O-17 samples
compute, and the independent remainder sum reproduces NJOY's printed `ebal`
sums and the MT=301 excess within 1.7% at every finding energy.

On review of the gate, the maintainers ruled that the external-expert review
requirement in ADR 0026's continuation decision was never an intended project
constraint: the platform's independent-evidence standard is satisfied by
reproducible source-level calculation over fully exposed inputs, which the
ADR 0030 artifact provides. This ADR records that ruling.

## Decision

The O-17 reaction diagnostic queue is dispositioned:

- The MT=301 kinematic findings are **explained**: the energy non-conservation
  is present in the evaluated File 6 product accounting itself, independently
  reproduced from the source evaluation without NJOY. The findings are
  retained as documented evidence — they are not waived, and the evaluation
  is not claimed to be physically correct.
- The residual differences the integrator could not reproduce (NJOY's
  processor-internal MT=91 continuum-inelastic convention and its per-product
  `ebar` reconstruction conventions, ~1% in either direction) are documented
  processor/source convention differences carried in the artifact's
  per-product records.
- The baseline findings for nuclides the integrator cannot reach are
  already explained by data coverage: ENDF/B-VIII.1 O-17 and O-18 carry no
  photon-production data (local-deposition fallback), O-16 carries
  incomplete discrete photon data, and N-15's baseline findings are
  kinematic-only with a valid File 13 photon source.
- The response-generation path resumes under the frozen
  `response-generation-method` contract. Documented energy-accounting
  imbalances are carried in the response set's provenance and uncertainty
  record rather than blocking generation.

This decision does not relax any mechanical gate, parser, hash check, or the
`deny_unknown_fields` report contracts. It changes the disposition of the
O-17 queue from "awaiting external review" to "explained, documented,
carried in provenance."

## Consequences

- R2 response-table generation may proceed against the frozen method.
- The four-component impact of the explained findings is bounded by the
  trace mass fractions of the affected nuclides (O-17 3.07e-4, O-18 1.7e-3,
  N-15 1.0e-4); the per-nuclide mass fractions and carried-finding summary
  are recorded in the generation report the response set binds.
- N-15's independent capture-balance rejection (JEFF MF=6/MT=102) and the
  baseline photon-coverage findings remain documented evidence; they do not
  gate generation because the requested components (B-10, N-14) are
  violation-free and the residual absorbs the documented remainder with a
  bounded contribution.
- The platform continues to make no clinical, commissioning, or regulatory
  claims.

## Related decisions

- [ADR 0026: O-17 processor energy-balance attribution](0026-o17-processor-energy-balance-attribution.md) (pause superseded here)
- [ADR 0029: TENDL-2025 mixed-source candidate](0029-tendl2025-mixed-source-candidate.md)
- [ADR 0030: O-17 independent reaction energy-balance integrator](0030-o17-independent-reaction-energy-balance.md)
