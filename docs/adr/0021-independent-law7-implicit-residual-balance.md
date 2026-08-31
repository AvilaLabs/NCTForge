# ADR 0021: Independent LAW=7 Implicit-Residual Balance

**Status:** Accepted and implemented; H-2 source balance passes, processor attribution completed by ADR 0022

**Date:** 2026-08-31

## Context

The JEFF-4.0 H-2 run has 15 NJOY MT 301 high-limit findings. Twelve are inside
the content-bound 20 MeV transport interval; the findings at 21, 25, and 30
MeV remain preserved but are outside that interval. A kinematic warning alone
does not distinguish impossible evaluated data from a limitation in the
processor's treatment of that data.

The exact JEFF MF=6/MT=16 section has one product: a laboratory-frame LAW=7
neutron distribution with yield two. It does not include a separate proton
product. Conservation therefore implies an unrepresented charged residual,
but assuming that its energy is physical without integrating the source would
be circular.

NJOY2016.78 detects the absent residual, prints `one-particle recoil approx.
used.`, synthesizes a recoil term, and adds a File 6 energy-balance remainder
to MT 301. The pinned HEATR implementation does not include that remainder in
the File 6 kinematic upper bound. Before interpreting the warning, NCTForge
needs evidence independent of NJOY's approximation and its output.

## Decision

NCTForge adds the immutable
`nctforge.endf-mf6-law7-implicit-residual/0.1.0` report and corresponding
calculate/verify commands. The calculation:

1. verifies the exact evaluated-source selection and photon-production
   inventory against the extracted evaluation;
2. binds the source evaluation and exact MF=3/MT=16 and MF=6/MT=16 sections by
   SHA-256;
3. requires H-2, LCT=1, one LAW=7 neutron product, neutron yield two, and no
   MT=16 photon representation;
4. integrates every nested outgoing-energy distribution over emission cosine
   and outgoing energy using the declared lin-lin interpolation;
5. tests the joint distribution normalization; and
6. computes the energy left for the implicit proton as
   `E + QM - yield * mean outgoing-neutron energy`.

The one repeated 180 keV point in the 5 MeV, zero-cosine spectrum is accepted
only because both duplicate ordinates are exactly zero. It contributes a
zero-width, zero-density interval, is collapsed without changing either
integral, and is counted in the report. A decreasing energy, a duplicate with
different density, or any unsupported interpolation remains rejecting.

The source-level gate passes only when every active distribution normalizes
within `1e-4` and no implicit-residual energy is negative beyond a relative
`1e-6` tolerance. The qualification is deliberately `checked_unreviewed`.
This calculation establishes a mean energy balance; it does not reconstruct
event-level neutron-neutron correlations, create a proton recoil spectrum, or
qualify a processed response table.

## Controlled result

The exact JEFF-4.0 H-2 source passes the narrow source-level gate:

| Measure | Result |
| --- | ---: |
| LAW=7 incident-energy nodes | 54 |
| Zero-cross-section threshold nodes | 1 |
| Active nodes tested | 53 |
| Failed normalization nodes | 0 |
| Failed residual-energy nodes | 0 |
| Maximum absolute normalization error | `5.673713188159013e-8` |
| Minimum implicit-proton energy | `443111.1989433097 eV` |

At 9 MeV, the independent mean outgoing-neutron energy is
`2228133.1701294947 eV` per neutron. After applying the yield of two, the
source leaves `2319167.6597410105 eV` for the implicit proton and a local
energy-release term of `318189.8029164666 eV*b`.

The frozen report SHA-256 is
`0cfaaf52c67f359b3fd2c70b147e92dd9e004e3495bb860f9ad5ab7707acd1d5`.

## Consequences

- The H-2 findings are not evidence that the evaluated LAW=7 neutron source
  overspends its available mean reaction energy.
- The result strongly motivates testing NJOY's missing-residual approximation
  as the cause, but it does not clear H-2 by itself. The immutable v0.3
  suitability report remains rejected.
- ADR 0022 binds this calculation to the exact execution receipt and
  reproduces NJOY's printed neutron moment, synthesized recoil, energy-balance
  remainder, final MT 301 value, and kinematic excess at every warning energy.
- Only a fully matched processor attribution may support a narrowly scoped
  H-2 reclassification. ADR 0023 now applies that exact disposition in a new
  suitability schema; it does not waive C-13, N-15, O-17, or O-18.

## Related decisions and primary sources

- [ADR 0012: Controlled NJOY execution evidence](0012-controlled-njoy-execution-evidence.md)
- [ADR 0019: Independent MF=6 capture-photon balance](0019-independent-mf6-capture-photon-balance.md)
- [ADR 0020: Content-bound transport-domain suitability](0020-content-bound-transport-domain-suitability.md)
- [ADR 0022: LAW=7 processor-approximation attribution](0022-law7-processor-attribution.md)
- [ADR 0023: Reaction-evidence-aware transported-photon suitability](0023-reaction-evidence-aware-suitability.md)
- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
