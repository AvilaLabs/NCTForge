# ADR 0017: Source-Aware Photon-Production Suitability

**Status:** Accepted and implemented; both evaluated-data selections remain rejected

**Date:** 2026-08-31

**Subsequent refinements:** [ADR 0019](0019-independent-mf6-capture-photon-balance.md)
and [ADR 0020](0020-content-bound-transport-domain-suitability.md)

## Context

ADR 0013 intentionally treated every NJOY2016.78
`no file 12 for this material` message as a conservative review blocker. That
was safe, but it was not a correct physical interpretation of the message.

The pinned HEATR implementation first looks for File 12 and prints that message
when File 12 is absent. It then looks for and processes File 13. HEATR only uses
its no-photon local-deposition path when neither its File 6 photon-product path
nor a File 12/13 path is available. The ENDF-6 Formats Manual likewise defines
File 13 absolute photon-production cross sections as a valid representation;
File 12 is preferred for strong capture and fission resonances, not universally
required.

A processor log alone cannot distinguish a valid File 13 alternative from an
incomplete evaluation. The decision therefore needs the exact source records
that produced the run.

## Decision

NCTForge retains the immutable log-only schema from ADR 0013 and adds two
source-aware evidence contracts.

`nctforge.endf-photon-production-inventory/0.1.0` parses the exact evaluation
files selected by the content-addressed source manifest. It records every
MF=6/12/13/14/15 section by MAT, MT, record count, and section hash. It also
parses:

- File 6 product subsections, including `ZAP=0` photon products and their laws;
- File 12 multiplicity and transition-probability representations;
- File 13 discrete and continuum cross-section subsections;
- File 14 isotropic and anisotropic representation headers; and
- File 15 continuous-energy component counts.

The inventory checks the format-level relationships required for the legacy
File 12/13 representation: angular data in File 14, File 15 for a continuum,
and orphan File 14/15 sections. Its qualification is always
`source_inventory_unreviewed`; format pairing is not a physics qualification.

`nctforge.njoy-transported-photon-suitability/0.2.0` binds four already
verifiable artifacts:

1. the ADR 0013 log-only report;
2. its execution receipt and processor reports;
3. the exact executed NJOY input manifest; and
4. the photon-production inventory whose source-selection hash is in that
   manifest.

The File 12 message becomes
`informational_file13_alternative` only when the matching source evaluation has
File 13 reactions, has a HEATR photon source, and has no inventory format
finding. Kinematic violations, explicit no-photon local fallback, incomplete
discrete-photon data, and source-format findings remain rejecting. Any conflict
between the source inventory and HEATR's local-fallback message fails the
assessment rather than choosing one silently.

The only passing state remains `candidate_unreviewed`. This gate does not
approve the evaluation, the photon spectra, or a response table.

## Controlled result

Both ten-nuclide inventories have zero File 12/13/14/15 pairing findings and
eight evaluations with a HEATR photon source. Those counts do not mean the
energy-balance calculation passes.

| Selection | Log-only rejected runs | Source-aware rejected runs | Corrected interpretation |
| --- | ---: | ---: | --- |
| ENDF/B-VIII.1 baseline | 4 | 4 | N-15 File 13 is valid, but its 10 kinematic violations still reject it |
| JEFF-4.0 candidate | 6 | 5 | N-15 File 13/14/15 plus MF=6/MT=102 clears the narrow gate with zero violations |

The corrected JEFF-4.0 blockers are C-13, H-2, O-16, O-17, and O-18. JEFF-4.0
therefore remains rejected; this decision removes one false-positive reason but
does not weaken or suppress any energy-balance diagnostic.

Evidence hashes:

- ENDF/B-VIII.1 photon inventory:
  `8ccf4da3f29d879e473b49f72fc14f979d002a05cb09947be21ba7624ec697cc`;
- JEFF-4.0 photon inventory:
  `8e03f3f9ca894a3e6aafae59f3568a8c5b1f09d9c890279e15e4407c760bdd92`;
- ENDF/B-VIII.1 source-aware suitability:
  `6bd6cdef99fd940e386ffce46964f98e5e5f77a82c40517478a4aaa234d1d680`;
- JEFF-4.0 source-aware suitability:
  `3bc909a8285f8654fd62d776c427d7b7ef0825f5608b19744740bcbbc8babe92`.

## Consequences

- A missing File 12 is no longer treated as synonymous with missing photon
  production.
- The ADR 0013 reports remain reproducible historical evidence; source-aware
  v0.2 reports supersede only their File 12 interpretation.
- Whole-library screening remains useful. ADR 0018 completes the first
  independent File 13/File 15 continuum moment calculation; the remaining
  causal step is the complete per-reaction energy balance.
- No table may be clipped, warning suppressed, or promoted because this
  format-level inventory is clean.

## Primary sources

- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [Independent continuum photon moments](0018-independent-continuum-photon-moments.md)
