# ADR 0008: OpenMC Nuclear-Data Capability Preflight

**Status:** Accepted and implemented; official data manifest pending

**Date:** 2026-08-31

## Context

A `cross_sections.xml` entry proves only that OpenMC can locate a named HDF5
file. It does not prove that the file is the intended release, has the selected
temperature and heating response, contains a required reaction, or carries the
secondary-photon data needed by the BNCT component definition. Treating a
missing reaction as a zero score would create a plausible but invalid result.

The official OpenMC ENDF/B-VIII.1 distribution includes incident-neutron,
photoatomic, atomic-relaxation, and thermal-scattering data and was processed
with NJOY2016.78. NCTForge's free-gas baseline needs no thermal-scattering table,
but coupled photon transport still requires element-specific photon data for
every element in the material.

## Decision

The OpenMC 0.16.0 adapter accepts only a case-scoped nuclear-data manifest. The
manifest pins:

- OpenMC `0.16.0` and tag commit
  `617d35a5063c57796b43428bc401e627d2011046`;
- ENDF/B-VIII.1 and OpenMC nuclear-data HDF5 format `3.0`;
- the source distribution URI and archive SHA-256;
- `cross_sections.xml` and every selected HDF5 file by normalized path and
  SHA-256;
- each neutron table's atomic-weight ratio, temperatures, corresponding
  incident-energy bounds, reaction MTs, and reaction MTs carrying photon
  products; and
- each photon table's reaction MTs, atomic-relaxation data, and Compton-profile
  data.

`scripts/inspect-openmc-data.py` extracts those facts directly from the HDF5
structures consumed by OpenMC. Its own SHA-256 and its Python, NumPy, h5py, and
HDF5 runtime versions are recorded in the manifest. Rust independently validates
the manifest, verifies every selected file without permitting symlink escape,
and cross-checks every table against exactly one matching
`cross_sections.xml` mapping.

For `NF-BNCT-001`, preflight requires exactly its ten neutron nuclides and the
H, B, C, N, and O photon tables. Every neutron table must contain MT 301 at a
temperature within `0.5 K` of `293.6 K`; the tolerance matches OpenMC's
integer-kelvin table selection and will be written explicitly to settings.
B-10 must contain MT 107, N-14 MT 103, H-1 MT 102 photon production, and B-10
MT 107 photon production. Photon tables must contain coherent MT 502,
incoherent MT 504, and photoelectric MT 522 data, plus atomic-relaxation and
Compton-profile structures.

At the selected temperature, the preflight also calculates the common neutron
transport interval in the same way as OpenMC: the maximum lower grid bound and
minimum upper grid bound across every loaded nuclide. Input generation requires
the reviewed component-response functions to cover that complete interval, not
only the monoenergetic source value.

The manifest is deliberately not hand-authored. The first checked-in manifest
will be generated from the downloaded archive, mechanically validated, and
reviewed before it can be referenced by a response set or prepared run.

## Consequences

- A renamed, replaced, tampered, path-escaping, incomplete, or wrongly mapped
  table fails before transport preparation.
- Explicit nuclide inputs cannot be silently collapsed to an available natural
  element or a smaller isotope set.
- The atomic-weight ratios used by OpenMC's weight-to-atom conversion remain
  available to reproduce the response-generation normalization.
- This preflight establishes data identity and declared capabilities; it does
  not validate evaluated nuclear physics or qualify a response table.
- HDF5 inspection remains an evidence-producing setup step. NCTForge's Rust
  runtime does not take a system HDF5 linkage dependency.

## Primary sources

- [OpenMC official data libraries](https://openmc.org/data/)
- [OpenMC cross-sections listing format](https://docs.openmc.org/en/v0.16.0/io_formats/cross_sections.html)
- [OpenMC 0.16.0 incident-neutron HDF5 reader](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/data/neutron.py)
- [OpenMC 0.16.0 photon HDF5 reader](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/photon.cpp)
- [OpenMC 0.16.0 data-format and temperature constants](https://github.com/openmc-dev/openmc/blob/v0.16.0/include/openmc/constants.h)
