# openbnct-openmc

OpenMC transport adapter for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT). Generates the pinned
OpenMC 0.16.0 input deck (materials, geometry, settings, tallies) from a
`TransportCase`, runs it under controlled conditions, and collects statepoint
results into the backend-neutral `PhysicalDoseBundle`. Owns the OpenMC input
manifest, run receipt, acceptance contract/report, nuclear-data acquisition and
selection contracts, and the weight-window emit/`vr validate` path.

Every generated run binds its inputs, data library, and variance-reduction
artifacts by SHA-256 in the input manifest.

Research software — results are benchmark evidence, not clinical predictions.
