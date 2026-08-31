# NJOY2016.78 Kinematic Findings for `NF-BNCT-001`

**Recorded:** 2026-08-31

**Evidence state:** Rejected execution evidence; not a qualified response table

## Scope

This note records the first controlled execution of the ten deterministic NJOY
decks for `NF-BNCT-001`. It reports what NJOY2016.78 observed; it does not yet
assign a physical cause to the findings.

HEATR describes its check as a comparison of energy-balance KERMA factors with
conservative kinematic limits. In the frozen source, `low` is printed when a
KERMA value is more than 10% below the computed lower limit and `high` is
printed when it is more than 10% above the computed upper limit. NCTForge maps
only markers in the final KERMA table and records the preceding table energy
and corresponding response MT.

## Result

| Nuclide | Status | Violations | Response | Direction and sampled energies |
| --- | --- | ---: | --- | --- |
| B-10 | passed | 0 | MT 301/407/443 present | none |
| C-12 | passed | 0 | MT 301/443 present | none |
| C-13 | passed | 0 | MT 301/443 present | none |
| H-1 | passed | 0 | MT 301/443 present | none |
| H-2 | passed | 0 | MT 301/443 present | none |
| N-14 | passed | 0 | MT 301/403/443 present | none |
| N-15 | rejected | 10 | MT 301 | `high` at 1e-5, 1e-4, 1e-3, 1e-2, 0.1, 1, 2, 5, 10, and 20 eV |
| O-16 | rejected | 15 | MT 301 | `low` at each integer MeV from 6 through 20 MeV |
| O-17 | rejected | 20 | MT 301 | `high` at 3.6, 4.0, 4.6, 5.0, 5.5, 6.0, and each integer MeV from 7 through 20 MeV |
| O-18 | rejected | 27 | MT 301 | `high` at 1e-5, 1e-4, 1e-3, 1e-2, 0.1, 1, 2, 5, 10, 20, 50, 100, and 200 eV; also each integer MeV from 7 through 20 MeV |

All processes exited zero, standard-error files were empty, and NJOY wrote the
expected tapes. For every nuclide, the production HEATR PENDF and the
check-enabled HEATR PENDF were byte-identical. A repeat execution reproduced
all PENDF and plot tapes and every structured violation exactly.

The machine-readable source of truth is the checked-in execution receipt,
SHA-256
`65a21b57507e76a68b77349e92390ae03ebb8c38f6ed6cee66197aa5ee4adea7`.
It includes all 72 individual energy/MT/direction records and the hashes of the
raw reports from which they were parsed.

## Qualification decision

ADR 0007 gate 3 requires no unexplained energy-balance or kinematic-limit
violation in the active domain. That gate is not met. No KERMA response set is
generated from these outputs, and the affected natural-isotope mass fractions
are not grounds for ignoring a failed gate.

The following actions are explicitly prohibited as a way to obtain a pass:

- clipping a response to a kinematic limit;
- removing N-15, O-16, O-17, or O-18 from the frozen material;
- treating a failed nuclide as a zero response;
- suppressing check-mode output; or
- adding a Q-value override without a separately reviewed, versioned method.

## Investigation queue

The next work is diagnostic, not corrective. It must determine:

1. whether each selected evaluation supplies the photon and secondary-product
   information on which HEATR's energy-balance path depends;
2. whether the observed limits are a known NJOY2016.78/evaluation interaction
   or reproduce in a separately built processor;
3. whether the official processed OpenMC ENDF/B-VIII.1 heating tables show the
   same behavior after atomic-weight-ratio normalization;
4. whether a newer evaluated-data release changes the result under a new,
   explicitly versioned profile; and
5. which independent calculations can test the MT 301 behavior without
   sharing HEATR's implementation assumptions.

Until those questions are resolved and independently reviewed, the benchmark
remains blocked before transport execution.

## Primary sources

- [NJOY2016.78 HEATR source](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [ENDF/B-VIII.1 incident-neutron release](https://www.nndc.bnl.gov/endf-releases/?sublibrary=neutrons&version=B-VIII.1)
