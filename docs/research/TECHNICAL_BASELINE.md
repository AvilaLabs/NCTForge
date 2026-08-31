# BNCT Technical Baseline

**Status:** Adopted research baseline, 2026-08-31

**Scope:** Macroscopic research dosimetry and independent verification. This is
not a clinical-dose specification or a claim that any transport code is a
reference standard.

## Outcome

NCTForge can begin with OpenMC, but its first scientific deliverable must be a
code-neutral benchmark rather than a patient workflow. The initial benchmark is
`NF-BNCT-001`, a synthetic DICOM and transport case with exact geometry,
materials, source, component definitions, provenance requirements, and
predeclared comparison criteria.

The first implementation sequence is therefore:

1. generate and verify the synthetic DICOM geometry;
2. generate a backend-neutral case manifest from it;
3. generate OpenMC input through a version-pinned adapter;
4. retain four unweighted macroscopic absorbed-dose components;
5. compare independent estimators and, subsequently, independent codes;
6. publish no reference dose values until the qualification gates pass.

## Scientific quantity being reported

The first dose model applies only at a spatial scale much larger than the ranges
of the charged reaction products, where charged-particle equilibrium or a
locally deposited KERMA approximation is defensible. Every component is an
absorbed dose in Gy per source neutron before biological weighting:

| Symbol | Interchange name | Required meaning |
| --- | --- | --- |
| `D_B` | `boron` | Local energy imparted by the charged products of the `B-10(n,alpha)Li-7` reaction. Energy carried by emitted photons is excluded. |
| `D_N` | `nitrogen` | Local energy imparted by charged products of neutron reactions assigned to the nitrogen group, principally `N-14(n,p)C-14`. Energy carried by photons is excluded. |
| `D_H` | `hydrogen` | The conventional BNCT hydrogen/fast-neutron group: non-photon neutron KERMA not assigned to `D_B` or `D_N`, dominated by recoil protons from neutron moderation on hydrogen. Contributing nuclides and reactions must remain inspectable. |
| `D_gamma` | `photon` | Energy imparted by photons from every origin, including incident contamination and secondary photons generated in the phantom. |

`D = D_B + D_N + D_H + D_gamma` is a physical absorbed-dose sum. A successful
sum does not imply biological equivalence. CBE, RBE, isoeffective models, and
boron pharmacokinetics remain separate, versioned layers.

The definition of `D_H` is deliberately broader than “H-1 elastic scattering.”
The IAEA describes that component as being produced mainly by hydrogen, and
current BNCT/OpenMC work has warned that other energetically allowed tissue
reactions must be examined to avoid an unreported remainder. A component model
that only retains H-1 elastic dose cannot demonstrate energy-accounting closure.

## Estimator design

The benchmark will retain independent estimators instead of silently selecting
one calculation as truth.

### Reported component estimator

- Neutron track-length fluence is folded with frozen, versioned,
  material-specific fluence-to-KERMA response functions for `D_B`, `D_N`, and
  `D_H`.
- Photon dose uses coupled neutron-photon transport and photon heating. In the
  released OpenMC baseline this is a collision estimator.
- Track-length fluence is divided by voxel volume before applying a response in
  Gy cm2 per particle, or an algebraically equivalent response is used. The
  exact unit path must be written to the run manifest.

The `D_H` response is generated as the classified non-photon neutron KERMA of
the complete benchmark material, excluding contributions assigned to `D_B` and
`D_N`. It must not be approximated as H-1 elastic scattering without a separate
error study.

### Audit estimators

- `B-10` MT=107 and `N-14` MT=103 reaction rates, multiplied by documented
  charged-particle energy releases, audit `D_B` and `D_N`.
- A neutron-only `heating` tally audits the sum of locally deposited neutron
  energy. With coupled transport, OpenMC's MT=301 heating data exclude energy
  carried away by secondary photons.
- A dedicated photon `heating` tally audits `D_gamma`.
- Energy-binned neutron and photon fluence is retained for diagnosing response
  interpolation, thermalization, and library differences.

Agreement between two estimators built from the same histories is an internal
consistency check, not independent validation.

Before execution, the adapter also verifies that the loaded evaluations contain
the required MT=107, MT=103, and heating responses and the secondary-photon data
needed to represent at least H-1 capture and the B-10 prompt-photon branch. A
missing datum is a failed capability preflight, not permission to report zero.

## OpenMC feasibility and limits

The current OpenMC tally system provides the necessary starting primitives:

- fixed-source coupled neutron-photon transport;
- regular mesh, energy, particle, nuclide, and energy-function filters;
- reaction-rate scores including `(n,p)` and `(n,a)`;
- total nuclear `heating` in eV per source particle;
- track-length neutron heating when the tally is explicitly neutron-only;
- direct photon energy-deposition scoring; and
- batch means and standard deviations in statepoint files.

OpenMC 0.16.0 also adds `ReactionFilter`, but it does not expose
reaction-specific neutron KERMA. In the 0.16.0 scoring implementation a
reaction filter forces a collision estimator while neutron heating continues to
use a total heating response; the source explicitly notes that there is no
reaction-wise heating cross section. Reaction-filtered heating is therefore a
diagnostic event partition, not the reported B-10, N-14, or residual-neutron
component estimator. See ADR 0005.

Important limits remain:

- standard `heating` data are not reaction-specific;
- photon heating uses collision scoring in the released baseline and may
  converge more slowly than neutron track-length responses;
- charged products such as alpha particles, Li-7 nuclei, and recoil protons are
  treated through local-energy or KERMA assumptions at this stage;
- OpenMC's charged-particle treatment does not establish microscopic,
  cell-layer, skin-interface, or electron-build-up accuracy;
- a successful OpenMC run is not evidence of correct DICOM geometry, response
  construction, source normalization, or component classification.

For these reasons NCTForge must advertise a macroscopic research capability
only. Microdosimetry is a different solver and validation problem.

## Nuclear-data baseline

The first candidate run is pinned to:

- OpenMC `0.16.0`;
- the official OpenMC ENDF/B-VIII.1 HDF5 incident-neutron, photoatomic, atomic
  relaxation, and thermal-scattering distribution;
- material temperature `293.6 K`;
- no thermal-scattering-law table in the baseline case, so that molecular model
  differences do not obscure the first cross-code comparison.

The official processed distribution has now been acquired and its case-scoped
selection frozen. The ten neutron and five photon tables pass the transport
capability preflight. A pointwise, no-interpolation comparison of every MT 301
table against NCTForge's NJOY2016.78 production outputs found corresponding
grids and a maximum relative difference of `4.892060e-7`. O-17 and O-18 have no
photon-production reactions and their MT 301 responses are effectively equal to
local-heating MT 901; therefore this evidence confirms, rather than removes, the
transported-photon KERMA qualification blocker. See
[`OPENMC_ENDFB81_PROCESSED_DATA_FINDINGS.md`](OPENMC_ENDFB81_PROCESSED_DATA_FINDINGS.md).

A later `NF-BNCT-001-SAB` variant will add hydrogen bound in water. A later
nuclear-data sensitivity study will repeat the calculation with at least
ENDF/B-VIII.0 and one non-ENDF evaluation. These variants must never overwrite
the baseline result.

Every run records:

- OpenMC semantic version, source commit when available, build options, and
  executable hash;
- the reviewed acquisition profile and receipt hashes, archive URL, exact byte
  count, publisher-digest status, and SHA-256 of the nuclear-data distribution;
- `cross_sections.xml` hash and hashes of every used HDF5 table;
- evaluated-data release, processing code/version, temperature, and thermal
  scattering tables;
- source definition, seed, stride, batches, particles per batch, MPI ranks,
  threads, and transport cutoffs; and
- input, statepoint, log, response-table, normalized result, and comparison
  artifact hashes.

“ENDF/B-VIII.1” alone is not a reproducible nuclear-data identifier.

## Statistical uncertainty

OpenMC reports statistics from batch realizations. NCTForge will retain the
component mean and one-sigma absolute standard uncertainty. Relative uncertainty
is a derived, nullable value and is undefined for a zero mean.

The following are distinct and must not be collapsed into one number:

1. Monte Carlo sampling uncertainty;
2. response interpolation and energy-group discretization error;
3. nuclear-data uncertainty and library-to-library sensitivity;
4. geometry, material, and source-model uncertainty; and
5. experimental uncertainty when measurements are added.

Component tallies from the same particle histories are correlated. NCTForge
must not estimate total-dose uncertainty as the root-sum-square of component
standard deviations unless covariance is available and used. The total needs a
dedicated estimator, batch-level covariance, or an explicit
`uncertainty_not_available` state.

Voxel precision is evaluated only in a declared scoring region. Relative-error
criteria are not applied to zero or negligible tallies. Aggregate ROI scores and
three independent seeds are required for the qualified reference run.

## Geometry baseline

NCTForge's canonical geometry is the DICOM patient-based right-handed LPS frame
in millimetres. For a biped, positive x is patient-left, positive y is posterior,
and positive z is toward the head.

`GridGeometry` is interpreted as follows:

- `shape = [columns, rows, slices]`;
- `spacing_mm = [column spacing, row spacing, slice spacing]`;
- `origin_mm` is the centre of voxel `[0, 0, 0]`; and
- the columns of `direction` map the column, row, and slice index axes into LPS.

The DICOM slice direction is the cross product of the row and column direction
cosines. Slice order and spacing are derived by projecting `Image Position
(Patient)` onto that normal; `Instance Number`, `Slice Location`, and nominal
`Slice Thickness` do not determine the stack geometry.

Contours are interpreted in patient coordinates and accepted only when their
referenced Frame of Reference is resolvable. Ambiguous, non-orthonormal,
duplicate, irregular, or mismatched geometry is rejected rather than silently
repaired in the first milestone.

The OpenMC adapter preserves the axes and converts millimetres to centimetres;
it does not introduce an LPS-to-RAS flip.

The benchmark writer and production importer must not share the code that
computes the expected affine or masks. Generated objects are also checked with
an external DICOM IOD validator. This prevents a writer and reader with the same
mistake from “verifying” each other.

## Independent evidence ladder

NCTForge will use the following qualification language:

1. **Exact/analytic checks:** units, transforms, source sampling, reaction-rate
   identities, and energy-accounting invariants.
2. **Single-code verification:** independent OpenMC estimators and convergence
   studies.
3. **Cross-code corroboration:** a separately implemented Geant4 case first,
   followed by MCNP or PHITS results from appropriately licensed collaborators.
4. **Experimental validation:** measured thermal-neutron and photon profiles in
   a sufficiently large water/tissue-equivalent phantom.
5. **Clinical qualification:** outside the present project scope and impossible
   to infer from the preceding steps alone.

Geant4 is suitable for the first openly redistributable comparison harness.
MCNP remains especially valuable to the field, but its distribution is export
controlled. PHITS requires an individual use licence. Neither restricted code
will be bundled with NCTForge.

OpenMC results remain `synthetic_research_only` until a genuinely independent
result exists. Cross-code agreement alone remains `cross_code_research_only`.

## Sources and design consequences

- [IAEA, *Advances in Boron Neutron Capture Therapy* (2023)](https://www.iaea.org/publications/15339/advances-in-boron-neutron-capture-therapy): four principal components, macroscopic KERMA guidance, nuclear-data risks, QA phantoms, and the need for cross-code and measurement comparisons.
- [IAEA-TECDOC-1223](https://www-pub.iaea.org/MTCD/Publications/PDF/te_1223_prn.pdf): historical reporting requirement to retain the four physical components separately from biological weighting.
- [OpenMC tally guide](https://docs.openmc.org/en/stable/usersguide/tallies.html): score units, filters, reaction-rate scores, heating, and tally normalization.
- [OpenMC 0.16.0 release](https://github.com/openmc-dev/openmc/releases/tag/v0.16.0): released implementation pinned by the first candidate run.
- [OpenMC 0.16.0 neutron-heating scoring](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/tallies/tally_scoring.cpp): total rather than reaction-wise neutron-heating response used by the estimator.
- [OpenMC ReactionFilter](https://docs.openmc.org/en/v0.16.0/pythonapi/generated/openmc.ReactionFilter.html): event-reaction filter added in 0.16.0.
- [OpenMC energy-deposition methods](https://docs.openmc.org/en/stable/methods/energy_deposition.html): MT=301/901 KERMA behavior and charged-particle energy-deposition assumptions.
- [OpenMC tally statistics](https://docs.openmc.org/en/stable/methods/tallies.html): batch means and standard-deviation estimation.
- [OpenMC official data libraries](https://openmc.org/data/): processed nuclear-data releases and temperatures.
- [OpenMC cross-section representation](https://docs.openmc.org/en/latest/methods/cross_sections.html): use of common ACE-derived data for direct code comparisons.
- [ESTRO 2024 OpenMC BNCT study](https://user-swndwmf.cld.bz/ESTRO-2024-Abstract-Book/3464/): voxel-specific KERMA/TLE approach, OpenMC performance, and the warning to examine all tissue reactions.
- [DICOM PS3.3 2026c, Image Plane Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.2.html): patient coordinates, image position/orientation, pixel spacing, and index-to-world mapping.
- [DICOM PS3.3 2026c, Structure Set Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.8.8.5.html) and [ROI Contour Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.8.8.6.html): Frame of Reference and contour-coordinate semantics.
- [DICOM PS3.5 2026c, UUID-derived UIDs](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_B.2.html): the `2.25.<UUID integer>` identifiers used by the synthetic generator.
- [DICOM-rs](https://github.com/Enet4/dicom-rs): permissively licensed Rust parsing/writing foundation selected for the production importer.
- [`dciodvfy`](https://manpages.debian.org/unstable/dicom3tools/dciodvfy.1.en.html): external IOD validation for generated benchmark objects.
- [NIST ICRU four-component soft tissue](https://physics.nist.gov/cgi-bin/Star/compos.pl?matno=262): density and elemental mass fractions for the synthetic material.
- [Geant4 licence](https://geant4.web.cern.ch/download/license), [MCNP distribution](https://mcnp.lanl.gov/how_to_get_the_mcnp_code.html), and [PHITS licence application](https://phits.jaea.go.jp/forms/license-en-new/index.html): constraints on comparison backends and redistribution.

## Implementation gates raised by this research

- Complete: replace the literal `HydrogenRecoil` contract with the conventional
  but explicitly defined `hydrogen` component.
- Complete: store absolute uncertainty, derive relative uncertainty only for a
  nonzero mean, and retain independently estimated physical-total uncertainty.
- Complete: bind the interchange bundle to content-hashed component-profile and
  neutron-response-set identities, and freeze the profile's contributor ledger.
- Complete: validate the response-set grid, units, interpolation, content
  references, pointwise neutron-KERMA closure, and review state; pending: create
  a table only after its derivation and evaluated inputs pass independent review.
- Complete: inspect and validate a case-scoped OpenMC nuclear-data manifest,
  including exact file hashes, cross-sections mappings, HDF5 format,
  temperatures, selected neutron energy bounds, MT 301, required reaction MTs,
  photon production, photoatomic data, atomic relaxation, and Compton profiles,
  and bind the acquisition profile, receipt, source URI, byte count, and archive
  identity into manifest schema `0.3.0`. Complete: run it on and freeze the
  processed official OpenMC ENDF/B-VIII.1 distribution selection.
- Complete: probe and safely acquire official data through a Rust path with
  HTTPS redirect confinement, exact resumable ranges, no overwrite, explicit
  size confirmation, publisher-digest verification when available, and an
  acquisition-only receipt.
- Complete: acquire the current NNDC ENDF/B-VIII.1 neutron archive, bind its
  receipt to the frozen material, and hash the exact ten selected evaluations.
  Complete: demonstrate pointwise MT 301 equivalence to the official processed
  library; source-container identity remains distinct and disclosed.
- Complete: generate byte-stable OpenMC 0.16 geometry, material, source,
  settings, response, audit, spectrum, and leakage XML directly in Rust; verify
  all content bindings and selected nuclear-data files; reject incomplete
  response energy coverage; and emit a hashed input manifest. Pending: supply
  real reviewed response tables and pass a controlled OpenMC smoke execution.
- Complete: freeze the NJOY2016.78 MT 407/403 partial-KERMA method and MT 301
  residual classification in ADR 0007, bind each evaluated ENDF material, and
  generate byte-stable production and diagnostic input decks. Complete: execute
  all ten nuclides through the controlled evidence path and preserve the
  rejected receipt after four MT 301 diagnostic failures. Complete: structure
  the matching photon-data fallback/incompleteness messages and reject the
  source selection for transported-photon KERMA. Complete: confirm against the
  official OpenMC tables that O-17 and O-18 retain effective local-photon
  fallback. Complete: bind an exact MF=6/12/13/14/15 source inventory and
  correct the log-only File 12 false-positive through a source-aware v0.2
  report; JEFF N-15 clears, but both selections remain rejected. Complete:
  independently reproduce all eight supported N-15 File 13/File 15 continuum
  energy moments and match NJOY at 58 shared source nodes. Complete:
  independently reconstruct JEFF N-15 MF=6/MT=102 photon and recoil moments
  and reject 33 of 37 source nodes on a 1% Q-value balance screen, while also
  matching NJOY's printed photon/recoil moments at 23 shared nodes. Complete:
  derive a content-bound common OpenMC transport domain from the exact
  manifest and material and apply it symmetrically; all baseline findings
  remain in domain, while O-16's sole JEFF finding is retained at 30 MeV but
  no longer rejects the bounded 20 MeV calculation. Complete: independently
  integrate all 54 JEFF H-2 MF=6/MT=16 LAW=7 source nodes; all 53 active nodes
  normalize and leave positive energy for the implicit proton. Complete: bind
  that result to the exact receipt and attribute all 15 H-2 findings to NJOY's
  excluded File 6 energy-balance remainder. Complete: add reaction-evidence-
  aware suitability schema `0.4.0`, clear only H-2 under that exact
  attribution, and incorporate N-15's independent capture-balance rejection;
  102 in-domain findings remain across C-13, O-17, and O-18. Pending: diagnose
  those three nuclides, then define, justify, and review a passing versioned
  response treatment.
- Do not use reaction-filtered neutron heating as a component partition; retain
  it only as a diagnostic under ADR 0005.
- Do not publish “golden” OpenMC dose arrays before an independent calculation is
  available.
- Do not implement boron-map recomposition or optimization in this benchmark;
  those subjects remain behind the documented Avify Dose IP review boundary.

## Ready implementation milestone

R1 is complete: one CLI command generates `NF-BNCT-001`, arbitrary file order
does not change the imported geometry, every declared affine and ROI mask is
reproduced, external DICOM IOD/entity validation is warning-free, and frozen
malformed variants fail closed. R2 now contains the response ledger,
transport-neutral material/source contracts, a verified official nuclear-data
selection, and deterministic OpenMC 0.16.0 deck generation. Adapter preparation
remains disabled until reviewed response artifacts satisfy the complete gate.
