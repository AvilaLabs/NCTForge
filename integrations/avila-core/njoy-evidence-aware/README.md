# Avila Core evidence-aware gate

This is the first NCTForge workflow driven through Avila Core. It does not move
NCTForge's scientific rules into Core. NCTForge regenerates and verifies the
five-document evidence-aware assessment; Core binds the exact executable and
inputs, records a receipt, extracts the result, and evaluates one explicit
research requirement.

The frozen JEFF-4.0 case is expected to execute successfully and produce:

- categorical evidence: `transported_photon_kerma_rejected`;
- remaining unexplained in-domain kinematic findings: `102`; and
- Core verdict: `FAIL` against the declared limit of zero.

A scientific rejection is therefore a valid checked result, not a process
failure. If NCTForge cannot verify the evidence chain or cannot emit its result,
Core instead rejects the execution and reports a stable runtime finding.

## Run the frozen case

From a workspace containing sibling `NCTForge` and `Avila-Core` repositories:

```sh
cd NCTForge
cargo build -p nctforge-cli --bin nctforge

cd ../Avila-Core
cargo run -p avila-core-cli -- run \
  ../NCTForge/integrations/avila-core/njoy-evidence-aware \
  --source-root nctforge=../NCTForge \
  --source-root case=../NCTForge/integrations/avila-core/njoy-evidence-aware \
  --capability nctforge-cli=../NCTForge/target/debug/nctforge \
  --workspace ../NCTForge/runs/avila-core-njoy-evidence-aware \
  --log ../NCTForge/runs/avila-core-attempts.jsonl
```

The specimen pins the exact local Linux debug binary used to freeze it. If a
rebuild has another SHA-256, Core will refuse execution. Inspect the change and
deliberately update the capability identity and committed producer identities;
do not bypass the pin.

## Use it during the next investigation

All six scientific inputs are declared free. Pass a changed report with
`--input NAME=PATH`; pass every changed member of the evidence chain in the
same run. Core then withholds the frozen claims, executes NCTForge over the new
bytes, binds the new result by receipt, and marks replay against the reference
case as not applicable.

The generated `claims.json` preserves the categorical qualification separately
from the exact count. The count drives the technical requirement; the category
is not encoded as an invented number and does not by itself become a Core
verdict.

The observed development effect and explicitly qualified counterfactual are
kept in the [Avila Core use-case
record](../../../docs/research/AVILA_CORE_USE_CASE_LOG.md).
