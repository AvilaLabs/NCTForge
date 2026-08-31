# ENDF/B-VIII.1 Neutron Archive Drift Record

**Observed:** 2026-08-31

**Disposition:** Current publisher object accepted for acquisition as an
unqualified candidate; equivalence to the OpenMC recipe object is unresolved

## Finding

NCTForge's first real evaluated-data acquisition stopped on the publisher
digest check. The stable NNDC URI returned the expected `343,724,780` bytes,
but those bytes did not match the MD5 frozen in OpenMC's merged ENDF/B-VIII.1
generation recipe.

| Identity | MD5 |
| --- | --- |
| OpenMC recipe at commit `66cfe45ff7a3aa47a4d7805b92b3d5ab6ee018b6` | `dc622c0f1c3c4477433e698266e0fc80` |
| NNDC release page and acquired object on 2026-08-31 | `1a6abeac85bd2425df47983752687a93` |

The acquired object's SHA-256 is
`decff90016bfb5c25c8d4c7fb8d81f94ff2f104853165f0fe6a21bf61c4164e4`.
Its gzip stream passes an integrity check. The inspected archive entries—the
release README, changelog, and selected evaluations—carry a 2026-05-06 16:48
timestamp. That is evidence that the current container was created or rewritten
after the 2024 final release; it does not establish why or whether evaluated
records changed.

## What public evidence does and does not establish

The current [NNDC neutron release
page](https://www.nndc.bnl.gov/endf-releases/?sublibrary=neutrons&version=B-VIII.1)
is authoritative for the current archive MD5. The [pinned OpenMC generation
script](https://github.com/openmc-dev/data/blob/66cfe45ff7a3aa47a4d7805b92b3d5ab6ee018b6/generate_endf.py)
is authoritative for the bytes used by that recipe.

NNDC's [ENDF/B-VIII.1 errata
page](https://www.nndc.bnl.gov/endf-library/B-VIII.1/errata/) says that its July
2026 documentation update did not modify ENDF/B-VIII.1 data files. That is
supportive context, but it does not identify this neutron-archive digest change
or prove member-for-member equivalence. The old archive is no longer available
at the stable publisher URI, and no immutable public copy was located, so a
direct comparison is presently impossible.

## Fail-closed decision

- Acquisition profile schema `0.2.0` records the current NNDC digest and the
  prior OpenMC-recipe digest as distinct identities.
- The current archive may supply candidate evaluated inputs, but neither its
  archive nor derived response tables are qualified by acquisition alone.
- The repository stores only content identities and receipts, not the
  343.7 MB publisher archive or extracted evaluations.
- Any future upstream change fails the same size and digest checks and requires
  another reviewed profile revision.

## Qualification gate

Before a response set can leave `method_frozen_tables_pending`:

1. bind every evaluation selected by `NF-BNCT-001` to its archive-relative
   path, byte count, and SHA-256;
2. generate total and partial KERMA from those exact members with the frozen
   NJOY2016.78 method;
3. compare the processed neutron tables and total KERMA against the official
   OpenMC ENDF/B-VIII.1 HDF5 distribution;
4. investigate and disclose any material difference; and
5. obtain the independent review required by ADR 0007.

An upstream confirmation or an authenticated copy of the older archive could
resolve the container drift earlier, but it would not replace the response-level
checks.
