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

Verify a selectively extracted official processed-data selection against the
checked case manifest and material contract with:

```sh
cargo run -p nctforge-cli -- openmc data verify-manifest \
  --manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --data-root PATH_TO_SELECTED_OPENMC_DATA \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json
```

The checked comparison report was regenerated from that selection and the
controlled NJOY execution using:

```sh
uv run --with-requirements scripts/requirements-openmc-data-inspector.txt \
  scripts/compare-openmc-njoy-heating.py \
  --manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --data-root PATH_TO_SELECTED_OPENMC_DATA \
  --execution-receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json \
  --execution-root PATH_TO_NJOY_EXECUTION \
  --report-id nctforge.nf-bnct-001.openmc-endfb81-vs-njoy2016-78-mt301.v1 \
  --output NEW_REPORT_PATH
```
