# Official OpenMC ENDF/B-VIII.1 Processed-Data Findings

**Observed:** 2026-08-31

**Disposition:** Transport selection verified; response qualification remains
blocked by explicit O-17/O-18 photon-data findings

## Outcome

NCTForge acquired the official OpenMC ENDF/B-VIII.1 processed distribution,
selected the exact data required by `NF-BNCT-001`, and inspected the HDF5
structures OpenMC 0.16.0 consumes. The case-scoped selection passes the Rust
artifact, mapping, and material-capability preflight. This establishes a usable
and reproducible transport-data identity; it does not qualify the independently
generated neutron response tables.

The official HDF5 MT 301 total-heating responses were then compared directly
with the controlled NJOY2016.78 production PENDF outputs for all ten benchmark
nuclides. Every grid corresponds pointwise without interpolation, every response
passes the `1e-6` relative tolerance, and the largest relative difference is
`4.892059192870681e-7`. The differences are consistent with ENDF decimal
serialization precision.

## Frozen evidence

| Evidence | Identity |
| --- | --- |
| Official archive | 9,661,406,540 bytes; SHA-256 `b7ad59cb4a3d76d8a291326093a98507f8d24b6e6af629116d3f7dc85f83c4cb` |
| Acquisition receipt | SHA-256 `71e5b7ded6e031f3c9b3c9f75b2f0cdc6f02428d1a5f8fa8a843e738d2ea7fb8`; `acquisition_only` |
| Case manifest | SHA-256 `3eaae09921172199c34f3fb236ae082ea5ace4567e0e04d2afcce357add73fb1` |
| Selected artifacts | `cross_sections.xml`, ten neutron HDF5 tables, five element photon HDF5 tables |
| NJOY execution receipt | SHA-256 `65a21b57507e76a68b77349e92390ae03ebb8c38f6ed6cee66197aa5ee4adea7` |
| MT 301 comparison report | SHA-256 `e9b1ffc5e70e3e489f23f9e185d12a5edeb7525161eb3b81470233d33f36f1e7` |

The official publisher supplies no digest for the processed archive. The
receipt therefore records observed byte identity only and cannot be promoted
beyond `acquisition_only`.

## B-10 photon-production semantics

The first preflight implementation incorrectly required photon production to
be attached directly to B-10 MT 107. ENDF defines MT 107 as the redundant sum of
the MT 800--849 alpha-production states. In the official OpenMC table, the
prompt-photon product is attached to nonredundant MT 801, while MT 107 remains
available as the aggregate reaction. OpenMC accumulates photon products from
the individual reactions it transports. The material capability contract now
accepts photon production on either aggregate MT 107 or discrete branch MT 801,
and retains the requirement that the aggregate MT 107 reaction itself exists.

This was a preflight false negative, not missing B-10 physics. A regression test
freezes both accepted representations and rejects a table containing neither.

## Remaining O-17/O-18 limitation

The O-17 and O-18 neutron tables contain no photon-producing reactions. For
both nuclides, OpenMC MT 301 total heating is effectively equal to MT 901 local
heating within the comparison tolerance. This independently confirms the local
photon-deposition fallback observed in the NJOY diagnostics.

The result narrows the scientific blocker:

- official OpenMC transport data identity and basic capabilities pass;
- the current evaluated-source archive produces the same total-heating curves
  as the official OpenMC tables;
- substituting the official processed library does not create missing
  O-17/O-18 secondary-photon information; and
- NCTForge must define and independently review an explicit response treatment
  before publishing neutron component tables or reference dose arrays.

The comparison report deliberately declares
`comparison_only_not_response_qualification`. No response-set review state or
transport capability flag is raised by this finding.

## Reproduction

The checked artifacts are:

- `benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-acquisition-receipt.json`;
- `benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json`; and
- `benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-njoy-mt301-comparison.json`.

`scripts/inspect-openmc-data.py` regenerates the manifest from the acquired
archive and selected data root. `scripts/compare-openmc-njoy-heating.py` verifies
the compared HDF5 and PENDF artifacts and the external receipt trust anchor
before parsing MF 3/MT 301 and selecting the 293.6 K HDF5 response. Both tools
refuse to overwrite their output.

## Primary sources

- [OpenMC official data libraries](https://openmc.org/data/)
- [OpenMC photon-production methods](https://docs.openmc.org/en/v0.16.0/methods/photon_physics.html)
- [OpenMC 0.16.0 incident-neutron data implementation](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/data/neutron.py)
- [OpenMC 0.16.0 neutron photon-production loading](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/nuclide.cpp)
- [NNDC ENDF reaction identifiers](https://www.nndc.bnl.gov/endf/help.html)
- [ENDF-6 Formats Manual for ENDF/B-VIII.0](https://www.nndc.bnl.gov/endf-b8.0/endf-manual-viii.0.pdf)
