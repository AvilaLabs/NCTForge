# ADR 0022: LAW=7 Processor-Approximation Attribution

**Status:** Accepted and implemented; all H-2 findings attributed, transition completed by ADR 0023

**Date:** 2026-08-31

## Context

ADR 0021 establishes independently that the exact JEFF-4.0 H-2
MF=6/MT=16 LAW=7 source normalizes and leaves positive mean energy for its
implicit proton at every active source node. That result is necessary but not
sufficient to dismiss NJOY's 15 MT 301 high-limit findings. A defensible
reclassification also has to prove what the pinned processor did at every
warning energy using the exact output bound by the execution receipt.

The NJOY2016.78 HEATR source records a missing File 6 residual in `mt6no`,
generates a one-particle recoil, computes `ebal6` as the remaining File 6
energy balance, and accumulates `h + ebal6` into total heating. For File 6
products, however, its kinematic-limit branch uses `h` for both bounds. The
added `ebal6` term is therefore present in MT 301 but absent from the bound.
The implementation itself suggests a cause; only a receipt-bound numerical
identity can establish that it explains this run.

## Decision

NCTForge adds the immutable
`nctforge.njoy-law7-implicit-residual-comparison/0.1.0` report. Assessment and
verification require the exact independent source report, execution receipt,
and complete execution directory. The comparison verifies the whole execution
root before reading the H-2 processor report and binds that report by path,
size, and SHA-256.

The parser requires:

- the missing-residual warning and one-particle generation notice in both the
  production and diagnostic HEATR passes;
- the diagnostic MT=16 neutron table for particle 1;
- the synthesized-recoil table for particle 1002, including every printed
  `ebal` row;
- the final MT 301/443 KERMA table and each `high` marker; and
- an exact one-for-one match between those markers and the receipt's
  structured violations.

At every shared source/processor node, the report reconstructs

`ebal = (E + printed Q) * cross section`
`       - mean neutron energy * neutron yield * cross section`
`       - synthesized recoil heating`.

At every warning node it additionally requires:

1. the synthesized recoil to be negative and `ebal` positive;
2. `MT443` to equal the printed kinematic maximum;
3. `MT301 - maximum` to equal `ebal` within print precision; and
4. `recoil heating + ebal` to agree with the independent implicit-proton local
   KERMA within the source/processor quadrature tolerance.

The declared tolerances are `2e-3` for independent-source versus NJOY
quadrature and `2e-4` for identities reconstructed from the five-significant-
digit printout. The schema caps them at `5e-3` and `1e-3`, respectively. The
qualification remains `unreviewed`; the comparison does not endorse the
one-particle recoil spectrum or generalize the result to another evaluation,
processor commit, reaction, or nuclide.

## Controlled result

The exact receipt-bound comparison passes:

| Measure | Result |
| --- | ---: |
| Independent active source nodes | 53 |
| NJOY MT=16 print nodes | 23 |
| Exact shared nodes | 22 |
| Receipt/final-table violations | 15 / 15 |
| Fully attributed violations | 15 |
| Failed comparison nodes | 0 |
| Maximum source/NJOY neutron-mean difference | `1.8639429185867214e-3` |
| Maximum reconstructed-`ebal` difference | `8.722171393734166e-5` |
| Maximum warning `ebal`/MT301-excess difference | `1.0545742156604271e-4` |
| Maximum warning independent/processor local-KERMA difference | `1.140146485932929e-3` |

At 9 MeV, NJOY prints a `379260 eV*b` energy-balance remainder. Its final
MT 301 value exceeds the kinematic maximum by `379300 eV*b`; their relative
difference is `1.0546e-4`. Adding the printed negative recoil heating to the
remainder gives `318553 eV*b`, versus the independent source result of
`318189.8029164666 eV*b`.

The frozen comparison SHA-256 is
`64b3985ed5fc3d57c7a41c55b58e13f8bba069403c72bafe50235a13e0ae5687`.

## Consequences

- All 15 H-2 findings, including the 12 inside the 20 MeV transport domain,
  are numerically explained by the same missing-residual approximation and
  excluded energy-balance remainder.
- No warning is deleted, and NJOY's negative synthesized recoil is not treated
  as physical proton-spectrum evidence. Both remain visible in the immutable
  comparison.
- The v0.3 suitability report remains immutable and rejected. ADR 0023's
  separately versioned, approximation-aware v0.4 report binds this comparison
  and the content-derived transport domain and reclassifies only H-2.
- The evidence provides no waiver for C-13, N-15, O-17, or O-18.

## Related decisions and primary sources

- [ADR 0012: Controlled NJOY execution evidence](0012-controlled-njoy-execution-evidence.md)
- [ADR 0020: Content-bound transport-domain suitability](0020-content-bound-transport-domain-suitability.md)
- [ADR 0021: Independent LAW=7 implicit-residual balance](0021-independent-law7-implicit-residual-balance.md)
- [ADR 0023: Reaction-evidence-aware transported-photon suitability](0023-reaction-evidence-aware-suitability.md)
- [NJOY2016.78 HEATR implementation](https://github.com/njoy/NJOY2016/blob/71a76bc6345fa15f36bacc816ae7900714345d97/src/heatr.f90)
- [NJOY2016 HEATR manual](https://github.com/njoy/NJOY2016-manual/blob/master/heatx.tex)
