# OpenMC Nuclear-Data Profiles

These reviewed profiles identify external data artifacts; the repository does
not redistribute either archive.

- `openmc-endfb81-official-library.json` identifies the 9.66 GB processed OpenMC
  ENDF/B-VIII.1 transport library. OpenMC publishes no digest for this object,
  so acquisition can establish observed SHA-256 identity but remains
  `acquisition_only`.
- `endfb81-neutron-evaluations.json` identifies the current 343.7 MB NNDC source
  archive used for response generation. Its current publisher MD5 differs from
  the digest pinned by OpenMC's generation recipe; both identities are retained
  and the candidate is not treated as scientifically equivalent.

Probe before any transfer:

```sh
cargo run -p nctforge-cli -- openmc data probe \
  --profile profiles/openmc/endfb81-neutron-evaluations.json
```

Acquisition requires the exact probed byte count and never overwrites completed
output. After selecting the ten `NF-BNCT-001` members, verify all bindings and
file hashes with:

```sh
cargo run -p nctforge-cli -- openmc data verify-selection \
  --selection benchmarks/synthetic/nf-bnct-001/transport/evaluated-neutron-source-selection.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --profile profiles/openmc/endfb81-neutron-evaluations.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/endfb81-neutron-acquisition-receipt.json \
  --evaluations-directory PATH_TO_EXACT_SELECTION
```

See [ADR 0010](../../docs/adr/0010-verifiable-nuclear-data-acquisition.md) and
the [archive drift record](../../docs/research/ENDFB81_NEUTRON_ARCHIVE_DRIFT.md)
for the qualification boundary.
