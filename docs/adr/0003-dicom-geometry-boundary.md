# ADR 0003: Fail-closed DICOM geometry boundary

**Status:** Accepted for R1

**Date:** 2026-08-31

## Context

Incorrect row/column interpretation, slice ordering, or frame linkage can yield
plausible-looking but spatially wrong dose comparisons. NCTForge therefore
needs a small, reviewable DICOM boundary whose successful return means more than
"the file parsed."

The DICOM Image Plane Module defines Image Position (Patient) as the centre of
the first transmitted voxel and Image Orientation (Patient) as the direction
cosines of the first row and first column. Patient coordinates are LPS for
BIPED anatomy. The RT Structure Set IOD links contours to a Frame of Reference
and referenced images.

Primary references:

- [DICOM PS3.3 C.7.6.2, Image Plane Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.2.html)
- [DICOM PS3.3 C.8.8.5, Structure Set Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.8.8.5.html)
- [DICOM PS3.3 C.8.8.6, ROI Contour Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.8.8.6.html)
- [DICOM PS3.5 B.2, UUID-derived `2.25` UIDs](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_B.2.html)

## Decision

1. Use `dicom-rs` 0.10.0 for DICOM Part 10 parsing and writing. Pin the three
   parsing-boundary crates exactly in the workspace manifest.
2. Keep NCTForge's coordinate and semantic checks in a dedicated
   `nctforge-dicom` crate. No `dicom-rs` object crosses into the core model.
3. Represent grids as `[column, row, slice]`. The origin is the first voxel
   centre in DICOM LPS millimetres. Direction-matrix columns are the increasing
   column, row, and slice axes; the matrix must be right-handed and orthonormal.
4. Sort CT slices by the dot product of Image Position (Patient) and the normal
   formed from the orientation vectors. Filenames and Instance Number are never
   geometric authorities.
5. Reject mixed frames, studies, series, dimensions, orientations, pixel
   spacing, rescale parameters, duplicate planes, nonuniform projected spacing,
   and in-plane slice drift.
6. R1 accepts only native signed 16-bit Explicit VR Little Endian, single-frame
   CT. It rejects compressed and enhanced multiframe objects until their decode
   and geometry paths have dedicated acceptance cases.
7. R1 RTSTRUCT import accepts `CLOSED_PLANAR` contours on referenced CT planes
   and samples masks at voxel centres. More than one polygon for one ROI on one
   plane is rejected until hole and XOR semantics are implemented and tested.
8. The deterministic benchmark writer and the frozen acceptance oracle remain
   separate implementations. Generated files must round-trip through the
   production importer and be byte-identical across runs.

## Consequences

The initial importer supports a deliberately narrow subset of real-world CT and
RTSTRUCT. This is preferable to silently accepting geometry NCTForge cannot yet
represent faithfully. Every broadened input class requires a malformed case and
an orientation-sensitive acceptance case.

Parsing and internal round-trip tests do not prove full DICOM IOD conformance.
An independent validator such as `dciodvfy` remains an R1 release gate and must
run against both CT and RT Structure Set outputs in CI before benchmark binaries
are published.
