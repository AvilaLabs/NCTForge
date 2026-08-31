# JEFF-4.0 response-treatment candidate

This directory records the first controlled alternate-library assessment for
`NF-BNCT-001`. JEFF-4.0 remains rejected for the frozen transported-photon
KERMA requirement. The immutable log-only v0.1 report rejects six nuclides; the
source-aware v0.2 report correctly clears N-15's valid File 13 alternative and
rejects five. The domain-aware v0.3 report binds the exact material to the
common OpenMC interval and clears O-16 because its sole finding is at 30 MeV,
strictly above the 20 MeV domain; the finding remains preserved. The
independent MF=6 capture-balance gate rejects N-15 again because 33 of 37
source nodes miss the Q-value budget by more than 1%. The candidate therefore
remains unsuitable for response-table generation. The independent H-2 LAW=7
source calculation now shows that all 53 active nodes normalize and leave
positive mean energy for the implicit proton. That narrows the H-2 problem to
processor attribution. The receipt-bound comparison attributes all 15 H-2
findings to NJOY's excluded energy-balance remainder with zero failed nodes.
Evidence-aware suitability v0.4 now binds that result and clears only H-2. It
also integrates the independent N-15 capture-balance rejection. C-13, N-15,
O-17, and O-18 therefore remain rejected, with 102 unexplained in-domain
kinematic findings across C-13, O-17, and O-18. The overall candidate remains
unsuitable for response-table generation.

The repository retains only manifests, deterministic NJOY decks, and evidence
receipts. It does not redistribute the 608 MB publisher archive, extracted
evaluations, processor binary, or NJOY output tapes.

Verify the selected evaluations against the matched publisher acquisition:

```sh
cargo run -p nctforge-cli -- openmc data verify-selection \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/evaluated-neutron-source-selection.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --profile profiles/njoy/jeff40-neutron-evaluations.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-neutron-acquisition-receipt.json \
  --evaluations-directory PATH_TO_EXACT_JEFF40_SELECTION
```

With the preserved external execution directory, independently verify the
processor evidence and regenerate its suitability decision:

```sh
cargo run -p nctforge-cli -- njoy verify-execution \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION

cargo run -p nctforge-cli -- njoy verify-suitability \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --suitability-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-suitability.json

cargo run -p nctforge-cli -- njoy verify-photon-inventory \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/evaluated-neutron-source-selection.json \
  --evaluations-directory PATH_TO_EXACT_JEFF40_SELECTION \
  --inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-endf-photon-production-inventory.json

cargo run -p nctforge-cli -- njoy verify-source-aware \
  --legacy-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-suitability.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --input-manifest benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/njoy/nctforge-njoy-input-manifest.json \
  --photon-inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-endf-photon-production-inventory.json \
  --source-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-source-aware-suitability.json

cargo run -p nctforge-cli -- njoy verify-photon-moments \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/evaluated-neutron-source-selection.json \
  --evaluations-directory PATH_TO_EXACT_JEFF40_SELECTION \
  --photon-inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-endf-photon-production-inventory.json \
  --moment-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-file13-continuum-photon-moments.json

cargo run -p nctforge-cli -- njoy verify-photon-moment-comparison \
  --moment-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-file13-continuum-photon-moments.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-vs-njoy2016-78-continuum-photon-moments.json

cargo run -p nctforge-cli -- njoy verify-capture-photon-balance \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/evaluated-neutron-source-selection.json \
  --evaluations-directory PATH_TO_EXACT_JEFF40_SELECTION \
  --photon-inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-endf-photon-production-inventory.json \
  --balance-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt102-capture-photon-balance.json

cargo run -p nctforge-cli -- njoy verify-capture-photon-moment-comparison \
  --balance-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt102-capture-photon-balance.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-vs-njoy2016-78-mf6-capture-photon-moments.json

cargo run -p nctforge-cli -- openmc data verify-transport-domain \
  --manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --transport-domain benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-neutron-transport-domain.json

cargo run -p nctforge-cli -- njoy verify-domain-aware \
  --source-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-source-aware-suitability.json \
  --legacy-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-suitability.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --input-manifest benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/njoy/nctforge-njoy-input-manifest.json \
  --nuclear-data-manifest benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-endfb81-processed-data-manifest.json \
  --material benchmarks/synthetic/nf-bnct-001/transport/material.json \
  --transport-domain benchmarks/synthetic/nf-bnct-001/transport/provenance/openmc-neutron-transport-domain.json \
  --domain-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-domain-aware-suitability.json

cargo run -p nctforge-cli -- njoy verify-law7-implicit-residual \
  --selection benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/evaluated-neutron-source-selection.json \
  --evaluations-directory PATH_TO_EXACT_JEFF40_SELECTION \
  --photon-inventory benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-endf-photon-production-inventory.json \
  --residual-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt16-law7-implicit-residual.json

cargo run -p nctforge-cli -- njoy verify-law7-implicit-residual-comparison \
  --residual-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt16-law7-implicit-residual.json \
  --receipt benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-execution-receipt.json \
  --execution-directory PATH_TO_JEFF40_EXECUTION \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-vs-njoy2016-78-law7-implicit-residual.json

cargo run -p nctforge-cli -- njoy verify-evidence-aware \
  --domain-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-domain-aware-suitability.json \
  --law7-residual-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt16-law7-implicit-residual.json \
  --law7-comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-vs-njoy2016-78-law7-implicit-residual.json \
  --capture-balance-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-mf6-mt102-capture-photon-balance.json \
  --capture-comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/jeff40-vs-njoy2016-78-mf6-capture-photon-moments.json \
  --evidence-aware-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-evidence-aware-suitability.json
```

The checked comparison is self-contained over the two content-addressed
suitability reports:

```sh
cargo run -p nctforge-cli -- njoy verify-comparison \
  --baseline-report benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-transported-photon-suitability.json \
  --candidate-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/njoy2016-78-transported-photon-suitability.json \
  --comparison-report benchmarks/synthetic/nf-bnct-001/transport/candidates/jeff40/provenance/endfb81-vs-jeff40-response-treatment-comparison.json
```

See [the detailed finding](../../../../../../docs/research/JEFF40_RESPONSE_TREATMENT_FINDINGS.md)
and [ADR 0016](../../../../../../docs/adr/0016-versioned-response-treatment-candidates.md)
plus [ADR 0017](../../../../../../docs/adr/0017-source-aware-photon-production-suitability.md)
and [ADR 0018](../../../../../../docs/adr/0018-independent-continuum-photon-moments.md),
then [ADR 0019](../../../../../../docs/adr/0019-independent-mf6-capture-photon-balance.md)
and [ADR 0020](../../../../../../docs/adr/0020-content-bound-transport-domain-suitability.md),
followed by [ADR 0021](../../../../../../docs/adr/0021-independent-law7-implicit-residual-balance.md).
The receipt-bound attribution is specified by
[ADR 0022](../../../../../../docs/adr/0022-law7-processor-attribution.md), and
the integrated v0.4 decision by
[ADR 0023](../../../../../../docs/adr/0023-reaction-evidence-aware-suitability.md).
