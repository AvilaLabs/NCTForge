# openbnct-dicom

Strict, fail-closed DICOM boundary for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT). Part 10 CT import with
projected-position slice ordering and orthonormal LPS grids, RTSTRUCT polygon
rasterization into voxel masks, RT Dose export, and the deterministic synthetic
`nf-bnct-001` DICOM case generator. Refuses ambiguous or unsupported encodings
explicitly rather than guessing.

Research software — the generated DICOM is a synthetic benchmark, not a
patient artifact.
