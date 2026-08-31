# ADR 0018: Independent Continuum Photon-Energy Moments

**Status:** Accepted and implemented; full reaction balance remains pending

**Date:** 2026-08-31

## Context

ADR 0017 established that NJOY's `no file 12` message is not itself a photon-
production failure when a valid File 13 path exists. It did not establish that
the File 13 production cross sections and File 15 spectra carry the right
energy, or that HEATR uses them correctly.

The next check must be numerically independent of NJOY. Repeating HEATR or
reading its PENDF output would preserve the same implementation as both the
producer and the verifier.

## Decision

NCTForge adds two fail-closed evidence contracts.

`nctforge.endf-continuum-photon-energy-moment/0.1.0` reads the exact ENDF files
bound by the evaluated-source selection and photon-production inventory. For
each supported single-component File 13 continuum with matching File 15 data,
it independently calculates at every File 15 incident-energy node:

- weighted spectrum normalization,
  `p(E) * integral(g(E_gamma | E) dE_gamma)`;
- mean emitted-photon energy,
  `E_bar = integral(E_gamma * g dE_gamma) / integral(g dE_gamma)`;
- the matching File 13 continuum production cross section; and
- the photon energy-release term, `E_bar * sigma_gamma`.

The implementation parses ENDF tabulations directly and does not invoke NJOY
or consume a PENDF. It supports ENDF interpolation laws 1 through 5 for
point evaluation and histogram or linear-linear interpolation for the outgoing
spectra integrated in this first evidence set. Unsupported representations are
rejected rather than approximated.

`nctforge.njoy-continuum-photon-moment-comparison/0.1.0` binds that independent
report to an already verified execution receipt and the exact NJOY processor
report. It parses HEATR's bounded-precision File 13 diagnostic tables and
compares only incident energies shared with File 15 source nodes within the
print tolerance. HEATR-only union-grid points and source-only nodes are both
counted so the independent implementation does not reproduce HEATR's
two-dimensional interpolation algorithm. The relative tolerance is `6e-5`,
chosen for NJOY's five-significant-digit printed values. Photon energy, cross
section, their product, and the negative heating sign closure are all checked.

Numerical evidence is deserialized with Serde JSON's `float_roundtrip` feature.
This is required so a serialized IEEE-754 value regenerates bit-for-bit instead
of occasionally moving by one unit in the last place when re-read.

Both qualifications remain `unreviewed`. These contracts verify one continuum
photon term; they do not qualify a complete MT 301 response.

## Controlled result

The supported scope is N-15's eight continuum reactions: MT=4, 16, 22, 28,
103, 104, 105, and 107.

| Check | ENDF/B-VIII.1 | JEFF-4.0 |
| --- | ---: | ---: |
| Source reactions | 8 | 8 |
| File 15 incident-energy nodes | 92 | 92 |
| Failed spectrum normalizations | 0 | 0 |
| Maximum absolute normalization error | `1.6075e-5` | `1.6075e-5` |
| NJOY printed samples | 85 | 85 |
| Shared source nodes compared | 58 | 58 |
| Source-only nodes disclosed | 34 | 34 |
| HEATR-only interpolated nodes disclosed | 27 | 27 |
| Failed printed-value comparisons | 0 | 0 |
| Maximum relative printed-value difference | `4.827186715582159e-5` | `4.827186715582159e-5` |

The independently calculated numerical samples are identical between the two
evaluated-data selections over this supported continuum scope even though the
source-section bytes and content hashes differ. For example, at 6 MeV and
MT=4, the independent result is a 5.25 MeV mean photon energy, a 0.11522 barn
continuum production cross section, and 604,905 eV-barn of photon energy
release. NJOY prints the same values at its bounded precision.

Evidence hashes:

- ENDF/B-VIII.1 source-moment report:
  `2f3cd758f0b7106f8a859fcf0887a1047cea1646233c4ae5e25fec11563dddee`;
- JEFF-4.0 source-moment report:
  `6dac7055c0b970addfa1aa9bd89e5fa0f95ce87ffc4901e8a0c817ea2b4c455f`;
- ENDF/B-VIII.1 NJOY comparison:
  `8d0660d519915b5d0dd5ee4ce0fdd8d4973cb51c1f746e0bc0826dab7f5bb809`;
- JEFF-4.0 NJOY comparison:
  `c69ae5e033571cc7526fb4c66456370ef596bebb88c451be7bd4a990cd40d555`.

## Consequences

- The File 13/File 15 continuum integration is independently reproduced and
  agrees with NJOY's printed diagnostics within their output precision.
- This result rules out the supported continuum photon moments as the cause of
  the different N-15 suitability outcomes. It does not prove which remaining
  term causes that difference.
- The next causal check is the complete per-reaction energy balance, beginning
  with N-15 MF=6/MT=102 photon and recoil treatment, then the five remaining
  JEFF-4.0 failures.
- No response table is promoted. Discrete photons, all MF=6 product laws,
  recoil energy, Q-value balance, and the complete MT 301 construction remain
  outside this contract.

## Primary sources

- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
