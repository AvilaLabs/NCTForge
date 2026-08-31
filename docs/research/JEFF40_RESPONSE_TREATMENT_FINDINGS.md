# JEFF-4.0 Response-Treatment Findings for `NF-BNCT-001`

**Recorded:** 2026-08-31

**Evidence state:** Controlled candidate rejected after independent capture-
balance and transport-domain refinements; not a qualified response table

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
| H-2 | candidate | rejected pending processor attribution | 15 kinematic violations; independent LAW=7 source balance passes all 53 active nodes |
| N-14 | candidate | candidate | no recognized blocker |
| N-15 | rejected | rejected by deeper gate | valid File 13/14/15 path plus MF=6/MT=102 clears the narrow processor check, but 33 of 37 capture-balance nodes fail independently |
| O-16 | rejected | candidate under bound domain | sole finding is at 30 MeV, above the common 20 MeV OpenMC domain |
| O-17 | rejected | rejected | 45 kinematic violations; recognized warning is absent |
| O-18 | rejected | rejected | 27 kinematic violations; no-photon local fallback |

“Recognized warning is absent” means only that none of the three messages
structured by ADR 0013 appeared. It is not evidence that the evaluation's
photon representation is complete or physically suitable.

The original log-only report has six rejected nuclides, 120 kinematic
violations, and three unique processor findings. The source-aware v0.2 report
correctly treats N-15's File 12 message as informational because the exact
evaluation supplies eight paired File 13/14/15 continuum reactions and an
MF=6/MT=102 photon product. Its corrected aggregate is five rejected nuclides:
C-13, H-2, O-16, O-17, and O-18. One baseline rejection clears, but two new
rejections are introduced, so the candidate remains rejected.

That v0.2 aggregate describes its intentionally narrow source-aware processor
gate. ADR 0019 adds a deeper source-level conservation gate and rejects N-15
again; it does not mutate the immutable v0.2 evidence.

ADR 0020 then derives the common OpenMC transport interval from the exact
processed-data manifest and material instead of supplying a loose cutoff. Its
closed diagnostic interval ends at 20 MeV. The domain-aware v0.3 report retains
all 120 JEFF findings, classifies 114 in domain and six above it, and clears
only O-16. C-13, H-2, O-17, and O-18 remain rejected. Together with the
independent N-15 rejection, the candidate still has five unresolved nuclides.

Both exact ten-nuclide inventories have zero format-pairing findings and eight
evaluations with a HEATR photon source. This establishes record availability,
not energy-balance validity.

## Evidence anchors

- execution receipt SHA-256:
  `b53dd718d944af20d80d8c43d0832e4b5edc7edbf695d04642ab1efafe0803d9`;
- suitability report SHA-256:
  `0b33a754c6f1223fcb680d7fe7916d311d93acf7848e733c87ce607e5793cdbb`;
- baseline comparison SHA-256:
  `bd6c63ac973f83e4872c9c17175dc8c2b10a815f095e3c6febb4023426698b03`.
- JEFF-4.0 photon inventory SHA-256:
  `8e03f3f9ca894a3e6aafae59f3568a8c5b1f09d9c890279e15e4407c760bdd92`;
- JEFF-4.0 source-aware suitability SHA-256:
  `3bc909a8285f8654fd62d776c427d7b7ef0825f5608b19744740bcbbc8babe92`;
- JEFF-4.0 independent continuum-moment SHA-256:
  `6dac7055c0b970addfa1aa9bd89e5fa0f95ce87ffc4901e8a0c817ea2b4c455f`;
- JEFF-4.0 NJOY moment-comparison SHA-256:
  `c69ae5e033571cc7526fb4c66456370ef596bebb88c451be7bd4a990cd40d555`;
- JEFF-4.0 MF=6 capture-balance SHA-256:
  `306a0d893f7ea8e3b5490a7cc6f5556a6de523e0171bb98dc23571bec1febbce`;
- JEFF-4.0 capture-moment print-comparison SHA-256:
  `e3b995922e91214d07f708c307c38f19166fe4b51c38e0611c6fcc01d5bdd831`;
- content-bound OpenMC transport-domain SHA-256:
  `1554dfb3167c0aa804cd6c893ce22a363cefbc0cba1b8f7781eeae1c2dccf89e`;
- baseline domain-aware suitability SHA-256:
  `e270708da7aabf0be6246d8b89fabf031af4ec01c155b015432e2ee174eb9d09`;
- JEFF-4.0 domain-aware suitability SHA-256:
  `6e46b627d9b766e596ad2219eaafca970bd9f3c5df1d5e400ad644397c44ce55`.
- JEFF-4.0 H-2 LAW=7 implicit-residual report SHA-256:
  `0cfaaf52c67f359b3fd2c70b147e92dd9e004e3495bb860f9ad5ab7707acd1d5`.

The baseline-comparison artifact is intentionally the immutable v0.1
log-only comparison. ADR 0017 and the v0.2 report supersede only its treatment
of the File 12 message.

The checked receipts bind the external processor logs and tapes; the large
external artifacts are not redistributed in the repository.

## Interpretation and next test

JEFF-4.0 does not solve the response-treatment blocker under this method. The
mixed changes are still diagnostically useful: O-16 loses the recognized
incomplete-discrete-photon warning and its sole remaining finding occurs at
30 MeV, outside the bound first-calculation domain. O-17 loses the local-
fallback warning but retains 43 in-domain violations. That separates “photon
records exist” from “the energy-balance response passes.”

The exact MF=6/12/13/14/15 inventory and source-aware interpretation are now
complete. NCTForge has also independently integrated all eight supported N-15
File 13/File 15 continuum reactions: both selections produce the same 92
source-node samples, and 58 shared nodes agree with NJOY's printed diagnostics
within `4.827186715582159e-5`. This rules out that supported continuum term as
the cause of N-15's different outcome; it does not by itself prove the cause.

The MF=6/MT=102 causal check is now complete. All 37 JEFF spectra normalize
within `3.93e-7`, and independent first/second moments reproduce NJOY's photon
and synthesized-recoil print tables at 23 shared nodes. Nevertheless, 33 source
nodes fail a conservative 1% Q-value balance screen, reaching 5.7512% at 20
MeV. Nine failures oversupply photons before any positive recoil is added,
including a 42.3 keV thermal excess. NJOY reports zero violations because its
File 6 kinematic limits are set to the calculated File 6 result itself; that is
not an independent conservation test.

O-16's scope question is now resolved without deleting its full-range
diagnostic. The first H-2 causal layer is also complete: all 53 active
MF=6/MT=16 LAW=7 nodes normalize within `5.674e-8`, and every node leaves
positive mean energy for the implicit proton. The minimum residual is 443.1
keV. This rules out a source-level mean-energy overspend, but H-2 remains
rejected until a receipt-bound comparison proves that each in-domain warning
is caused by NJOY's missing-residual approximation and excluded
energy-balance remainder. C-13, O-17, and O-18 remain untouched.

## Primary sources

- [JEFF-4.0 evaluated nuclear data library](https://data.oecd-nea.org/records/e9ajn-a3p20)
- [NJOY2016.78 HEATR source](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [Source-aware photon-production decision](../adr/0017-source-aware-photon-production-suitability.md)
- [Independent continuum photon-moment decision](../adr/0018-independent-continuum-photon-moments.md)
- [Independent MF=6 capture-balance decision](../adr/0019-independent-mf6-capture-photon-balance.md)
- [Content-bound transport-domain decision](../adr/0020-content-bound-transport-domain-suitability.md)
- [Independent LAW=7 implicit-residual decision](../adr/0021-independent-law7-implicit-residual-balance.md)
