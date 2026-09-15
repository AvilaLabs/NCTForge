# TENDL-2025 Mixed-Source Candidate Findings for `NF-BNCT-001`

**Recorded:** 2026-09-11

**Evidence state:** Controlled candidate rejected at the domain-aware layer;
not a qualified response table

## Question

Does a selection that keeps the six clean ENDF/B-VIII.1 nuclides and
substitutes TENDL-2025 evaluations for the four baseline failures resolve the
transported-photon KERMA blockers when every other frozen processing input
remains unchanged?

## Bound inputs

- TENDL-2025 archive: 3,517,450,425-byte `TENDL-n.tgz` from the official
  Imperial College mirror;
- publisher digest: **none distributed** — the acquisition is bound by exact
  byte count, transfer metadata, locally computed SHA-256, and receipt rather
  than publisher attestation;
- locally computed archive SHA-256:
  `e547527688506cbe09813364dcefa2aed11f474139bfa129d7cd4ca24fae21fa`;
- ENDF/B-VIII.1 archive: 343,724,780-byte
  `ENDF-B-VIII.1-neutrons.zip`, publisher digest matched, archive SHA-256
  `decff90016bfb5c25c8d4c7fb8d81f94ff2f104853165f0fe6a21bf61c4164e4`;
- acquisition profile SHA-256s: ENDF/B-VIII.1
  `8966a40d91064826080554c81be84ab5ac490aa190b5a830afaa650c8e0ab0c4`,
  TENDL-2025
  `b6927549d0a95056e411d59725fda4a1dd198b993216ec11431785fbeb488bd1`;
- source-selection SHA-256:
  `52e6358f3a8e2addb574b1ae1695dc4fa3bed011e41389689d83503c3e6b4f2c`;
- input-manifest SHA-256 recorded in `njoy/openbnct-njoy-input-manifest.json`
  (schema `0.2.0`);
- processor: NJOY2016.78 commit
  `71a76bc6345fa15f36bacc816ae7900714345d97`, executable SHA-256
  `8a37cf70cf801b0c30ba70735f53a7b6aa51f18e53a10071fa0aff3341174c2d`.

The material, temperature, reconstruction tolerance, no-Q-override policy,
normal/check HEATR passes, and requested partial reactions are identical in
meaning to the baseline. The evaluated-data source for the four blocked
nuclides is the intended independent variable.

TENDL-2025 distributes no hydrogen evaluations, so a TENDL-only selection
cannot satisfy the ten-nuclide material. The selection is mixed under schema
`0.3.0`: ENDF/B-VIII.1 supplies B-10, C-12, C-13, H-1, H-2, and N-14;
TENDL-2025 supplies N-15, O-16, O-17, and O-18. Every evaluation carries a
content-addressed `acquisition_sha256` binding.

## Result

| Nuclide | Source | Baseline violations | Candidate violations | Outcome |
| --- | --- | --- | --- | --- |
| B-10 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| C-12 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| C-13 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| H-1 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| H-2 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| N-14 | ENDF/B-VIII.1 | 0 | 0 | remained candidate |
| N-15 | TENDL-2025 | 10 | 6 | remained rejected |
| O-16 | TENDL-2025 | 15 | 16 | remained rejected |
| O-17 | TENDL-2025 | 20 | 45 | remained rejected |
| O-18 | TENDL-2025 | 27 | 10 | remained rejected |

The mixed selection reproduces the ENDF/B-VIII.1 baseline exactly on the six
nuclides both selections share — the first direct evidence that per-nuclide
substitution behaves as the schema intends. All four TENDL-2025 substitutions
still produce high-direction MT 301 kinematic diagnostics: 77 violations
across four rejected runs, against the baseline's 72. No baseline rejection
was resolved and none was introduced.

## Suitability chain

- **v0.1 log-only:** 77 kinematic violations, 4 rejected runs, 0 processor
  data findings (`transported_photon_kerma_rejected`).
- **Photon-production inventory:** 315 MF=6/12/13/14/15 sections across all
  ten evaluations; every evaluation, including all four TENDL files, carries
  a HEATR photon source; 0 format findings. The failures are not explained by
  missing photon-production records.
- **v0.2 source-aware:** unchanged; 4 runs remain rejected.
- **v0.3 domain-aware:** 77 full-domain violations partition into 70
  in-domain and 7 out-of-domain; no run reclassifies; 4 remain rejected.

## Deeper gates

- **O-17 processor attribution:** all 43 in-domain O-17 findings reproduce
  NJOY's printed per-reaction File 6 energy-balance remainders
  (maximum printed-remainder/final-excess relative difference
  3.72e-4). As with JEFF-4.0, this explains the processor's accounting and
  still requires independent physical validation; nothing is waived.
- **N-15 capture photon balance: not computable.** The independent gate
  rejects TENDL N-15's MF=3/MT=102 tabulation as invalid ENDF: it contains
  two duplicated energy grid points (930.101 keV and 30 MeV). The same
  defect appears systematically across the TENDL oxygen evaluations — O-16,
  O-17, and O-18 each carry duplicated grid points in nearly every File 3
  section. NJOY2016.78 tolerates these duplicate boundary points and
  completes, but OpenBNCT's strict source parsers correctly refuse them, so
  no independent capture-balance or File 13/15 continuum-moment evidence can
  be generated for any TENDL-selected nuclide. This is a source-format
  finding against the library itself, distinct from the kinematic failures.
- **H-2 LAW=7 implicit residual: not applicable.** ENDF/B-VIII.1's H-2
  MF=6/MT=16 records two explicit products (NK=2); there is no implicit
  residual to check. The gate was designed for JEFF-style single-product
  representations.
- **File 13/15 continuum photon moments: not computable.** No supported
  single-component continuum reactions exist in this selection.
- **v0.4 evidence-aware and diagnostic triage: not reachable.** Both layers
  require the capture-balance and comparison reports that the TENDL source
  defect makes uncomputable. The candidate is therefore rejected at v0.3
  with two independent reasons: 70 in-domain kinematic violations, and a
  source-format defect that prevents the deeper independent checks from
  existing at all.

## Baseline comparison

The verified comparison binds the rejected ENDF/B-VIII.1 baseline v0.1 report
to the candidate v0.1 report:

- baseline rejected runs: 4; candidate rejected runs: 4;
- resolved baseline rejections: 0; introduced rejections: 0;
- kinematic violations: baseline 72, candidate 77;
- qualification: `candidate_rejected`.

## Interpretation

TENDL-2025 does not resolve the external-evidence gate. The four nuclides it
was selected for remain rejected — O-17 materially worse (45 vs 20
violations) — and the library's duplicated-grid tabulation style is
incompatible with the strict source-level independent gates the evidence
chain depends on. Two observations bound the failure:

1. The kinematic failures are source-physics findings, not missing-record
   artifacts: all photon-production machinery is present and inventoried.
2. The uncomputability of the deeper gates is itself rejecting evidence
   about TENDL-2025's suitability under this chain; it is not a gap to be
   patched by relaxing the parsers.

Per ADR 0026, response qualification remains paused. The remaining resume
condition is a reviewed independent O-17 reaction calculation; no further
published evaluated library is currently a credible mechanical candidate.

## Preserved artifacts

All evidence lives under
`benchmarks/synthetic/nf-bnct-001/transport/candidates/endfb81-tendl2025/`:
the `0.3.0` selection, the `0.2.0` input manifest, the copied acquisition
receipts, the v0.1/v0.2/v0.3 suitability reports, the photon inventory, the
O-17 attribution, the verified baseline comparison, and its machine check.
The 3.28 GiB archive, extracted evaluations, processor binary, and output
tapes are retained outside the repository per the storage policy.
