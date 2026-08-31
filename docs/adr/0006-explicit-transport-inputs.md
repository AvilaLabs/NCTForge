# ADR 0006: Explicit Transport Material and Source Inputs

**Status:** Accepted and implemented

**Date:** 2026-08-31

## Context

The frozen `NF-BNCT-001` material is stated as natural H, C, N, and O plus
pure B-10. Natural-element convenience APIs are not a stable interchange
boundary. OpenMC expands elements using its abundance and atomic-mass tables,
then may alter the expansion according to the nuclides available in a selected
`cross_sections.xml`. Another backend may use a different table or policy.

The original transport case also called its requested work count
`requested_source_particles`. That becomes ambiguous if a source samples more
than one site per history or uses non-unit statistical weights.

## Decision

Transport-ready NCTForge materials contain explicit, normalized nuclide mass
fractions. A backend may not repeat natural-element expansion or substitute a
natural-element evaluation. Missing named nuclides are a failed data preflight,
not permission to renormalize, merge, or omit a constituent.

The first contract explicitly records density in g/cm3, temperature in kelvin,
and the neutron thermal treatment. R2 permits only `free_gas`; a bound-atom
thermal-scattering model requires a new content-bound contract.

Fixed-source inputs record particle type, sites per history, statistical weight
per site, spatial distribution in Cartesian centimetres, angular distribution,
and energy in electronvolts. The first normalization profile requires exactly
one unit-weight source site per history. A transport case requests `histories`,
not loosely named particles.

## `NF-BNCT-001` expansion

For isotope `i` of natural element `E`, the resolved mass fraction is

```text
w_i = w_E * (a_i * m_i) / sum_j(a_j * m_j)
```

where `w_E` is the frozen elemental mass fraction, `a_i` is the representative
natural atom fraction, and `m_i` is the isotope atomic mass. The benchmark uses
the IUPAC 2013 representative abundances and AME2020 masses exposed by the
OpenMC 0.16.0 source tree at lightweight tag commit
`617d35a5063c57796b43428bc401e627d2011046`. The exact upstream AME2020 file
used for the calculation has SHA-256
`e8599c6d7f724fac91934e59f1b9de8fb8f63e820f4b39456b790665ed2a3307`.

The resulting ten-nuclide composition and fixed source are checked-in JSON
artifacts consumed directly by Rust contract tests.

## Consequences

- OpenMC, Geant4, and imported comparison paths receive the same material
  meaning rather than independently interpreting “natural.”
- Nuclear-data preflight must locate all ten named nuclides and the required
  photon-production data before preparation succeeds.
- Changes to abundance inputs, atomic masses, isotope list, or source
  normalization require a new material/source identifier and benchmark version.
- The material definition is exact transport input; the derivation record
  explains it but cannot override it.

## Sources

- [OpenMC 0.16.0 natural-element implementation](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/element.py)
- [OpenMC 0.16.0 abundance and atomic-mass sources](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/data/data.py)
- [IUPAC representative isotopic compositions](https://doi.org/10.1515/pac-2015-0503)
- [AME2020 atomic-mass evaluation](https://doi.org/10.1088/1674-1137/abddaf)
- [OpenMC material documentation](https://docs.openmc.org/en/v0.16.0/usersguide/materials.html)
