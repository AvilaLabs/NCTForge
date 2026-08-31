# ADR 0014: Evidence-Aware Workbench Shell

**Status:** Accepted and implemented for the first research shell

**Date:** 2026-08-31

## Context

The R1 desktop application began as a strict tri-planar geometry viewer. That
proved the DICOM and view-coordinate boundary but did not show how geometry,
transport preparation, component dose, and evidence will fit together. An early
workbench shell is useful for testing that workflow before calculation results
exist.

A scientific interface can also overstate maturity. A polished green status,
enabled run button, sample heat map, or placeholder DVH can look like evidence
even when the corresponding physics or execution path is absent.

## Decision

The native egui application has five stable workspaces:

1. Overview;
2. Geometry;
3. Transport;
4. Dose components; and
5. Evidence.

The shell follows these rules:

- `verified` is shown for geometry only after `load_nf_bnct_001` completes the
  runtime DICOM and artifact-integrity gate;
- checked benchmark contracts and reports are described as `frozen`, not as a
  verified local run;
- the O-17/O-18 response issue is always visible as `blocked`;
- transport controls reflect the backend descriptor and remain disabled while
  `prepare`, `execute`, and `import` are unavailable;
- the component workspace names the four physical quantities but displays no
  synthetic values, heat maps, totals, uncertainties, or DVHs; and
- the GUI cannot raise a result's qualification state. Future result views must
  consume the same normalized, validated bundle as the CLI.

The current readiness model is a small explicit shell model, not a second
scientific authority. As run orchestration is implemented, static frozen states
will be replaced by typed evidence loaded from the run bundle. Tests require all
five empty workspaces to render at the minimum supported viewport and require
the response and execution gates to remain blocked or pending.

## Consequences

- The intended product workflow is visible now without implying that dose
  calculation is implemented.
- A user can distinguish runtime verification, frozen project evidence,
  scientific blockers, and future work at a glance.
- UI affordances cannot silently get ahead of backend capability flags.
- Demonstration screenshots remain honest: an unavailable result is visibly
  unavailable instead of being represented by fabricated data.
- Later dose, comparison, and biological-model views must add typed artifact
  inputs and tests before replacing their empty states.
