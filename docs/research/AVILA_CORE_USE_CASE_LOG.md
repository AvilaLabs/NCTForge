# Avila Core use-case record

This record tracks when NCTForge uses Avila Core, what Core contributed, and
what likely would have happened without it. It is an engineering record, not a
testimonial. Entries must keep direct observations separate from
counterfactual estimates.

## Recording rules

- Link the NCTForge and Core revisions, package, and durable run evidence.
- Record failures, neutral outcomes, and added work as well as benefits.
- Label a result **observed** only when repository history or run artifacts
  support it. Label controlled comparisons and timings **measured**.
- State counterfactuals as **inferred**, give a confidence level, and explain
  the basis. Do not turn an unmeasured estimate into hours saved.
- Say whether Core changed the domain result, the assurance around it, the
  development process, or some combination of those.
- Add a new dated entry for each bounded use rather than rewriting an earlier
  assessment with hindsight.

## Entry template

```text
Date and task:
NCTForge revision:
Core revision:
Core package/run evidence:

Starting point:
What Core did:
Observed result:
Development effect:
Likely result without Core (inferred):
Counterfactual confidence and basis:
What was not measured or established:
Follow-up:
```

## 2026-09-03 — JEFF-4.0 evidence-aware NJOY gate

**NCTForge revision:** `5b57f40` (`Dogfood Avila Core with the NJOY evidence
gate`)

**Core revisions exercised:** `1b2f3ff` (`Add actionable runtime diagnostics
and attempt logs`) and `cfe4eac` (`Add hash-bound external checker adapters`)

**Package and evidence:**
[`integrations/avila-core/njoy-evidence-aware/`](../../integrations/avila-core/njoy-evidence-aware/)
and [ADR 0024](../adr/0024-avila-core-evidence-loop.md)

### Starting point

NCTForge already owned the domain calculation and had a frozen six-document
evidence chain for the transported-photon KERMA assessment. It did not have a
narrow command for an external orchestrator to regenerate that assessment,
nor a Core package binding the exact inputs, checker, executable, output, and
requirements into one attempt record.

### What Core did

- Staged and hash-checked six NCTForge inputs.
- Bound the exact NCTForge executable and external-checker descriptor.
- Invoked NCTForge through a receipt-producing adapter and verified the output.
- Preserved the categorical scientific qualification separately from the
  numeric requirement.
- Evaluated the exact remaining-finding count against a declared limit.
- Recorded execution diagnostics and made replay inapplicable when free inputs
  changed, instead of comparing changed evidence with the frozen result.

### Observed result

- NCTForge regenerated and matched its evidence-aware result.
- The scientific result remained
  `transported_photon_kerma_rejected`, with `102` unexplained in-domain
  findings. Core correctly returned `FAIL` against a limit of zero while the
  checker process itself succeeded.
- The integration required NCTForge to expose the stable
  `njoy check-evidence-aware` machine boundary instead of asking Core to
  understand nuclear-data report internals.
- The exercise exposed that the checker descriptor's content identity was not
  initially part of Core's invocation and memoization identity. Core revision
  `cfe4eac` corrected that boundary.
- A changed-input run withheld frozen claims, reran the checker, and did not
  claim that replay of the reference case applied.

### Development effect

This first use was infrastructure investment, not a demonstrated time saving.
The NCTForge command and package were added while missing Core adapter behavior
was also being built. Core did improve consistency and inspectability in this
slice: the same attempt connected content identities, invocation, receipt,
claims, requirement verdict, and diagnostics without duplicating NCTForge's
scientific rules.

### Likely result without Core — inferred

NCTForge would very likely have reached the same scientific rejection and
count, because NCTForge computes and verifies those values. The probable
alternative was a direct CLI invocation or project-specific script plus manual
comparison of its JSON output. That would have been quicker for this one frozen
case, but it likely would not have produced the same reusable cross-project
package, verified attempt log, categorical/numeric claim separation, or
uniform changed-input behavior.

It is also plausible that the missing checker-descriptor identity would have
remained undiscovered until a later external-tool integration.

**Counterfactual confidence:**

- **High** that the scientific result would be unchanged: it is generated and
  independently checked inside NCTForge.
- **Medium** that the fallback would have been a bespoke command/script and
  manual evidence review: that matches the pre-integration boundary, but no
  controlled implementation was built for comparison.
- **Medium** that the descriptor-identity defect would have survived longer:
  this use exposed it, but another integration could also have done so.
- **Low / not estimated** for calendar or engineering time saved. No comparable
  baseline or prospective timing was captured, and this slice expanded Core
  itself.

### What this entry does not establish

It does not validate the scientific method, qualify a response table, show a
net speed improvement, or demonstrate reuse across a second NCTForge task. A
credible productivity claim needs prospective task timing or comparable work,
and a credible consistency claim needs the same Core path reused on later
investigations.

### Follow-up

For the next bounded NCTForge task, record the starting state and start time
before using Core, then capture interventions, failed attempts, output changes,
and elapsed active work. Reuse this package shape where it fits rather than
expanding Core merely to make the record look favorable.
