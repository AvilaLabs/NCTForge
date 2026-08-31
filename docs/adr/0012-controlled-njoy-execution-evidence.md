# ADR 0012: Controlled NJOY Execution Evidence

**Status:** Accepted and implemented; first canonical execution rejected by
diagnostics

**Date:** 2026-08-31

## Context

ADR 0011 freezes deterministic NJOY input bytes, but a successful processor
exit does not establish which executable ran, whether its environment was
controlled, which files it produced, or whether HEATR's optional kinematic
checks passed. Accepting outputs from an ad hoc shell invocation would leave a
gap at the most scientifically consequential transformation in the first
response-generation path.

NJOY also reports kinematic-limit findings in formatted text rather than its
process exit status. Those findings must be represented as structured evidence
and must not be lost when a run is rejected.

## Decision

The `nctforge-njoy` crate owns a controlled NJOY2016.78 execution boundary. The
`njoy execute` command:

- regenerates the expected input bundle from all content-bound source
  documents and requires exact byte equality with the supplied bundle;
- refuses symlinks, missing files, extra files, changed bytes, an existing
  destination, or an output path overlapping an input root;
- binds the real processor executable and every explicitly declared runtime
  support file by filename, size, and SHA-256;
- invokes one nuclide at a time with no inherited environment, locale `C`,
  timezone `UTC`, and a finite wall-clock timeout;
- requires a successful exit, the `njoy 2016.78` banner, empty standard error,
  no NJOY fatal-error marker, and the diagnostic report;
- requires exactly the expected run files, special MF=3 response sections, and
  byte-identical production and diagnostic PENDF tapes;
- maps every `low` or `high` marker in the final KERMA table to its energy and
  response MT, failing if any marker cannot be mapped; and
- hashes every input, log, report, and tape into an immutable JSON receipt.

The complementary `njoy verify-execution` command accepts an external receipt
as its trust anchor and rechecks its schema, internal invariants, exact file
set, sizes, and hashes. It rejects path traversal, symlinks, extra artifacts,
changed artifacts, and a non-identical receipt inside the execution root.

Two receipt qualifications are currently possible:

- `execution_observed_unreviewed`: every mechanical and kinematic gate passed,
  but scientific review has not occurred; or
- `execution_observed_diagnostics_failed`: execution evidence was preserved,
  but at least one run exceeded a kinematic limit.

Neither state qualifies a neutron response table. The execution command exits
nonzero for the second state after writing the receipt. Verification of an
authentic rejected evidence root succeeds while continuing to report its
rejected scientific qualification.

## Canonical execution result

The first complete `NF-BNCT-001` execution used the official NJOY2016.78 source
at commit `71a76bc6345fa15f36bacc816ae7900714345d97`. The executable contained
the NJOY core statically and had SHA-256
`8a37cf70cf801b0c30ba70735f53a7b6aa51f18e53a10071fa0aff3341174c2d`;
its six dynamically loaded system runtime files are separately bound in the
receipt.

All ten processes exited successfully, wrote empty standard error, emitted the
required special MF=3 sections, and produced byte-identical production and
diagnostic PENDF. Six nuclides passed the kinematic check. N-15, O-16, O-17,
and O-18 produced a total of 72 structured MT 301 violations, so the receipt is
`execution_observed_diagnostics_failed` and response-table generation remains
blocked.

The frozen receipt is
`benchmarks/synthetic/nf-bnct-001/transport/provenance/njoy2016-78-execution-receipt.json`
with SHA-256
`65a21b57507e76a68b77349e92390ae03ebb8c38f6ed6cee66197aa5ee4adea7`.
The roughly 70 MiB raw execution root is retained outside Git and still needs a
versioned release-artifact publication; the receipt alone does not pretend the
raw outputs are already publicly archived.

ADR 0013 derives a separate transported-photon suitability report from the
receipt-bound processor logs. It structures the missing/incomplete photon-data
messages for the same four nuclides and independently confirms that this source
set cannot advance to response-table generation under the frozen component
definition.

A second controlled execution reproduced all 50 NJOY output tapes and all 72
structured findings byte-for-byte. NJOY's standard output and report files
were not byte-identical because they contain elapsed-time text, so each
execution receipt binds the logs actually observed rather than asserting log
byte determinism.

## Consequences

- A normal process exit cannot conceal a failed scientific diagnostic.
- Rejected data remain inspectable evidence without being promoted to a
  response table.
- Processor and host-runtime identity are explicit, although declared support
  artifacts are not claimed to be a proof of the complete operating-system
  dependency closure.
- The current evaluated-data profile must be investigated or replaced through
  a new versioned profile; NCTForge will not clip values, suppress diagnostics,
  or introduce an undocumented Q-value override.
- This checkpoint improves reproducibility but does not resolve the existing
  archive-equivalence question or independently review the physical method.

## Primary sources

- [NJOY2016.78 release](https://github.com/njoy/NJOY2016/releases/tag/2016.78)
- [NJOY2016.78 HEATR source and kinematic-check implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
- [NJOY2016 upstream Test24 input](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/tests/24/input)
