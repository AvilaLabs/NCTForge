# ADR 0007: Partial-KERMA Response Generation

**Status:** Method and canonical inputs accepted; first processing evidence rejected

**Date:** 2026-08-31

## Context

OpenMC exposes a total neutron `heating` response but not a reaction-specific
heating cross section. A BNCT component partition therefore cannot be obtained
by attaching a reaction filter to that total response. At the same time, using
only fixed textbook Q values would discard evaluated energy dependence and the
photon-production information needed to avoid local/transported double counting.

NJOY HEATR generates pointwise KERMA coefficients in eV-barns. Its documented
energy-balance method subtracts energy carried by secondary neutrons and photons
from the energy available to a reaction. HEATR writes a requested reaction's
partial KERMA at `reaction MT + 300` and always writes total heating at MT 301.

## Decision

The first response-generation method is frozen as
`nctforge.nf-bnct-001.response-generation.v1`:

- NJOY `2016.78`, source commit
  `71a76bc6345fa15f36bacc816ae7900714345d97`;
- ENDF/B-VIII.1 incident-neutron evaluations;
- RECONR/BROADR fractional tolerance `0.001` at `293.6 K`;
- HEATR with transported photons (`local=0`), no Q-value overrides, and
  kinematic consistency checks enabled;
- B-10 MT 107 partial KERMA from MT 407;
- N-14 MT 103 partial KERMA from MT 403;
- material total neutron KERMA from each nuclide's MT 301; and
- the conventional `hydrogen` response as material total minus the boron and
  nitrogen responses.

ADR 0011 implements the content-bound, byte-stable NJOY input generator and
freezes the ten canonical decks. ADR 0012 implements controlled execution and
artifact verification. The first complete run produced all expected sections,
but HEATR reported 72 MT 301 kinematic-limit violations across N-15, O-16,
O-17, and O-18. The preserved receipt is therefore
`execution_observed_diagnostics_failed`; no response table was generated. ADR
0013 also rejects the selection for transported-photon KERMA after structuring
NJOY's missing or incomplete photon-data messages for the same four nuclides.

The response generator uses the exact material weight fractions and the atomic
weight ratios in the selected OpenMC HDF5 tables, matching OpenMC's conversion
of `wo` material inputs to atom densities. If `N_i` is an atom density in
atom/barn-cm, `k_i,c(E)` is a HEATR coefficient in eV-barns, and `rho` is the
material density in kg/cm3, the material response is

```text
R_c(E) = (1.602176634e-19 J/eV) / rho
         * sum_i N_i * k_i,c(E)                         [Gy cm2]
```

All total and partial pointwise knots are placed on one union grid without
downsampling. OpenMC applies linear-linear interpolation. A mesh `flux` score
is track length per source history; dividing by mesh-cell volume gives fluence
per source neutron, which is folded with `R_c(E)` to obtain Gy/source neutron.

Photon dose remains the coupled photon `heating` estimator at the deposition
location. The dedicated physical total remains coupled `heating` without a
particle filter.

The backend-neutral response-set contract binds the component profile,
material, nuclear-data manifest, and generation method by SHA-256. It requires
a strictly increasing pointwise grid covering the declared transport domain,
finite non-negative curves, and pointwise closure of boron, nitrogen, and
hydrogen against retained material MT 301. A table may be represented as
`generated_unreviewed` for inspection, but dose folding requires the
`independently_reviewed` state and a content-bound review artifact. Every
normalized physical-dose bundle binds the exact response set it used.

## Qualification gates

The checked-in method remains deliberately marked
`method_frozen_tables_pending`. The first execution fails gate 3 below. No
response value is qualified until all of the following evidence exists:

1. every source ENDF evaluation, generated PENDF, NJOY input, log, selected
   HDF5 table, and output table is SHA-256 bound;
2. B-10 MT 107 and N-14 MT 103 have the evaluated photon/secondary-product data
   needed for meaningful partial KERMA over the active energy domain;
3. HEATR reports no unexplained energy-balance or kinematic-limit violation in
   that domain;
4. the three component curves and residual are finite and non-negative;
5. pointwise `boron + nitrogen + hydrogen` closes to material MT 301 within
   floating-point tolerance;
6. the regenerated total response agrees with the official OpenMC HDF5 heating
   response after matching atomic-weight-ratio normalization;
7. reaction-rate times independently calculated mean charged-product energy
   checks B-10 and N-14 at declared energies; and
8. a second reviewer approves the derivation before candidate transport runs.

An evaluation that fails these checks is investigated or replaced only by a
new versioned nuclear-data profile. NCTForge will not clip a negative value,
apply an undocumented Q override, or relabel a missing response as zero.

## Consequences

- The primary component partition uses evaluated energy and photon-production
  data while remaining reproducible outside OpenMC.
- `hydrogen` includes every remaining non-photon neutron-heating contributor,
  not just H-1 elastic recoil.
- The full grid avoids an unquantified table-reduction error in the first case;
  later compression requires its own error bound.
- Shared-history estimator comparisons remain correlated diagnostics rather
  than independent validation.

## Primary sources

- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [NJOY2016.78 release](https://github.com/njoy/NJOY2016/releases/tag/2016.78)
- [OpenMC 0.16.0 NJOY processing templates](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/data/njoy.py)
- [OpenMC energy-function filter](https://docs.openmc.org/en/v0.16.0/pythonapi/generated/openmc.EnergyFunctionFilter.html)
- [OpenMC material weight-fraction normalization](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/material.cpp)
