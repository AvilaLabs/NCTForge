# ENDF/B-VIII.1 + TENDL-2025 mixed response-treatment candidate

This directory records the first mixed-source assessment for `NF-BNCT-001`,
and the first use of selection schema `0.3.0`. TENDL-2025 distributes no
hydrogen evaluations, so the selection pairs the six clean ENDF/B-VIII.1
evaluations (B-10, C-12, C-13, H-1, H-2, N-14) with TENDL-2025 evaluations
for the four baseline failures (N-15, O-16, O-17, O-18). The TENDL archive
publishes no digest; its acquisition is bound by exact byte count, SHA-256,
transfer metadata, and receipt.

**The candidate remains rejected.** The six shared nuclides reproduce the
baseline exactly; all four TENDL-2025 substitutions still emit high-direction
MT 301 kinematic diagnostics (77 violations vs the baseline's 72; 70 remain
in the bound transport domain). Every TENDL evaluation also carries
duplicated File 3 energy-grid points — a source-format defect that NJOY
tolerates but the strict independent parsers reject, so the capture-balance,
continuum-moment, and evidence-aware layers are not computable for this
candidate. The verified baseline comparison resolves zero rejections and
introduces none.

The repository retains only manifests, deterministic NJOY decks, and evidence
receipts. It does not redistribute the 3.28 GiB publisher archive, extracted
evaluations, processor binary, or NJOY output tapes.

Verify the selected evaluations against both bound acquisitions (the
`--profile`/`--receipt` arguments pair positionally):

```sh
cargo run -p openbnct-cli -- openmc data verify-selection \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/evaluated-neutron-source-selection.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --profile profiles/openmc/endfb81-neutron-evaluations.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/provenance/endfb81-neutron-acquisition-receipt.json \
  --profile profiles/njoy/tendl2025-neutron-evaluations.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/tendl2025-neutron-acquisition-receipt.json \
  --evaluations-directory PATH_TO_EXACT_MIXED_SELECTION
```

With the preserved external execution directory, independently verify the
processor evidence and regenerate the suitability decisions:

```sh
cargo run -p openbnct-cli -- njoy verify-execution \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_MIXED_EXECUTION

cargo run -p openbnct-cli -- njoy verify-suitability \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_MIXED_EXECUTION \
  --suitability-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-suitability.json

cargo run -p openbnct-cli -- njoy verify-photon-inventory \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/evaluated-neutron-source-selection.json \
  --evaluations-directory PATH_TO_EXACT_MIXED_SELECTION \
  --inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/endfb81-tendl2025-endf-photon-production-inventory.json

cargo run -p openbnct-cli -- njoy verify-source-aware \
  --legacy-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-suitability.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_MIXED_EXECUTION \
  --input-manifest benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/njoy/openbnct-njoy-input-manifest.json \
  --photon-inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/endfb81-tendl2025-endf-photon-production-inventory.json \
  --source-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-source-aware-suitability.json

cargo run -p openbnct-cli -- njoy verify-domain-aware \
  --source-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-source-aware-suitability.json \
  --legacy-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-suitability.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_MIXED_EXECUTION \
  --input-manifest benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/njoy/openbnct-njoy-input-manifest.json \
  --nuclear-data-manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --transport-domain benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-neutron-transport-domain.json \
  --domain-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-domain-aware-suitability.json

cargo run -p openbnct-cli -- njoy verify-energy-balance-attribution \
  --domain-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-domain-aware-suitability.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_MIXED_EXECUTION \
  --attribution-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/endfb81-tendl2025-o17-njoy-energy-balance-attribution.json
```

The checked comparison is self-contained over the two content-addressed
suitability reports:

```sh
cargo run -p openbnct-cli -- njoy verify-comparison \
  --baseline-report benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-transported-photon-suitability.json \
  --candidate-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-suitability.json \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/endfb81-vs-endfb81-tendl2025-response-treatment-comparison.json

cargo run -p openbnct-cli -- njoy check-candidate-comparison \
  --baseline-report benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-transported-photon-suitability.json \
  --candidate-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/njoy2016-78-transported-photon-suitability.json \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/provenance/endfb81-vs-endfb81-tendl2025-response-treatment-comparison.json \
  --output NEW-CANDIDATE-COMPARISON-CHECK.json
```

See [the detailed
findings](../../../../../../docs/research/TENDL2025_MIXED_CANDIDATE_FINDINGS.md),
[ADR 0028](../../../../../../docs/adr/0028-mixed-evaluated-neutron-source-selections.md)
for the selection contract, and
[ADR 0029](../../../../../../docs/adr/0029-tendl2025-mixed-source-candidate.md)
for the executed outcome.
