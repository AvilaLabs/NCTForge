# ADR 0002: Macroscopic BNCT Dose Semantics

**Status:** Accepted for implementation

**Date:** 2026-08-31

## Context

The R0 scaffold names the third component `HydrogenRecoil`. That name is too
narrow for a transport-neutral interchange contract. BNCT literature commonly
calls the component hydrogen dose, fast-neutron dose, neutron dose, or
proton-recoil dose, while the IAEA describes hydrogen as the main rather than
the exclusive contributor. Other tissue nuclides and reactions can contribute
locally deposited neutron energy.

OpenMC's neutron `heating` score is total rather than reaction-specific. A model
that reports only H-1 elastic scattering also cannot demonstrate closure against
total neutron heating.

## Decision

The first scientific profile is `nctforge.macroscopic-absorbed-dose.v1` and
contains four canonical components:

- `boron` (`D_B`);
- `nitrogen` (`D_N`);
- `hydrogen` (`D_H`); and
- `photon` (`D_gamma`).

`hydrogen` is a conventional group name. In this profile it means non-photon
neutron KERMA not assigned to the boron or nitrogen groups, with hydrogen recoil
expected to dominate. Each result records the response-table/profile identifier
and, when available, its nuclide/reaction contributors.

Photon energy is assigned to `photon` at its deposition location, regardless of
whether the photon entered with the source or was created by a neutron reaction.
It is not also deposited locally in another component.

The profile applies only at macroscopic voxel scales under a documented local
charged-particle/KERMA approximation. It does not define cellular dose.

## Data-model consequences

Before R2:

1. migrate `DoseComponent::HydrogenRecoil` to a serialized `hydrogen` value;
2. bind every bundle to a component-definition profile;
3. store absolute one-sigma uncertainty, allowing uncertainty to be absent;
4. represent relative uncertainty as derived and undefined when the mean is
   zero; and
5. represent total-dose uncertainty independently of component uncertainties.

Because this repository has no release or consumer, the early schema can break
cleanly rather than preserving an ambiguous serialized name.

## Consequences

- Results can be mapped to the four groups used in BNCT reporting without
  implying that only four nuclear reactions exist.
- Cross-code imports must declare their component mapping rather than matching
  labels loosely.
- Contributor-level differences remain diagnosable.
- Microdosimetry, biological weighting, and clinical qualification remain
  explicitly outside this profile.
