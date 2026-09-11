# Paper 1 — HEATR kinematic findings in BNCT tissue nuclides

**Status:** Evidence exists. Not blocked by R2.

**Date opened:** 2026-08-31

## Target venues

Nuclear-data audience, not a medical-physics one:

- *Applied Radiation and Isotopes* — carries BNCT nuclear- and radiobiological-data work
- *Annals of Nuclear Energy*
- *Nuclear Data Sheets*
- *EPJ Web of Conferences* — nuclear-data conference proceedings

## Claim

Processing ENDF/B-VIII.1 incident-neutron evaluations with NJOY2016.78 under a
frozen, fully documented recipe produces MT 301 kinematic-limit violations in
N-15, O-16, O-17, and O-18 — 72 violations in total, with zero in B-10, C-12,
C-13, H-1, H-2, and N-14. Separately, transported-photon KERMA is unsupported
for the same four nuclides: O-17 and O-18 have no photon-production files, N-15
lacks File 12, and O-16 has a potentially incomplete discrete photon sequence.

These four nuclides appear in every tissue composition used in BNCT dosimetry.
The finding is therefore about data the field shares, not about this project.

The contribution is that the recipe, the execution, and the findings are
content-bound and independently regenerable from the raw processor logs — so a
reader can reproduce the violation set rather than take it on assertion.

## Required evidence

- [x] Frozen, content-bound generation method (ADR 0007)
- [x] Deterministic input decks with a no-overwrite manifest (ADR 0011)
- [x] Controlled execution with preserved rejected receipt (ADR 0012)
- [x] Structured, independently regenerable suitability report (ADR 0013)
- [ ] Comparison against the official processed OpenMC ENDF/B-VIII.1
      distribution, establishing whether the same defect is present in the
      libraries the field actually uses — **this determines whether the paper
      is a note about one recipe or a finding about shared data**
- [ ] Explicit statement of which processing choices would suppress the
      violations, and what each costs physically

## Notes

The comparison in the second unchecked item is the difference between a minor
methods note and a result the field needs. Without it, a reviewer will ask
whether the violations are an artifact of this recipe alone.
