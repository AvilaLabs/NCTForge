# Avila Core diagnostic-triage gate

This NCTForge workflow is driven through Avila Core without moving NCTForge's
scientific rules into Core. NCTForge regenerates and verifies the complete
evidence chain plus its diagnostic triage. Core binds the exact executable and
inputs, records a receipt, extracts typed evidence, and evaluates two explicit
research requirements.

The frozen JEFF-4.0 case is expected to execute successfully and report:

- all `102` original in-domain findings preserved;
- `59` findings attached to source-data-blocked C-13 and O-18 runs;
- `43` O-17 findings still requiring independent reaction diagnostics;
- response qualification `transported_photon_kerma_rejected`; and
- two Core verdicts: `FAIL` for the nonempty diagnostic queue and `FAIL` for
  the categorical candidate-status requirement.

Triage is not a waiver or a numerical explanation. It prevents work on C-13
and O-18 from being mistaken for the next useful diagnostic task while keeping
their rejection visible. A scientific rejection is a valid checked result, not
a process failure. An unverifiable evidence chain is instead an execution or
admission failure.

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
  --workspace ../NCTForge/runs/avila-core-njoy-diagnostic-triage \
  --log ../NCTForge/runs/avila-core-attempts.jsonl
```

The specimen pins the exact local Linux debug binary used to freeze it. If a
rebuild has another SHA-256, Core will refuse execution. Inspect the change and
deliberately update the capability and committed producer identities; do not
bypass the pin.

## Use it after the O-17 processor attribution

All seven scientific inputs are declared free. Pass a changed report with
`--input NAME=PATH`; pass every changed member of the evidence chain in the
same run. Core then withholds the frozen claims, runs NCTForge over the new
bytes, binds the result by receipt, and marks replay against the reference case
as not applicable.

The generated claims keep the exact queue count, response category, and triage
category distinct. The count drives a quantitative verdict. The response
category drives a closed-vocabulary categorical verdict without being encoded
as an invented number.

The O-17 attribution is intentionally not used to rewrite this package's
scientific inputs or expected result. It explains NJOY's internal accounting
but retains all 43 findings for independent physical validation. Rerunning this
unchanged contract after the attribution should therefore still produce both
FAIL verdicts. That is a regression control, not a limitation to work around.

The observed development effect and explicitly qualified counterfactual are
kept in the [Avila Core use-case
record](../../../docs/research/AVILA_CORE_USE_CASE_LOG.md).
