# openbnct-nifti

NIfTI-1 image import/export for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT) with explicit affine and
LPS/RAS semantics. Loads NIfTI volumes (including oblique grids), resamples in
world coordinates onto `GridGeometry`, and writes dose/field volumes back out.
Pairs with `openbnct.registration` transforms.

Research software.
