# NF-BNCT-001: Macroscopic Component-Dose Truth Case

**Specification version:** 0.1.0

**Status:** Geometry, resolved material, and source frozen; response tables and
reference results are unqualified pending implementation and independent review

**Qualification ceiling:** Synthetic research only

## Purpose

`NF-BNCT-001` is the first end-to-end NCTForge conformance case. It tests one
traceable path from synthetic DICOM geometry to a transport-neutral material and
source model, component-resolved macroscopic absorbed dose, uncertainty, and an
evidence bundle.

It is designed to reveal:

- DICOM row/column, slice-order, unit, and left/posterior orientation errors;
- material and isotope normalization errors;
- source-position, direction, and per-source normalization errors;
- double counting of capture photons;
- incomplete classification of non-photon neutron KERMA;
- response-table interpolation errors;
- invalid uncertainty propagation; and
- backend-specific meaning hidden behind a shared component label.

It is not intended to model a clinical beam or patient.

## Frozen identifiers

The generator derives UIDs from UUIDv5 URL names and encodes them using the
DICOM `2.25.<UUID integer>` form.

| Object | UID |
| --- | --- |
| Frame of Reference | `2.25.240883953911088373736134884257182446642` |
| Study | `2.25.149214599444245138262873740736845471752` |
| CT Series | `2.25.337319594251465962942344971245692083782` |
| RTSTRUCT Series | `2.25.50705181539640583496141175374452175263` |
| RTSTRUCT Instance | `2.25.277528316852233615277963392913905893031` |

CT SOP Instance UIDs are generated from
`https://nctforge.org/benchmarks/nf-bnct-001/ct-slice-NNN`, where `NNN` is the
zero-padded slice index.

The first and last derived CT SOP Instance UIDs are respectively
`2.25.43546999367060429143037900891741988095` and
`2.25.224181827055039319855832006853907618875`. Part 10 files use the frozen
implementation-class UID `2.25.265222385035053258666337852178839144876`,
implementation version `NCTFORGE_0_1`, and synthetic content date/time
`20260101 / 000000` where those attributes belong to the IOD. These values make
independently generated artifacts byte-comparable; they do not assert clinical
acquisition history.

## Canonical coordinate system

- Patient-based right-handed LPS coordinates in millimetres.
- Positive x is patient-left, positive y is posterior, and positive z is toward
  the head.
- NCTForge array order is `[column, row, slice]`.
- OpenMC uses the same axis directions in centimetres.

For voxel index `(i, j, k)`, its centre is:

```text
P_mm(i,j,k) = [-97.5 + 5i, -97.5 + 5j, -97.5 + 5k]
```

## Synthetic CT

| Property | Value |
| --- | --- |
| SOP Class | CT Image Storage |
| Transfer syntax | Explicit VR Little Endian |
| Columns, rows, slices | `40, 40, 40` |
| Pixel spacing | `5.0\\5.0 mm` (row, column) |
| Slice centre spacing | `5.0 mm` |
| Slice thickness | `5.0 mm` |
| Image Orientation (Patient) | `1\\0\\0\\0\\1\\0` |
| Image Position (Patient), slice `k` | `-97.5\\-97.5\\(-97.5 + 5k) mm` |
| Image Laterality | `U` (unpaired synthetic phantom) |
| Rescale slope/intercept | `1 / 0` |
| Pixel representation | signed 16-bit |
| Stored pixel value | `0` everywhere |

The resulting voxel boundaries are exactly `[-100, 100] mm` on all three axes.
CT number does not infer material in this case. The benchmark manifest assigns
the frozen synthetic material below, preventing an HU calibration curve from
becoming an uncontrolled input.

The generator sets all patient identity fields to visibly synthetic values and
contains no source patient data.

The source implementation is in `crates/nctforge-dicom/src/synthetic.rs`. The
acceptance oracle is maintained separately in
`crates/nctforge-dicom/src/benchmark.rs` so generation and verification do not
share ROI mask calculations.

Generation also writes `case.json` using schema identifier
`nctforge.case-manifest/0.1.0`. It records the frozen coordinate system and
geometry, DICOM UIDs, ROI truth values, material/source model identifiers, and
SHA-256 for all 40 CT instances plus the RT Structure Set. The verifier rejects
missing, modified, duplicated, path-escaping, or unexpected DICOM artifacts.
The R1 desktop viewer consumes this same verified case boundary; it does not
provide a bypass for opening individual or unverified DICOM files.

The generated files are checked with dicom3tools snapshot `20240118131615`:
`dciodvfy -new` must report neither errors nor warnings for any of the 40 CT
instances or the RT Structure Set, and `dcentvfy` must accept the 41-instance
collection without entity-consistency findings. See
`scripts/validate-dicom-iod.sh`. This external mechanical check is intentionally
in addition to NCTForge's semantic geometry oracle and is not described as
DICOM certification.

## RTSTRUCT

Contours are `CLOSED_PLANAR`, lie on CT slice centres, use the frozen Frame of
Reference, and are aligned to voxel boundaries. Rasterization uses a voxel-centre
inclusion rule with half-open upper bounds.

| ROI | Bounds in LPS mm | Expected mask voxels | Expected volume |
| --- | --- | ---: | ---: |
| `PHANTOM` | `[-100,100)` on x, y, z | 64,000 | 8,000 cm3 |
| `CORE` | `[-20,20)` on x, y, z | 512 | 64 cm3 |
| `LEFT_ANTERIOR_MARKER` | x `[60,80)`, y `[-80,-60)`, z `[-80,-60)` | 64 | 8 cm3 |
| `RIGHT_POSTERIOR_MARKER` | x `[-80,-60)`, y `[60,80)`, z `[60,80)` | 64 | 8 cm3 |
| `CENTRAL_AXIS_2CM` | x `[-5,5)`, y `[-5,5)`, z `[-85,-75)` | 8 | 1 cm3 |

The asymmetric markers make left/right, anterior/posterior, and slice reversal
observable. `CENTRAL_AXIS_2CM` is centred 20 mm inside the incident face; it is a
reporting ROI, not an assertion that the thermal-fluence maximum occurs there.

## Transport geometry and boundary

- Homogeneous cube with boundaries `[-10, 10] cm` in x, y, and z.
- Vacuum boundary on all six faces.
- One material fills the cube.
- The scoring mesh matches the CT exactly: lower-left `[-10,-10,-10] cm`,
  upper-right `[10,10,10] cm`, dimensions `[40,40,40]`.
- No variance reduction is allowed in the first qualified result.

## Material

The base is the NIST ICRU four-component soft-tissue composition at
`1.00000 g/cm3`. Exactly `40 microgram of B-10 per gram of final material` is
introduced by scaling the four base elemental mass fractions by `0.99996` and
assigning the remaining mass fraction to B-10.

| Constituent | Final mass fraction |
| --- | ---: |
| H, natural | `0.10116795312` |
| C, natural | `0.11099556000` |
| N, natural | `0.02599896000` |
| O, natural | `0.76179752688` |
| B-10 | `0.00004000000` |

The fractions sum to one. Natural isotopic abundances are used for H, C, N, and
O; no B-11 is present. Temperature is `293.6 K`. The baseline deliberately uses
the free-gas treatment for hydrogen. A bound-hydrogen variant receives a new
case suffix and reference result.

The transport contract does not ask a backend to interpret “natural.” It
expands the elemental fractions once using the IUPAC 2013 representative atom
fractions and AME2020 masses distributed with OpenMC 0.16.0:

```text
w_i = w_E * (a_i * m_i) / sum_j(a_j * m_j)
```

| Nuclide | Frozen transport mass fraction |
| --- | ---: |
| H-1 | `0.10113647042677168` |
| H-2 | `0.00003148269322832` |
| C-12 | `0.10966437305411902` |
| C-13 | `0.00133118694588098` |
| N-14 | `0.02589697162573985` |
| N-15 | `0.00010198837426015` |
| O-16 | `0.75977638114772760` |
| O-17 | `0.00030676400854396` |
| O-18 | `0.00171438172372844` |
| B-10 | `0.00004000000000000` |

These values again sum to one within the declared `1e-12` tolerance. The
derivation is fixed by ADR 0006 to OpenMC tag commit
`617d35a5063c57796b43428bc401e627d2011046`; its `mass_1.mas20.txt` input has
SHA-256 `e8599c6d7f724fac91934e59f1b9de8fb8f63e820f4b39456b790665ed2a3307`.
The machine input is [`transport/material.json`](transport/material.json).
Every named isotope is mandatory: a backend must reject missing data rather
than substitute a natural-element evaluation or silently merge an isotope.

## Source

- Fixed source; exactly one unit-weight source neutron per source history.
- Uniform position over x `[-5,5) cm`, y `[-5,5) cm` at z `-9.999999 cm`.
- Monodirectional unit vector `(0, 0, +1)`.
- Monoenergetic `1.000 keV` neutron.
- No source photons.
- Sites outside the source square have zero probability.

The source plane is just inside the incident phantom face so every source site
is inside the OpenMC geometry. The offset is an implementation guard, not a
physical air gap. The source is intentionally simple and is not a model of a
beam-shaping assembly. Secondary photons originate only from interactions
following the source neutron.

The machine input is [`transport/source.json`](transport/source.json). Its
contract distinguishes source histories, sites per history, and statistical
weight so the `Gy/source neutron` normalization cannot be inferred from a
backend-specific particle counter.

## Required component output

The output profile is `nctforge.macroscopic-absorbed-dose.v1` as defined in
ADR 0002. Each voxel contains the mean and, when statistically defined, the
one-sigma absolute standard uncertainty in `Gy/source neutron` for:

- `boron` (`D_B`);
- `nitrogen` (`D_N`);
- `hydrogen` (`D_H`); and
- `photon` (`D_gamma`).

The output also contains a dedicated physical total. The total is not used to
hide a missing component and is not biologically weighted.

## Required tallies and diagnostics

1. Component response tallies on the 40-cubed mesh.
2. Dedicated total neutron heating with a neutron particle filter.
3. Dedicated photon heating with a photon particle filter.
4. Dedicated coupled total heating without a particle filter.
5. B-10 MT=107 and N-14 MT=103 reaction rates.
6. Neutron fluence over a versioned diagnostic energy grid containing explicit
   boundaries at `0.5 eV`, `1 keV`, and `10 keV`.
7. Photon fluence over a versioned diagnostic grid containing explicit
   boundaries around `478 keV` and `2.224 MeV`.
8. Surface leakage by particle type.
9. Mesh-cell material mass used in every energy-to-dose conversion.

Before running, the adapter fails capability preflight unless the selected data
contain B-10 MT=107, N-14 MT=103, neutron-heating responses, and the secondary
photon-production data required by the component definition. Missing data never
produce an accepted zero-valued component.

Response-table files, grids, interpolation policy, reaction classification,
and unit conversions are hashed benchmark inputs. They are not embedded as
unreviewed constants in GUI code.

The frozen semantic ledger is
[`transport/component-profile.json`](transport/component-profile.json), with
SHA-256 `a35b26c0134ae02d3b1b0ede5b8c6f38e86966e86c65e1727b4c7f38677ab41a`.
The pending table-generation method is
[`transport/response-generation-method.json`](transport/response-generation-method.json),
with SHA-256
`8b46fb1d624b986a1031c45d8869591a73c79e58cd3a354b33749fafd31d5519`.
ADR 0007 defines its unit path and acceptance evidence. The method artifact is
not a response table and does not raise the benchmark qualification ceiling.

## Execution profiles

### Smoke

The smoke profile only establishes that the full pipeline executes and all
artifacts validate. It has no scientific acceptance threshold and cannot create
reference results.

Its machine input is
[`transport/openmc-smoke-profile.json`](transport/openmc-smoke-profile.json),
with SHA-256
`73c644e483e9b9008a88be93d0f47ede174e5180f4c137c208fd7cc62be23e07`.
It freezes five active batches, seed `20260831`, stride `152917`, coupled photon
transport, atomic relaxation, local electron energy deposition, probability
tables, nearest-temperature selection within `0.5 K`, history-based transport,
and the diagnostic energy boundaries required above. The case's requested
history count must divide exactly into those batches.

### Candidate reference

- At least 50 statistically active batches.
- At least three independent seeds.
- Particle count is increased until the precision gates below are met.
- Each seed produces a separate immutable evidence bundle before aggregation.
- Results are compared statistically and never required to be bit-for-bit
  identical across hardware or parallel layouts.

The three initial seeds are frozen as decimal `20260831`, `314159265`, and
`271828182`.

## Predeclared acceptance gates

### Geometry

- Imported shape, spacing, origin, direction, and Frame of Reference equal the
  values above.
- Every expected ROI mask count and volume is exact.
- Marker centroids lie in the named LPS quadrants.
- Reversing CT file order does not change geometry or masks.
- A mismatched Frame of Reference, duplicate slice position, non-orthonormal
  direction, or nonuniform projected spacing is rejected.

### Units and invariants

- Material mass fractions sum to one within `1e-12`.
- Component values and absolute uncertainties are finite and non-negative.
- Relative uncertainty is absent rather than infinite/NaN when the mean is zero.
- Energy-to-dose conversion uses the scored voxel mass and
  `1 eV = 1.602176634e-19 J`.
- No photon energy is counted in both a neutron component and `D_gamma`.
- The dedicated physical total agrees with the component sum within the
  estimator-comparison tolerance declared below.

### Monte Carlo precision

- For `CORE` and `CENTRAL_AXIS_2CM`, one-sigma relative sampling uncertainty is
  at most 1% for every nonzero component.
- For voxels at or above 20% of a component's maximum, the median relative
  sampling uncertainty is at most 3% and the 95th percentile is at most 5%.
- Independent-seed ROI means have a reduced chi-square compatible with the
  stated sampling uncertainties; failures trigger investigation rather than
  selective seed removal.

### Estimator comparison

For `CORE`, `CENTRAL_AXIS_2CM`, and the central-axis depth profile above 20% of
each maximum:

- B-10 and N-14 reaction-rate audits agree with their response estimators within
  2%;
- the summed neutron components agree with dedicated neutron heating within 3%;
  and
- the physical component sum agrees with dedicated coupled total heating within
  3%.

Because these estimators use the same histories, their standard uncertainties
are correlated and are not combined as if independent. If paired batch
differences are retained, their directly estimated uncertainty is reported as
an additional diagnostic; it does not replace the percentage gate.

These are conformance thresholds for this synthetic case, not clinical
commissioning tolerances.

### Cross-code comparison

No candidate output becomes a reference output until a separately implemented
transport path reproduces the source, geometry, material, nuclear-data intent,
and scoring definitions.

- Neutron fluence, B-10 reaction rate, and N-14 reaction rate ROI means: within
  2% or three combined standard uncertainties.
- Component ROI means: within 3% for `D_B`, `D_N`, and `D_H`, and within 5% for
  `D_gamma`, or three combined standard uncertainties when wider.
- Any failed spatial bin is reported; aggregate agreement cannot erase a
  localized discrepancy.

The initial openly redistributable implementation target is Geant4, with its
nuclear-data differences declared. A same-underlying-data comparison with MCNP
is the stronger way to separate code behavior from evaluated-data differences
and is desired when a licensed collaborator is available. MCNP and PHITS result
imports must be produced by licensed users and those codes cannot be bundled
with NCTForge.

## Evidence bundle

Each run contains, at minimum:

```text
case.json
source.json
materials.json
component-profile.json
response-tables/
dicom-manifest.json
engine-manifest.json
nuclear-data-manifest.json
run-settings.json
inputs/
logs/
statepoints-or-native-results/
normalized-dose.json-or-hdf5
qa-report.json
artifact-manifest.json
```

The artifact manifest binds every file by SHA-256 and records the qualification
boundary. Paths are relative and cannot escape the evidence-bundle root.

## Deliberately excluded

- CBE, RBE, Gy-Eq, or isoeffective dose;
- time-varying or optimized boron distributions;
- patient data and HU-to-tissue calibration;
- clinical beam spectra or monitor units;
- contour editing;
- charged-particle microdosimetry;
- skin/interface claims;
- clinical acceptance or commissioning claims; and
- any Avify Dose patent-sensitive recomposition or certificate workflow.

## Coverage limits and follow-on cases

This first case tests axis signs, index order, slice sorting, and uniform-grid
transforms, but it does not qualify general DICOM support. A separate geometry
case is required before R1 completion for oblique image orientation,
non-square pixels, nonuniform or missing slices, nested/XOR contours, and
alternate transfer syntaxes. Enhanced multi-frame CT and DICOM SEG remain later
profiles.

## Remaining freeze gate

Geometry, material, source, classification, response-generation method, and
OpenMC smoke execution profile are frozen by this version. Deterministic input
generation is implemented, but the benchmark cannot enter execution until the
official evaluated-data selection and generated KERMA tables have been hashed,
checked against the ADR 0007 gates, and independently reviewed. Changing any
frozen quantity creates a new benchmark specification version and cannot
silently replace earlier results.
