# ADR 0016: Versioned response-treatment candidates

**Status:** Accepted and implemented; first candidate rejected

**Date:** 2026-08-31

## Context

The canonical ENDF/B-VIII.1 source set fails the frozen
transported-photon KERMA requirement for N-15, O-16, O-17, and O-18. Replacing
an evaluated library informally would erase that baseline, weaken provenance,
and make improvements and regressions difficult to distinguish.

An alternate library is not a correction merely because it is newer or more
complete. It must be assessed with the same material, processor identity,
temperature, reconstruction tolerance, HEATR options, partial reactions, and
fail-closed interpretation.

## Decision

NCTForge introduces
`nctforge.evaluated-neutron-source-selection/0.2.0` for alternate
response-treatment candidates. It is separate from the frozen
ENDF/B-VIII.1-only `0.1.0` contract and requires:

- qualification `response_treatment_candidate_unreviewed`;
- a publisher-digest-matched acquisition with artifact role
  `incident_neutron_evaluations`;
- an exact, case-scoped nuclide selection bound to the unchanged material; and
- an evaluated-data release that exactly matches the candidate generation
  method.

The existing controlled preparation, execution, and transported-photon
suitability gates then apply unchanged. A new deterministic comparison schema,
`nctforge.response-treatment-candidate-comparison/0.1.0`, binds the baseline
and candidate suitability reports by ID and SHA-256. It records each nuclide's
status transition and aggregate rejected-run, kinematic-violation, and
processor-finding counts.

The comparison has only two qualifications:

- `candidate_rejected`; or
- `candidate_mechanical_gate_clear_unreviewed`.

The second state is deliberately not approval. Independent physical review and
the remaining ADR 0007 gates are still required.

## First controlled candidate

The first candidate uses the complete 593-evaluation JEFF-4.0 incident-neutron
archive from the OECD Nuclear Energy Agency Data Bank. Its 608,170,633 bytes
match publisher MD5 `51d00ee7bf1491d428f9b30a9782e41d`. The exact archive,
ten selected evaluations, and NJOY2016.78 executable/runtime are all
content-bound.

The candidate is rejected:

| Measure | ENDF/B-VIII.1 baseline | JEFF-4.0 candidate |
| --- | ---: | ---: |
| Rejected nuclides | 4 | 6 |
| Kinematic violations | 72 | 120 |
| Unique processor findings | 4 | 3 |
| Baseline rejections resolved | — | 0 |
| New rejections introduced | — | 2 |

N-15, O-16, O-17, and O-18 remain rejected. C-13 and H-2 become newly
rejected. The result is specific to the frozen NCTForge method and is not a
general judgment about JEFF-4.0.

## Consequences

- JEFF-4.0 cannot replace the baseline for the first response set.
- A whole-library substitution is now evaluated as evidence, not treated as an
  implicit upgrade.
- Candidate comparisons refuse a non-rejected baseline, mismatched case or
  requirement, different nuclide sets, or altered aggregate counts.
- JENDL-5 or any other candidate must enter through its own acquisition and
  source-selection profile. Patch chains must be represented explicitly.
- The investigation should now prioritize an exact photon-data inventory and
  an independent energy-balance calculation rather than assume another library
  swap will clear the blocker.

## Primary sources

- [JEFF-4.0 evaluated nuclear data library](https://data.oecd-nea.org/records/e9ajn-a3p20)
- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [ADR 0013: transported-photon KERMA suitability](0013-transported-photon-kerma-suitability.md)
