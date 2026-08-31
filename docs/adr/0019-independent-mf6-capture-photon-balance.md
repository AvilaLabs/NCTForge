# ADR 0019: Independent MF=6 Capture-Photon Balance

**Status:** Accepted and implemented; N-15 candidate rejected by the deeper gate

**Date:** 2026-08-31

## Context

ADR 0018 independently reproduced N-15's File 13/File 15 continuum photon
moments and ruled them out as the cause of the different ENDF/B-VIII.1 and
JEFF-4.0 processor outcomes. The remaining N-15 difference is JEFF-4.0's
MF=6/MT=102 photon product. It has no explicit residual-nucleus product, and
NJOY reports that it substitutes photon-momentum recoil.

NJOY's zero capture kinematic violations cannot qualify that source by itself.
In the pinned HEATR implementation, File 6 sets the kinematic lower and upper
bounds to the calculated heating value itself. Its general final-product
energy-balance remainder is also excluded for MT=102. This makes the narrow
processor limit check non-independent for this capture path.

The ENDF-6 manual defines LCT=3 light-particle energy and angle in the center-
of-mass frame and says particle, photon, and recoil distributions together
should conserve energy. A source-level check can therefore test the evaluated
photon term without accepting NJOY's self-bounds.

## Decision

NCTForge adds two fail-closed evidence contracts.

`nctforge.endf-mf6-capture-photon-balance/0.1.0` reads the exact File 3 and
File 6 sections from the evaluation bound by the source selection and photon
inventory. Its initial scope is one MT=102, LCT=3, LAW=1 photon product with
LANG=1, LEP=1, no angular coefficients, and no explicit recoil product.
Unsupported forms are rejected rather than approximated.

For every File 6 incident-energy node, the calculator independently derives:

- discrete-plus-continuum spectrum normalization;
- raw and normalized first and second photon-energy moments;
- photon yield and total emitted-photon energy;
- photon-momentum recoil from the normalized second moment;
- the internal capture budget, `Q + A/(A+1) E`;
- the equivalent laboratory budget after adding `E/(A+1)` translation energy;
- signed and relative residuals; and
- the residual folded with the exact File 3 capture cross section.

The photon-momentum recoil is explicitly identified as an independent second-
moment approximation. It matches the model HEATR says it substitutes; it is
not represented as an evaluated recoil distribution.

The absolute normalization tolerance is `1e-4`. The relative energy-balance
tolerance is `1e-2`. The latter is a deliberately generous project screening
threshold, not an ENDF, regulatory, or clinical acceptance criterion. It is
orders of magnitude above the observed spectrum-normalization errors, so ENDF
decimal rounding cannot plausibly manufacture a failure at that threshold.

The qualifications are:

- `missing_capture_photon_data_rejected`;
- `spectrum_normalization_rejected`;
- `capture_photon_energy_balance_rejected`; and
- `capture_photon_energy_balance_checked_unreviewed`.

`nctforge.njoy-mf6-capture-photon-moment-comparison/0.1.0` then binds the
independent report to an already verified NJOY execution receipt. It compares
only shared source/processor nodes and tests raw mean photon energy, photon
yield, and synthesized photon recoil against NJOY's five-significant-digit
printout with a `6e-5` relative tolerance. This comparison validates the
independent integration; NJOY remains outside the energy-balance decision.

## Controlled result

| Check | ENDF/B-VIII.1 N-15 | JEFF-4.0 N-15 |
| --- | ---: | ---: |
| MF=6/MT=102 photon source | absent | one LCT=3, LAW=1 product |
| Source incident-energy nodes | 0 | 37 |
| Failed spectrum normalizations | 0 | 0 |
| Maximum absolute normalization error | 0 | `3.9270394447399326e-7` |
| Failed 1% energy-balance samples | not applicable | 33 |
| Failed photon oversupply samples | not applicable | 9 |
| Failed photon undersupply samples | not applicable | 24 |
| Maximum absolute relative residual | not applicable | `5.751207778410636e-2` |

Only the 0.4, 0.6, 0.8, and 1.0 MeV source nodes fall within 1%. At thermal
energy, the source emits 2,532,280 eV in photons on average against a roughly
2,490,000 eV internal budget; adding 24.01 eV of photon-momentum recoil leaves
a `-42,304` eV residual. Because recoil energy cannot be negative, this
oversupply is independently rejecting. At 20 MeV, the reconstructed residual
is `+1,220,972` eV, or 5.7512% of the internal budget.

The receipt-bound print comparison finds 23 shared nodes among 37 source and
52 processor nodes. All 23 pass, with maximum relative difference
`4.5422371274853783e-5`. The independent thermal and 20 MeV recoil values,
24.0103 eV and 2,215.66 eV, reproduce NJOY's printed 24.010 eV and 2,215.7 eV
within its output precision. This confirms that the rejection is not caused by
a divergent interpretation of the source moments.

Evidence hashes:

- ENDF/B-VIII.1 missing-source report:
  `2f8a5b6bdf057d110ce4e28987d5c6850df01fb52c7e088a49e6c61938e05858`;
- JEFF-4.0 capture-balance report:
  `306a0d893f7ea8e3b5490a7cc6f5556a6de523e0171bb98dc23571bec1febbce`;
- JEFF-4.0 NJOY print comparison:
  `e3b995922e91214d07f708c307c38f19166fe4b51c38e0611c6fcc01d5bdd831`.

## Consequences

- JEFF-4.0 N-15 no longer counts as cleared for a complete response-treatment
  gate. It cleared only the narrower source-aware processor check from ADR
  0017; its capture photon source fails this independent energy screen.
- The result explains why replacing “File 12 absent” with “MF=6 present” is
  insufficient. Representation availability and processor self-consistency do
  not establish conservation.
- The immutable source-aware v0.2 report remains reproducible historical
  evidence. ADR 0019 adds a deeper gate; it does not rewrite that schema.
- No response table is promoted. ADR 0020 subsequently establishes that
  O-16's sole JEFF-4.0 finding is above the content-bound 20 MeV transport
  domain. The remaining causal work is C-13, H-2, O-17, and O-18, followed by
  a separately reviewed response-treatment decision.

## Primary sources

- [ENDF-6 Formats Manual, 2023](https://www.nndc.bnl.gov/endfdocs/ENDF-102-2023.pdf)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
