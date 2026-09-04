# ADR 0024: Avila Core Evidence Loop

**Status:** Accepted and implemented as a research integration

**Date:** 2026-09-03

## Context

NCTForge now has an immutable, independently regenerable v0.4 assessment for
the JEFF-4.0 transported-photon investigation, but the next work will change
parts of that evidence chain repeatedly. A useful external loop needs to say
which bytes and executable were checked, keep scientific rejection distinct
from tool failure, and return compact machine evidence without moving nuclear-
data rules into an orchestrator.

Avila Core provides the first pressure test for that boundary. Its declarative
external-checker adapter can stage hash-bound inputs, invoke an exact binary,
record and verify a receipt, and extract exact and closed-set categorical
claims. NCTForge must remain responsible for regenerating and validating the
domain result.

## Decision

NCTForge adds `njoy check-evidence-aware`. The command reads the domain-aware
report, both independent reaction reports, both processor-attribution reports,
and the evidence-aware report. It regenerates the assessment through
`verify_against_evidence` and refuses any content or binding mismatch. Only
after that verification does it create, without overwrite, a deterministic
`nctforge.njoy-evidence-aware-check/0.1.0` result containing:

- the exact source-report identity;
- the transport requirement;
- the categorical suitability qualification;
- the rejected-run count;
- the exact remaining in-domain finding count;
- a `regenerated_and_matched` verification state; and
- explicit non-claims and limitations.

The command exits successfully when verification succeeds even if the
scientific qualification is `transported_photon_kerma_rejected`. Rejection is
data. Invalid evidence, a failure to regenerate, I/O failure, or inability to
write the result is an execution error.

The integration under `integrations/avila-core/njoy-evidence-aware/` binds the
six frozen inputs, the checker adapter, the expected result, and the exact local
Linux debug executable used to freeze the specimen. Core extracts the remaining
count as an exact dimensionless claim and the qualification as a separate
unquantified categorical claim. Only the count drives the explicit requirement
that no unexplained in-domain finding remain.

All six scientific inputs are declared free for subsequent investigations.
Supplying a changed member makes Core withhold the frozen claims and rerun
NCTForge; every mutually dependent changed report must be supplied together.
Replay against the frozen case is then not applicable rather than silently
treated as a mismatch.

## Follow-on

[ADR 0025](0025-diagnostic-triage-of-remaining-njoy-findings.md) reuses and
revises this same package after the first investigation pass. The package now
binds a seventh diagnostic-triage input, gates the 43-finding independent queue,
and—using Core's closed categorical requirements—also gates response
qualification directly. The description above remains the historical first
slice rather than being rewritten as though that capability existed initially.

## Boundary

This integration does not approve a response table, qualify transport, make a
clinical claim, or establish that Core understands NJOY or ENDF physics. Core's
receipt establishes process provenance, not method validity. The checked-in
executable hash is a development-machine identity, not a reproducible release
artifact; rebuilding on another platform requires deliberate re-freezing after
inspection.

## Consequences

- The follow-on triage uses this integration for real NCTForge evidence and
  narrows the next independent investigation to O-17.
- A domain rejection remains a valid, queryable result while process failures
  remain unmistakable failures.
- NCTForge exposes one narrow stable machine surface instead of asking an
  orchestrator to parse the full scientific report.
- Future integration work should first replace the local executable pin with a
  reproducible release identity, then add qualification evidence only when the
  underlying method has actually earned it.

## Related decisions

- [ADR 0019: Independent MF=6 capture-photon balance](0019-independent-mf6-capture-photon-balance.md)
- [ADR 0021: Independent LAW=7 implicit-residual balance](0021-independent-law7-implicit-residual-balance.md)
- [ADR 0022: LAW=7 processor-approximation attribution](0022-law7-processor-attribution.md)
- [ADR 0023: Reaction-evidence-aware transported-photon suitability](0023-reaction-evidence-aware-suitability.md)
- [ADR 0025: Diagnostic triage of remaining NJOY findings](0025-diagnostic-triage-of-remaining-njoy-findings.md)
- [Integration case](../../integrations/avila-core/njoy-evidence-aware/README.md)
- [Avila Core use-case record](../research/AVILA_CORE_USE_CASE_LOG.md)
