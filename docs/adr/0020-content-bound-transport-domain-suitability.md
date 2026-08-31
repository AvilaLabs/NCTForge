# ADR 0020: Content-Bound Transport-Domain Suitability

**Status:** Accepted and implemented; JEFF-4.0 O-16 reclassified, candidate still rejected

**Date:** 2026-08-31

## Context

The controlled NJOY runs retain kinematic diagnostics over each source
evaluation's complete energy range. That is useful processor evidence, but it
is broader than the first `NF-BNCT-001` transport calculation. Its selected
OpenMC neutron tables do not all share the same upper endpoint: the common
case capability ends at 20 MeV even though individual tables and evaluated
sources can extend higher.

JEFF-4.0 O-16 has one NJOY MT 301 high-limit finding, at 30 MeV. It has no
finding at or below 20 MeV. Calling the 30 MeV point irrelevant without a
separate contract would still be unsafe: a command-line number, the 1 keV
source energy, or a diagnostic plotting grid could silently narrow the audit
without being part of the calculation's identity.

The scope therefore has to be derived from the actual backend capability and
bound to the exact material used by NJOY.

## Decision

NCTForge adds two immutable evidence contracts.

`nctforge.openmc-neutron-transport-domain/0.1.0` derives the intersection of
the selected-temperature incident-neutron intervals for every nuclide in an
exact OpenMC nuclear-data manifest and material. It binds the input bytes by
ID and SHA-256 and records the OpenMC version and source commit. For
`NF-BNCT-001`, the result is
`[9.999999999999999e-6, 20,000,000] eV`.

The diagnostic interval is closed and conservative. A finding at exactly
20 MeV remains in domain; only a finding strictly above 20 MeV can be
classified out of domain. This boundary policy scopes the evidence review. It
does not change OpenMC's separate source-energy endpoint rules.

`nctforge.njoy-transported-photon-suitability/0.3.0` binds:

1. the immutable source-aware v0.2 report;
2. its immutable log-only v0.1 report;
3. the exact execution receipt and its detailed violation energies;
4. the executed NJOY input manifest; and
5. the derived OpenMC transport-domain document.

The input manifest's material reference must exactly equal the transport
domain's material reference. Assessment and verification also require the
exact manifest and material bytes and regenerate the domain before using it;
a merely well-formed document with a self-asserted narrower interval is
rejected. The report partitions every NJOY kinematic finding into in-domain
and out-of-domain counts and retains the complete out-of-domain finding
records. Only the in-domain kinematic count affects the v0.3 suitability
state. Source-format findings and rejecting processor-data findings remain
rejecting regardless of energy.

The v0.1 and v0.2 reports continue to describe their full-evaluation and
source-aware gates. They are not rewritten. A v0.3 pass remains only
`candidate_unreviewed`; it cannot approve a response table or a clinical use.

## Controlled result

The same domain is applied to the baseline and candidate:

| Selection | Full findings | In domain | Out of domain | Reclassified runs | Rejected runs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ENDF/B-VIII.1 baseline | 72 | 72 | 0 | 0 | 4 |
| JEFF-4.0 candidate | 120 | 114 | 6 | 1 | 4 |

The six JEFF-4.0 out-of-domain findings are all MT 301 high-limit findings:

- H-2 at 21, 25, and 30 MeV;
- O-16 at 30 MeV; and
- O-17 at 21 and 25 MeV.

H-2 and O-17 retain 12 and 43 in-domain findings, respectively, so neither
changes status. O-16 has zero in-domain findings and no rejecting source or
processor finding; it alone changes from `rejected` to
`candidate_unreviewed`. C-13 and O-18 remain rejected with all 32 and 27
findings in domain, and both also retain rejecting local-photon-fallback
evidence.

The candidate is therefore still rejected by this gate. Combined with ADR
0019's independent N-15 capture-balance rejection, the unresolved set is now
C-13, H-2, N-15, O-17, and O-18. O-16 is no longer an honest blocker for the
bound 20 MeV calculation, but its 30 MeV source behavior remains preserved in
the full-range receipt and v0.1/v0.2 reports.

Evidence hashes:

- OpenMC transport domain:
  `1554dfb3167c0aa804cd6c893ce22a363cefbc0cba1b8f7781eeae1c2dccf89e`;
- ENDF/B-VIII.1 domain-aware suitability:
  `e270708da7aabf0be6246d8b89fabf031af4ec01c155b015432e2ee174eb9d09`;
- JEFF-4.0 domain-aware suitability:
  `6e46b627d9b766e596ad2219eaafca970bd9f3c5df1d5e400ad644397c44ce55`.

## Consequences

- O-16's apparent remaining JEFF blocker was a scope mismatch, not bad news
  about the planned 20 MeV calculation.
- No diagnostic was deleted, clipped, or changed. The full-range result and
  the scoped decision coexist and are independently reproducible.
- The scope cannot be changed by a loose numeric flag. A different material,
  OpenMC manifest, or serialized input produces a different content identity
  and requires a new assessment.
- The next causal work should address the 114 findings still inside the bound
  interval, beginning with a reaction/source analysis of C-13, H-2, O-17, or
  O-18. Whole-library substitution remains a control, not a substitute for
  those checks.

## Related decisions and primary sources

- [ADR 0008: OpenMC nuclear-data preflight](0008-openmc-nuclear-data-preflight.md)
- [ADR 0013: Transported-photon KERMA suitability](0013-transported-photon-kerma-suitability.md)
- [ADR 0017: Source-aware photon-production suitability](0017-source-aware-photon-production-suitability.md)
- [ADR 0019: Independent MF=6 capture-photon balance](0019-independent-mf6-capture-photon-balance.md)
- [OpenMC 0.16.0 cross-section representation](https://docs.openmc.org/en/v0.16.0/methods/cross_sections.html)
- [OpenMC 0.16.0 release](https://github.com/openmc-dev/openmc/releases/tag/v0.16.0)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
