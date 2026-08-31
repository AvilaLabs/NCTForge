# ADR 0010: Verifiable Nuclear-Data Acquisition

**Status:** Accepted and implemented; evaluated-source candidate acquired

**Date:** 2026-08-31

## Context

The official OpenMC ENDF/B-VIII.1 processed-data archive is large enough that a
browser download is not an adequate scientific setup procedure. The publisher
link currently resolves to `endfb81.tar.xz`, whose HTTP range response reports
`9,661,406,540` bytes and byte-range support. OpenMC's download page does not
publish a digest for that processed archive. A SHA-256 calculated after a
download identifies the observed bytes, but it cannot retroactively become a
publisher signature.

The evaluated ENDF/B-VIII.1 neutron sources are a separate artifact. The merged
OpenMC data-generation recipe at commit
`66cfe45ff7a3aa47a4d7805b92b3d5ab6ee018b6` downloads the NNDC neutron archive,
checks MD5 `dc622c0f1c3c4477433e698266e0fc80`, and processes the release at six
temperatures. Its exact script SHA-256 is
`b1b0342b35c3fe15f493dd4eafea457a28e441d079ec3ac831ecf7f6c5ac4f9c`.
On 2026-08-31, NNDC published MD5 `1a6abeac85bd2425df47983752687a93` at the
same stable archive URI. The current archive remains `343,724,780` bytes but is
not byte-identical to the object frozen by the OpenMC recipe. Those evaluated
sources are required for NCTForge's independently generated partial-KERMA
responses even though the processed OpenMC archive is used for transport. The
observed drift and its qualification boundary are recorded in
[`ENDFB81_NEUTRON_ARCHIVE_DRIFT.md`](../research/ENDFB81_NEUTRON_ARCHIVE_DRIFT.md).

## Decision

NCTForge stores reviewed acquisition profiles under `profiles/openmc/` and
implements probing and acquisition in Rust. A profile binds the release page,
source URI, permitted HTTPS redirect-host suffixes, filename, media type,
observed byte count, current publisher digest when available, known historical
digests, and the pinned upstream OpenMC generation recipe. Adding the digest
history is a breaking profile-schema change to
`nctforge.data-acquisition-profile/0.2.0`. The raw SHA-256 of the frozen
processed-library profile is
`237a45d81b7f57dbbb0f1acace641e5dcbda13757e9bfcef686b4daf145ecab7`;
Git attributes force LF checkout for these raw-byte trust anchors.

The CLI has two explicit operations:

```text
nctforge openmc data probe --profile PROFILE
nctforge openmc data acquire --profile PROFILE \
  --output-directory DIRECTORY --confirm-size-bytes EXACT_BYTES
```

`probe` sends `Range: bytes=0-0`, validates the complete size and redirect
boundary, and retains no response body. `acquire` requires the user to repeat
the exact byte count. It:

- follows at most ten redirects and permits only HTTPS hosts covered by the
  profile allowlist;
- requests identity encoding so hashes apply to the published archive bytes;
- permits long transfers without a total-request deadline while failing a
  connection attempt after 30 seconds;
- writes a new `.part` file and never overwrites a completed artifact or
  receipt;
- resumes only when the server advertises ranges and returns the exact requested
  `Content-Range` and remaining `Content-Length`;
- rejects byte-count growth, truncation, and publisher-digest mismatch;
- syncs the completed bytes before publishing them under the final filename;
  and
- emits a JSON receipt containing the profile SHA-256, archive SHA-256, transfer
  origin, resume offset, response identity headers, publisher-digest status, and
  completion time.

All receipts declare `acquisition_only`. For the processed OpenMC archive the
publisher-digest status must remain `unavailable`; matching a locally computed
SHA-256 does not raise that state. For the NNDC source archive, acquisition
checks the MD5 currently published by NNDC and records SHA-256. The different
MD5 frozen by the OpenMC recipe is retained as digest history rather than
silently replaced. MD5 is an upstream byte identifier, not a modern security
signature or proof of scientific equivalence.

The OpenMC HDF5 inspector requires both the profile and receipt. It independently
rehashes the archive and binds the profile and receipt hashes into nuclear-data
manifest schema `0.3.0`. Rust preflight accepts only the exact checked-in
processed-library profile ID, profile SHA-256, source URI, byte count, absent
publisher-digest state, and acquisition-only state. A caller cannot substitute
a self-authored profile while continuing to claim the frozen distribution.

## Consequences

- Interrupted transfers are recoverable without weakening range or hash checks.
- A changed Box object, NNDC artifact, redirect destination, or profile fails
  before table inspection or transport preparation.
- The 9.66 GB transfer is an explicit operator action rather than a build or CI
  side effect.
- Acquisition evidence establishes provenance and byte identity, not evaluated
  nuclear-data correctness, response-table qualification, or clinical fitness.
- The current NNDC neutron archive is an unqualified source candidate until its
  selected evaluations and generated responses are compared with the processed
  OpenMC distribution; public evidence does not establish that the archive
  change was packaging-only.
- The processed archive remains an unqualified candidate until it is actually
  acquired, inspected, reviewed, and bound to the case response evidence.

## Primary sources

- [OpenMC official data libraries](https://openmc.org/data/)
- [OpenMC data-generation PR 96](https://github.com/openmc-dev/data/pull/96)
- [Pinned OpenMC ENDF generation script](https://github.com/openmc-dev/data/blob/66cfe45ff7a3aa47a4d7805b92b3d5ab6ee018b6/generate_endf.py)
- [NNDC ENDF/B-VIII.1 neutron release](https://www.nndc.bnl.gov/endf-releases/?sublibrary=neutrons&version=B-VIII.1)
- [NNDC ENDF/B-VIII.1 errata](https://www.nndc.bnl.gov/endf-library/B-VIII.1/errata/)
