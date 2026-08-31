# ADR 0011: Deterministic NJOY Input Preparation

**Status:** Accepted and implemented for input preparation; execution and output qualification pending

**Date:** 2026-08-31

## Context

ADR 0007 freezes the scientific intent of the first partial-KERMA method, but a
method description alone does not prove that NJOY receives the intended cards.
Handwritten decks can bind the wrong ENDF material, request a reaction MT where
HEATR expects a partial-KERMA MT, process the wrong temperature count, or enable
local photon deposition. Those mistakes can still produce a successful process
exit and apparently plausible files.

The selected ENDF/B-VIII.1 archive is also an unresolved candidate under ADR
0010. Input preparation therefore has to preserve that qualification ceiling
while making every byte independently reviewable.

## Decision

NCTForge generates NJOY inputs in the standalone `nctforge-njoy` Rust crate.
The crate invokes no transport backend and does not execute NJOY. Before writing
anything, it verifies:

- the evaluated-source selection, material, response-generation method,
  acquisition profile, and acquisition receipt by content hash;
- every selected evaluation's exact filename, byte count, SHA-256, and ENDF
  MF=1/MT=451 material number; and
- the frozen NJOY2016.78 version and source commit, transported-photon setting,
  absence of Q-value overrides, temperature, reconstruction tolerance, and
  requested partial channels.

Output is written only to a new directory. The generator emits one byte-stable
deck per selected nuclide plus
`nctforge-njoy-input-manifest.json`. The canonical `NF-BNCT-001` bundle is
checked in under `benchmarks/synthetic/nf-bnct-001/transport/njoy/`; its
manifest SHA-256 is
`d855cce368da9b5683c1895fc8bfc618f4922e76a75efae14ab8b840bf7882ab`.
Tests regenerate all eleven files and require byte equality with that bundle.

Each deck uses this tape plan:

| Stage | Input | Output | Frozen options |
| --- | --- | --- | --- |
| RECONR | 20 | 21 | material-specific, `0.001` tolerance |
| BROADR | 20 and 21 | 22 | one temperature at `293.6 K`, `0.001` tolerance |
| HEATR production | 20 and 22 | 23 | all PENDF temperatures, transported photons, minimal print |
| HEATR diagnostic | 20 and 22 | 24 and plot 25 | one temperature, transported photons, check print option |

HEATR supplies MT 301 automatically. Card 3 requests partial-KERMA output MTs,
not their source reaction MTs: B-10 requests `407 443`, N-14 requests
`403 443`, and every other selected nuclide requests `443`. Thus the production
PENDF is expected to contain MT 301 and MT 443 for every nuclide, plus MT 407
for B-10 or MT 403 for N-14.

The manifest qualification is always `input_preparation_only`. It binds intended
inputs and decks; it does not bind a compiled processor executable, prove a
successful run, interpret HEATR diagnostics, qualify the generated PENDF, or
create a neutron response table.

## Implementation evidence

- The locally built official NJOY2016.78 source at commit
  `71a76bc6345fa15f36bacc816ae7900714345d97` passed upstream `Test24`, which
  exercises standard and diagnostic HEATR partial-KERMA processing.
- Non-qualifying local smoke executions of the canonical B-10 and N-14 decks
  exited successfully with empty standard error. Their production PENDF files
  contained exactly the required special response sections: MT 301/407/443 for
  B-10 and MT 301/403/443 for N-14.
- The initial smoke attempt intentionally remains outside the repository. It
  revealed that reaction MT 107 on HEATR card 3 does not generate MT 407; this
  was corrected before the canonical bundle was frozen.

These observations test deck semantics but are not accepted execution evidence.
A later checkpoint must execute through a controlled Rust path, bind the NJOY
binary and runtime environment, hash every log and tape, parse the diagnostic
result, and reject missing or unexpected response sections.

## Consequences

- The scientific method and exact processor inputs are reviewable without
  installing OpenMC or trusting a shell template.
- A changed source archive, ENDF material number, method document, or deck byte
  causes a deterministic failure or manifest change.
- A zero exit status alone cannot advance response-table qualification.
- The NJOY adapter remains separate from the OpenMC execution adapter even
  though both currently reuse the same acquisition and source-selection
  contracts.

## Primary sources

- [NJOY2016.78 release](https://github.com/njoy/NJOY2016/releases/tag/2016.78)
- [NJOY2016 HEATR implementation and input specification](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [NJOY2016 upstream Test24 input](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/tests/24/input)
- [OpenMC 0.16.0 NJOY processing templates](https://github.com/openmc-dev/openmc/blob/v0.16.0/openmc/data/njoy.py)
