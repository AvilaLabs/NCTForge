# ADR 0013: Transported-Photon KERMA Suitability

**Status:** Accepted and implemented; canonical source set rejected

**Date:** 2026-08-31

**Subsequent refinement:** [ADR 0017](0017-source-aware-photon-production-suitability.md)
retains this log-only report as historical evidence but corrects the File 12
message using content-bound MF=6/12/13/14/15 source records. A valid File 13
alternative is informational in the source-aware v0.2 assessment.

## Context

The first component profile excludes photon energy from neutron KERMA and
scores its nonlocal deposition through coupled photon transport. A PENDF can be
mechanically valid while being unsuitable for that definition.

HEATR documents that, when an evaluation contains no photon data, it returns
only the neutron term. This is equivalent to depositing all photon energy
locally because the evaluation will create no photon transport source. HEATR
also warns that neutron/photon energy-balance defects can distort the spatial
distribution of heat in a small system, and that ad hoc local-heating fixes are
dangerous.

The first controlled execution exposed both conditions. Treating its generated
MT 301 tables as ordinary inputs would silently change the component meaning.

## Decision

NCTForge adds an execution-derived, fail-closed suitability stage for the
requirement
`transported_photon_kerma_with_coupled_photon_transport`.

`njoy assess-execution` first verifies the complete execution root against its
external receipt. It then reads only the receipt-bound processor reports and
structures these NJOY2016.78 data findings:

- no photon-production files, causing explicit local-deposition fallback;
- absence of photon multiplicity File 12; and
- a missing MF/MT in a discrete photon sequence that NJOY labels potentially
  incomplete.

The parser rejects an incomplete-photon message it cannot map to a file and MT.
Each unique finding records its occurrence count; the canonical decks run both
production and diagnostic HEATR, so every source-data warning occurs twice.
Kinematic violations from the execution receipt are combined with these
findings. Either condition rejects the nuclide.

The deterministic report binds the execution receipt and each processor report
by SHA-256. It is written only to a new path. `njoy verify-suitability` verifies
the execution root, regenerates the assessment, and requires exact semantic
equality with the supplied report.

The only non-rejected state is
`transported_photon_kerma_candidate_unreviewed`. It means this gate found no
blocker; it does not approve the evaluated data or replace independent review.
In particular, absence of File 12 alone is a conservative review blocker, not a
claim that no alternative MF=6 or MF=13 photon representation could be valid.

## Canonical result

The `NF-BNCT-001` report is
`benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-transported-photon-suitability.json`
with SHA-256
`39f32c071e715d4b712a92a25faf1424ba99f548aeabe88c934e84b5d2e48e22`.
It is `transported_photon_kerma_rejected`:

| Nuclide | NJOY data finding | Kinematic violations |
| --- | --- | ---: |
| N-15 | File 12 absent | 10 |
| O-16 | MF=12/MT=51 may be missing; discrete photon data may be incomplete | 15 |
| O-17 | no photon-production files; local-deposition fallback | 20 |
| O-18 | no photon-production files; local-deposition fallback | 27 |

B-10, C-12, C-13, H-1, H-2, and N-14 pass this narrow mechanical gate and
remain unreviewed candidates. Other processor messages and independent
physical checks remain separate qualification work.

## Consequences

- The current ENDF/B-VIII.1 selection cannot supply response tables for the
  frozen transported-photon component definition.
- The official OpenMC processed tables reproduce the selected MT 301 curves and
  retain effective local-photon fallback for O-17 and O-18; using the official
  transport distribution therefore does not clear this response gate.
- A successful NJOY exit and a complete PENDF file set cannot bypass source
  suitability.
- A future data profile must pass both kinematic diagnostics and this
  photon-data gate, then still pass the remaining ADR 0007 checks.
- Changing to local photon deposition would be a different component profile
  and benchmark version, not a workaround.
- The assessment is specific to the frozen NJOY2016.78 messages; support for a
  different processor version requires a versioned parser and evidence schema.

## Primary sources

- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
