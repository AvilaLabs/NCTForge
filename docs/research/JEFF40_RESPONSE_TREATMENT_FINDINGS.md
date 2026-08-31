# JEFF-4.0 Response-Treatment Findings for `NF-BNCT-001`

**Recorded:** 2026-08-31

**Evidence state:** Controlled candidate rejected; not a qualified response
table

## Question

Does a coherent JEFF-4.0 evaluated-neutron selection resolve the four
transported-photon KERMA failures observed with the canonical ENDF/B-VIII.1
selection when every other frozen processing input remains unchanged?

## Bound inputs

- publisher archive: 608,170,633-byte
  `JEFF40-Evaluations-Neutron-593.zip`;
- publisher MD5: `51d00ee7bf1491d428f9b30a9782e41d`;
- locally computed archive SHA-256:
  `202c43e8bbea9ed530b6368cc7ef9fc919776e87bc79db5d4bbde937a999472f`;
- acquisition profile SHA-256:
  `8700f11ee9cd3c339ff78d24bb21c3faaf0d47c217840f17004787978abbe7d9`;
- source-selection SHA-256:
  `010bceddfd83d1bafbe5b46238195bdb64cd75ab0f368b6c6662de98c5c34dcf`;
- input-manifest SHA-256:
  `46dd5087dbd49f67399203dc149de586be698afa17796d17014c44ee330c191b`;
- processor: NJOY2016.78 commit
  `71a76bc6345fa15f36bacc816ae7900714345d97`, executable SHA-256
  `8a37cf70cf801b0c30ba70735f53a7b6aa51f18e53a10071fa0aff3341174c2d`.

The material, temperature, reconstruction tolerance, no-Q-override policy,
normal/check HEATR passes, and requested partial reactions are identical in
meaning to the baseline. The evaluated-data release and its content-bound
source files are the intended independent variable.

## Result

| Nuclide | Baseline | JEFF-4.0 | Candidate evidence |
| --- | --- | --- | --- |
| B-10 | candidate | candidate | no recognized blocker |
| C-12 | candidate | candidate | no recognized blocker |
| C-13 | candidate | rejected | 32 kinematic violations; no-photon local fallback |
| H-1 | candidate | candidate | no recognized blocker |
| H-2 | candidate | rejected | 15 kinematic violations |
| N-14 | candidate | candidate | no recognized blocker |
| N-15 | rejected | rejected | File 12 absent; kinematic count improves from 10 to 0 |
| O-16 | rejected | rejected | one kinematic violation; recognized warning is absent |
| O-17 | rejected | rejected | 45 kinematic violations; recognized warning is absent |
| O-18 | rejected | rejected | 27 kinematic violations; no-photon local fallback |

“Recognized warning is absent” means only that none of the three messages
structured by ADR 0013 appeared. It is not evidence that the evaluation's
photon representation is complete or physically suitable.

The aggregate candidate result is six rejected nuclides, 120 kinematic
violations, and three unique processor findings. Compared with the baseline,
zero of four rejected nuclides clears the complete gate and two new rejections
are introduced. The comparison is therefore `candidate_rejected`.

## Evidence anchors

- execution receipt SHA-256:
  `b53dd718d944af20d80d8c43d0832e4b5edc7edbf695d04642ab1efafe0803d9`;
- suitability report SHA-256:
  `0b33a754c6f1223fcb680d7fe7916d311d93acf7848e733c87ce607e5793cdbb`;
- baseline comparison SHA-256:
  `bd6c63ac973f83e4872c9c17175dc8c2b10a815f095e3c6febb4023426698b03`.

The checked receipts bind the external processor logs and tapes; the large
external artifacts are not redistributed in the repository.

## Interpretation and next test

JEFF-4.0 does not solve the response-treatment blocker under this method. The
mixed changes are still diagnostically useful: O-16 loses the recognized
incomplete-discrete-photon warning and drops from 15 violations to one, while
O-17 loses the local-fallback warning but increases to 45 violations. That
separates “photon records exist” from “the energy-balance response passes.”

The next high-value work is an exact MF=6/12/13/14/15 inventory for the four
baseline failures and the corresponding JEFF evaluations, followed by an
independent energy-release balance calculation that does not reuse HEATR's
implementation. Further whole-library candidates remain useful controls, but
they should not displace that causal investigation.

## Primary sources

- [JEFF-4.0 evaluated nuclear data library](https://data.oecd-nea.org/records/e9ajn-a3p20)
- [NJOY2016.78 HEATR source](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
