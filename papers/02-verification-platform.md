# Paper 2 — A transport-neutral platform for independent BNCT dose verification

**Status:** Blocked. Requires R2 response tables and a demonstrated second backend.

**Date opened:** 2026-08-31

## Target venues

- *Medical Physics* (AAPM) — preferred. Independent secondary dose
  verification is an established discipline in this community, which fits a
  verifier framing better than a planner framing.
- *Physics in Medicine and Biology* — the closest comparator appears to have
  submitted here; direct comparability, same audience.
- *Applied Radiation and Isotopes* — the field's traditional home.

## Claim

BNCT component doses are not comparable between institutions because component
definitions, nuclear-data processing, estimator choices, and biological
weighting all differ and are rarely recorded in a machine-checkable form.

NCTForge normalizes component-resolved physical dose into a backend-neutral,
content-bound interchange contract that separates physical transport from
biological interpretation, retains voxel-level uncertainty without assuming
independence between component tallies drawn from shared histories, and binds
every reported number to hashed inputs.

The demonstration is a result produced outside OpenMC, imported and normalized
without loss of meaning, and compared against the OpenMC path on a frozen case.

## Required evidence

- [ ] Reviewed response tables — gated on the R2 data blocker
- [ ] A second `TransportBackend` implementation. The trait currently has one
      implementor, so transport neutrality is asserted, not demonstrated
- [ ] Analytic validation of the folding chain — exact, and available without
      transport
- [ ] Comparison against published measured phantom data
- [ ] Cross-code comparison on a frozen case
- [ ] Published, versioned interchange schema with golden valid and invalid
      documents
- [ ] Correct physical-total uncertainty, demonstrated rather than specified
- [ ] Independent reproduction outside Avila Labs

## Notes

Cross-code agreement is the weakest evidence in this list, not the strongest —
two codes can agree while sharing the same wrong data. Analytic and measured
references carry the argument; the cross-code result demonstrates the
interchange contract.
