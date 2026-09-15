# openbnct-boron

PET-derived boron uptake modeling for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT). Maps SUV volumes to
per-voxel B-10 fields (`openbnct.boron-field`) under declared uptake models
(`openbnct.boron-uptake-model`: tumor:blood-ratio, linear, uniform), with
optional time-dependent washout and per-voxel noise propagation. Optional
rigid-registration chaining resamples PET onto the case grid before mapping.

Research software — derived boron fields carry `pet_derived_boron_research_only`
qualification.
