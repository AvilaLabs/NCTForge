# ADR 0004: Patient-aligned anatomical view conventions

**Status:** Accepted for R1

**Date:** 2026-08-31

## Context

A view can be internally consistent while still reversing patient left/right,
anterior/posterior, or superior/inferior. Those errors are particularly easy to
miss with symmetric synthetic phantoms. View mapping therefore belongs in a
tested geometry layer rather than ad hoc egui texture code.

## Decision

R1 names and labels anatomical views only for grids whose direction matrix is
aligned to canonical DICOM LPS axes within `1e-6`. The screen mappings are:

| View | Fixed grid axis | Screen horizontal | Screen vertical | Edge labels |
| --- | --- | --- | --- | --- |
| Axial | slice / z | column / +x | row / +y | R–L, A–P |
| Coronal | row / y | column / +x | reversed slice / -z | R–L, S–I |
| Sagittal | column / x | row / +y | reversed slice / -z | A–P, S–I |

The single crosshair is stored as `[column, row, slice]`. Selecting a pixel in
one view replaces the two visible coordinates while preserving that view's
fixed coordinate. Every view is then regenerated from the same crosshair.

The mapping, extraction order, normalized screen-edge behavior, patient labels,
world-coordinate conversion, and three-plane round trip are tested without a
windowing toolkit. egui consumes this API and does not recreate the formulas.

## Consequences

The R1 GUI fails closed for oblique, flipped, or permuted input grids even when
the DICOM importer can represent them numerically. Supporting those cases will
require a separately specified patient-space resampling policy and
orientation-sensitive golden images. Until then, displaying generic grid axes
is preferable to asserting incorrect anatomical labels, but that generic mode
is outside the R1 milestone.
