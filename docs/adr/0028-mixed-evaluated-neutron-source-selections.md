# ADR 0028: Mixed evaluated-neutron source selections

**Status:** Accepted and implemented; first mixed candidate executed and rejected (ADR 0029)

**Date:** 2026-09-11

## Context

ADR 0026 paused the response-treatment qualification path pending either a
controlled alternative evaluated-data profile or an independently reviewed O-17
calculation. TENDL-2025 is the remaining credible alternative: its informal
screening cleared N-15, O-16, O-17, and O-18 — exactly the four nuclides that
reject both ENDF/B-VIII.1 and JEFF-4.0.

Two publisher constraints make the existing candidate schema unable to express
that experiment:

- TENDL covers Z ≥ 3 only. It distributes no H-1 or H-2 evaluations, so no
  TENDL-only archive can satisfy the ten-nuclide NF-BNCT-001 material.
- TENDL-2025 publishes no archive digest. The 0.2.0 candidate schema requires a
  `matched` publisher-digest status and therefore rejects TENDL categorically.

Per-nuclide mixing is not a scientific inconsistency. ENDF-6 evaluations are
independent per isotope; there are no inter-nuclide reactions to keep coherent.
Transport libraries routinely combine evaluations from different releases. What
must not happen silently is which nuclides came from which archive.

## Decision

NCTForge introduces
`nctforge.evaluated-neutron-source-selection/0.3.0` for selections that bind
more than one publisher acquisition:

- `acquisition` is replaced by `acquisitions`, a non-empty list of acquisition
  blocks ordered canonically by archive SHA-256 with no duplicates.
- Every evaluation record carries `acquisition_sha256`, the archive SHA-256 of
  the acquisition it was extracted from. Every declared acquisition must be
  referenced by at least one evaluation; every evaluation must reference a
  declared acquisition. The binding is therefore content-addressed end to end.
- Each acquisition is still validated against its own profile and receipt pair.
  The CLI accepts `--profile`/`--receipt` once per bound acquisition, paired
  positionally.
- Acquisition roles may be `endfb_incident_neutron_evaluations` or
  `incident_neutron_evaluations`, so a baseline-role archive can supply the
  nuclides a candidate archive cannot.
- The publisher-digest rule is preserved rather than weakened: when a profile
  declares a digest, the receipt must still report `matched`. When a profile
  documents that the publisher distributes no digest, `unavailable` is
  accepted. The absence of a digest is recorded in the profile itself, so the
  weaker evidence state is explicit and per-archive.
- Qualification remains `response_treatment_candidate_unreviewed`. Every
  downstream gate — deterministic deck generation, controlled NJOY execution,
  the suitability chain, and independent review — is unchanged.

`njoy prepare` emits manifest schema
`nctforge.njoy-input-manifest/0.2.0` for a mixed selection, binding each
acquisition's profile and receipt SHA-256 as a list. Single-acquisition
selections continue to emit `0.1.0` manifests byte-for-byte, so the frozen
baseline and JEFF-4.0 reproduction recipes are unaffected.

A mixed selection is a deliberate scientific act. It is recorded in the
contract itself, not in prose alone, and it still requires the full evidence
chain before any response table may be generated.

## Consequences

- The first mixed selection — ENDF/B-VIII.1 for B-10, C-12, C-13, H-1, H-2, and
  N-14; TENDL-2025 for N-15, O-16, O-17, and O-18 — can be assessed under the
  frozen method.
- TENDL-2025's undigested archive is bound by exact byte count, SHA-256,
  transfer metadata, and receipt rather than by publisher attestation.
- Schema `0.1.0` and `0.2.0` documents are unchanged and continue to require a
  single matched acquisition.
- No `0.3.0` selection may silently drop a nuclide's provenance or declare an
  acquisition that no evaluation uses.
