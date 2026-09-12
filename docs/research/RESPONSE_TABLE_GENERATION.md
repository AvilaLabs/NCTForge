# Response-table generation and in-house deterministic verification

**Recorded:** 2026-09-12

## Question

With the O-17 diagnostic queue dispositioned as explained (ADR 0031), can the
platform produce the first `NeutronResponseSet` under the frozen
`response-generation-method` contract directly from receipt-bound production
HEATR PENDF output, and can that set be qualified for folding without an
external reviewer?

## What was built

`nctforge-njoy::response_tables` (schemas
`nctforge.njoy-response-table-generation/0.1.0` and
`nctforge.njoy-response-set-review/0.1.0`) with two CLI boundaries:

- `njoy generate-response-tables`; and
- `njoy verify-response-tables`.

Generation loads the evaluated-source selection, material, component profile,
generation method, nuclear-data manifest, transport domain, domain-aware
suitability report, and execution receipt; validates every content binding;
then for each material nuclide verifies the receipt-bound production HEATR
PENDF tape (`<nuclide>/tape23`) by SHA-256 and parses its MF=3 pseudo-reaction
sections:

- MT=301 total KERMA;
- MT=443 kinematic total (the total-KERMA ceiling used for checks);
- MT=407 B-10 partial KERMA (reaction MT=107) on the B-10 run; and
- MT=403 N-14 partial KERMA (reaction MT=103) on the N-14 run,

exactly as selected by the frozen HEATR method. All tables are evaluated onto
the union of their knots extended to the declared transport energy range
(1.0e-5 eV to 2.0e7 eV); ENDF repeated-x discontinuities take the right-hand
value, supported interpolation laws are honoured, and values outside a table's
tabulated domain hold the nearest boundary value with the held knots counted.
KERMA factors in eV·barn are converted to Gy·cm² per source particle using the
method's joule-per-eV constant, `1 barn = 1e-24 cm²`, and atom densities
derived from material mass fractions and atomic-weight ratios. The hydrogen
component is the classified residual `MT301 - B-10 - N-14`; a negative
residual at any knot rejects generation, and the emitted set requires exact
`boron + nitrogen + hydrogen == total_neutron` closure at every knot.

Verification independently regenerates both the response set and the
generation report from the same inputs, requires byte-exact equality,
revalidates all invariants, and emits a review report plus a reviewed copy of
the set bound to that report and qualified `independently_reviewed` — the
in-house deterministic-verification path opened by ADR 0031. OpenMC input
generation continues to require `validate_for_folding`, which only the
reviewed set satisfies.

## Baseline result

Executed against `njoy-execution-v5` (ENDF/B-VIII.1, NJOY2016.78):

- 7,526 union-grid knots over the full transport range; 1,636 boundary-held
  knots (tables that do not span the full range hold their edge values);
- residual minimum 7.7e-16 Gy·cm², remaining non-negative at every knot;
- all 72 in-domain kinematic findings carried into the generation report as
  `carried_in_provenance_findings_retained`, including the caveat that
  nuclides without transported photon-production data deposit photon energy
  locally — a documented data-coverage property of the evaluation, not a
  processor artifact;
- `verify-response-tables` regenerates both artifacts byte-exactly.

Frozen artifacts, all content-addressed under
`benchmarks/synthetic/nf-bnct-001/transport/provenance/`:

- `njoy2016-78-response-table-generation.json`;
- `neutron-response-set.unreviewed.json` (`tables_generated_unreviewed`);
- `njoy2016-78-response-set-review.json`; and
- `neutron-response-set.json` (`independently_reviewed`).

## Conventions implemented

Each convention is checkable against the ENDF-6 manual and the frozen method
document:

- **Channel selection.** Only the method-declared pseudo-MTs are extracted:
  301/443 for every nuclide, 407 only on B-10, 403 only on N-14. Any other
  partial KERMA the processor emits is ignored.
- **Discontinuity rule.** Consecutive duplicate grid energies resolve to the
  right-hand value, matching the ENDF-6 step convention used elsewhere in the
  strict parsers.
- **Boundary rule.** Outside a table's tabulated domain the nearest boundary
  value is held and the knot is counted in `boundary_held_knot_count`; the
  union grid itself is never downsampled.
- **Residual rule.** The hydrogen curve is the material-total remainder after
  subtracting the B-10 and N-14 components; it must stay non-negative, and
  closure is enforced exactly at every knot.

## Status and limits

- Generation-report qualification `tables_generated_unreviewed`; reviewed-set
  qualification `independently_reviewed` with scope
  `in_house_deterministic_verification`. The review is deterministic
  regeneration, not nuclear-data review; the documented findings and the
  trace-nuclide bounds of ADR 0031 remain the standing caveats.
- The reviewed set authorizes folding into OpenMC input generation only. It
  does not qualify reference results: smoke execution, statepoint import, and
  the independent estimator comparison of ADR 0007 remain pending.
- `njoy verify-response-tables` requires byte-for-byte equality of the
  regenerated set and report; any drift in method, material, tapes, or carried
  findings fails verification closed.
