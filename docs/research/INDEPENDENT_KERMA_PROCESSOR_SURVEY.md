# Independent KERMA processor survey

**Recorded:** 2026-09-11

## Question

ADR 0026 allows the response-treatment path to resume with either a reviewed
independent O-17 reaction-level energy-balance calculation or an independent
trusted processing comparison. This note records whether any existing
processor can supply that comparison for JEFF-4.0 O-17 MT 301.

## Result: no off-the-shelf independent path exists

| Candidate | Verdict | Reason |
| --- | --- | --- |
| OpenMC `openmc.data` | not independent | Its HDF5 MT=301 heating copies the ACE heating number produced by NJOY HEATR (`neutron.py` creates reaction 301 from `ace.xss` heating). The existing `openmc-njoy-mt301-comparison` therefore verifies conversion fidelity, not independent physics. |
| Official OpenMC libraries | not independent | Published HDF5 libraries (ENDF/B-VIII.1, JEFF-4.0) are generated from NJOY-processed ACE. |
| NJOY21 | not independent | Wraps NJOY2016's Fortran HEATR; modernisation went component-based and HEATR was never reimplemented. |
| Modern njoy components | unavailable | RECONR, THERMR, LEAPR, ACER exist as C++ components; there is no HEATR component. |
| FUDGE | unavailable | LLNL's toolkit is on PyPI (≤1.1.1) but its `use_2to3` build tooling does not run on supported Python. |
| PREPRO | not applicable | The IAEA suite has no HEATR-equivalent KERMA production. |

## Consequence

The only remaining resume condition is the in-house independent O-17
reaction-level energy-balance integrator, specified below.

## Integrator scope (JEFF-4.0 O-17, NF-BNCT-001)

- Contributing reaction MTs (from the frozen attribution): 16, 22, 24, 28,
  32, 33, 34, 41, 91, 103, 104, 105, 106, 107, 108.
- All 15 use a uniform MF=6 layout: LCT=3 (CM system), NK=3–4 products
  (light ejectile, heavy residual, photon), every product LAW=1.
- Per-product data: TAB1 yield, then a LAW=1 TAB2 over incident energies,
  then LIST spectra. Across the sections, 314 spectra are isotropic (NA=0)
  and 256 carry one Legendre angular parameter (NA=1) — needed for the
  CM→lab mean outgoing energy, so NA=1 support is required, not optional.
- The existing strict parsers already cover MF=3 tabulations and the
  isotropic two-column LIST subset; the extension needed is bounded
  (per-MT Q-values from MF=3 heads, product yields, discrete + continuum
  first moments, NA=1 LIST records, and the CM→lab mean-energy transform).
- Output: per-reaction heating remainders and their sum at each of the 43
  in-domain finding energies, compared with NJOY's printed `ebal`
  contributions — emitted as an unreviewed evidence artifact. Review remains
  a human gate per ADR 0026.
