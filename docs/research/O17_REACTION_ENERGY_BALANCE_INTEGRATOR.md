# O-17 independent reaction energy-balance integrator

**Recorded:** 2026-09-12

## Question

ADR 0026 paused the JEFF-4.0 response-treatment path until there is either a
reviewed independent O-17 reaction-level energy-balance calculation or an
independent trusted processing comparison.
`INDEPENDENT_KERMA_PROCESSOR_SURVEY.md` established that no off-the-shelf
processor can supply the comparison. This note records the in-house
alternative: a File 3/File 6 energy-balance integrator that does not call NJOY
and does not consume processed heating data.

## What was built

`openbnct-njoy::reaction_energy_balance` (schema
`openbnct.endf-reaction-energy-balance/0.1.0`) with two CLI boundaries:

- `njoy calculate-reaction-energy-balance`; and
- `njoy verify-reaction-energy-balance`.

For every MT present in both File 3 and File 6 of the selected evaluation the
calculator parses the source ENDF records and forms, at each in-domain finding
energy `E`, the independent reaction remainder

```text
h_MT(E) = sigma_MT(E) * [ (E + QM_MT) - sum_p yield_p(E) * ebar_p,lab(E) ]
```

which is compared against NJOY's printed per-reaction `ebal` remainders and
against the processor's `MT301 - kinematic maximum` excess from the same
receipt-bound run. Per-product `ebar` values are compared individually where
NJOY prints them.

## Conventions implemented

Each convention below is checkable against the ENDF-6 manual and is listed so
a reviewer can accept or dispute it explicitly:

- **Q-field selection.** File 3 carries the mass-difference Q (C1) and the
  reaction Q (C2). NJOY's File 6 `ebal` header prints the mass-difference Q
  (demonstrated by MT=91, where C1=0 and C2=-7.7636e6 eV), so the calculator
  uses C1.
- **LCT=3 frames.** Products tabulated with an angular distribution
  (`LANG=2`, Kalbach-Mann) are centre-of-mass and receive the CM-to-lab mean
  transform `ebar_lab = ebar_cm + E_T + 2*sqrt(E_T)*<sqrt(E')*mu>` with
  `E_T = E*AWP/(AWR+1)^2`. Isotropic products (`LANG=1`) are already
  laboratory-frame and are not boosted. This reproduces NJOY's printed ebar
  convention to sub-percent on transform-dominated products.
- **Kalbach-Mann parameters.** `NA=1` tabulates only the precompound fraction
  `r`; the slope `a` is computed from the ENDF-6 LANG=2 systematics
  (Kalbach, doi:10.1103/PhysRevC.37.2350), and the mean cosine is `r*L(a)`
  with `L` the Langevin function. A zero `r` is fully compound (isotropic)
  and does not require the slope. `NA=2` tabulated slopes are also accepted.
- **Spectra.** LAW=1 LIST records support discrete outgoing lines plus
  continuum under histogram (`LEP=1`) or linear-linear (`LEP=2`)
  interpolation, with analytic moments for both.
- **Incident interpolation.** Bracketing spectra are interpolated pointwise
  on the union of their outgoing-energy grids (the ENDF-6 LAW=1 convention),
  then moments are taken of the interpolated spectrum — exact for both
  histogram and linear-linear LEP.
- **Duplicate grid points.** Legal ENDF-6 discontinuities (consecutive equal
  x values, used by JEFF-4.0 to drop every O-17 cross section at 3.0e7 eV)
  resolve to the right-hand value. Strictly decreasing grids remain
  rejected.
- **Product identity.** JEFF-4.0 writes nonstandard ZAP codes for some light
  ejectiles (for example `1002` with `AWP=2.99` for the triton); the product
  mass is authoritative and the canonical `1000*Z+A` is recorded alongside
  the raw ZAP.
- **Disposition.** Light products with canonical ZA at most 2004 (neutron,
  photon, and light ejectiles) are carried away; heavier residuals deposit
  locally.

## Frozen result (JEFF-4.0 O-17, NF-BNCT-001)

Artifact:
`benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-o17-endf-reaction-energy-balance.json`,
SHA-256 `9a102394d11a25a90928b0cebd6b5aee6475704dbbe8ef68c6f52701e0711a55`.

| Observation | Result |
| --- | ---: |
| Shared File 3/File 6 MTs evaluated | 28 |
| In-domain finding samples | 43 |
| Samples fully computed | 43 |
| Samples partially computable | 0 |
| Samples matching printed remainder sum (tolerance 5e-3) | 17 |
| Maximum aggregate remainder relative difference | `1.682e-2` |
| Maximum independent-remainder vs MT=301 excess difference | `1.682e-2` |
| Maximum product `ebar` relative difference | `5.097e-1` |
| Product `ebar` comparisons | 515 |

## Interpretation

At every finding energy the independently computed File 6 remainder sum
reproduces both NJOY's printed `ebal` sum and the final `MT301` excess to
within 1.7%. The energy non-conservation flagged by the kinematic diagnostic
is therefore a property of the evaluated File 6 product accounting, not an
artifact of NJOY's MT=301 assembly — that is the conclusion this artifact was
built to check, and it is stated as a measured agreement, not a validation.

Two difference classes remain and are preserved per-reaction and per-product
in the artifact rather than smoothed over:

- **MT=91 continuum-inelastic neutron.** The source MF=6 spectrum is a
  degenerate distribution near 0.5 eV, while NJOY prints a neutron ebar of
  ~31-33 keV and balances with `q = 0`. This is a processor-internal
  convention for continuum inelasticity that the source File 6 data alone
  cannot reproduce (the ebar differences reach 51%). MT=91's remainder still
  matches within 0.2% because the carried neutron energy is a small part of
  its balance.
- **Per-product `ebar` reconstruction differences.** NJOY's printed ebars
  differ from the tabulated-spectrum means by up to ~1% in both directions
  for identical spectra at different incident energies (for example the
  MT=107 alpha: 1.0160e6 vs 1.0028e6 eV against a fixed 1.0102e6 eV source
  mean). This indicates an internal spectrum-reconstruction convention that
  is not derivable from the ENDF text; reactions whose remainder is a small
  fraction of `E + Q` amplify these into per-reaction differences of up to
  ~10% (MT=104/105/108) while σ-weighted aggregates stay within 1.7%.

## Status and limits

- Qualification: `source_remainders_computed_unreviewed`; evidence scope
  `independent_source_calculation_unreviewed`; finding disposition
  `retained_for_independent_physical_validation`. All 43 findings remain
  open.
- This artifact does not clear ADR 0026 and does not change any suitability
  verdict. The calculation is unreviewed; a qualified nuclear-data reviewer
  must confirm the conventions listed above before the result can support
  the resume condition.
- Supported representation: MF=6 `LCT=3`, product `LAW=1`, `LANG` in {1, 2},
  `LEP` in {1, 2}, `NA` in {0, 1, 2}, File 3 and yields under histogram or
  linear-linear interpolation. Anything else fails closed as
  `unsupported representation` rather than approximating.
- `njoy verify-reaction-energy-balance` regenerates the report from the
  bound evaluation, execution receipt, and attribution and requires
  byte-for-byte equality of the report contents.
