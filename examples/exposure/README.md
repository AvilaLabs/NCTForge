# Exposure-plan example: weighted field/fraction accumulation

Demonstrates `nctforge accumulate`: an `nctforge.exposure-plan/0.1.0`
artifact binds two weighted exposures — a lateral field at weight 1.0 and an
AP field at weight 0.5 — each pointing at a physical dose bundle by SHA-256.

Both exposures reference the same smoke bundle here for illustration; a real
plan would bind distinct per-field runs. Weights are multiplicative delivery
scales on each bundle's gray-per-source-particle values; per-exposure
`duration_s` and `boron_assumption` record delivery semantics, and the
declared covariance (`independent_exposures`) means 1-sigma uncertainties
add in quadrature.

```text
# Copy a collected bundle beside the plan (paths resolve relative to the
# plan file); if your bundle differs from the demonstration smoke bundle,
# recompute its SHA-256 and update the plan — bundle bytes are
# content-bound.
cp DOSE-BUNDLE.json examples/exposure/smoke-dose-bundle.json
nctforge accumulate \
  --plan examples/exposure/two-field-plan.json \
  --output accumulated-dose.json
```

The accumulated output is an ordinary
`nctforge.physical-dose-bundle/0.2.0` — it flows through `dvh`,
`bio apply`, and the GUI unchanged, with provenance binding the plan hash
and the recorded covariance assumption. When exposures bind different
response sets (for example differing boron loading per field), the output's
`response_set` reference points at the plan artifact, which enumerates them.

Research only: weights and boron notes are illustrative, not a treatment
plan.
