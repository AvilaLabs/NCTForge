# Third-Party Licenses

NCTForge does not vendor third-party source, transport engines, nuclear data,
or patient data. Rust packages are resolved from crates.io and locked by the
committed `Cargo.lock`.

The current direct third-party packages are:

| Package | Resolved version | Declared license |
| --- | ---: | --- |
| clap | 4.6.6 | MIT OR Apache-2.0 |
| dicom-core | 0.10.0 | MIT OR Apache-2.0 |
| dicom-dictionary-std | 0.10.0 | MIT OR Apache-2.0 |
| dicom-object | 0.10.0 | MIT OR Apache-2.0 |
| eframe | 0.36.1 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| thiserror | 2.0.20 | MIT OR Apache-2.0 |
| uuid | 1.26.0 | Apache-2.0 OR MIT |
| tempfile (development only) | 3.27.0 | MIT OR Apache-2.0 |

This table records direct packages, not a substitute for the complete
transitive license and notice bundle. CI must generate and review that bundle
from `Cargo.lock` before the first binary or archival release.

OpenMC is a planned external transport backend and is not part of NCTForge.
Users are responsible for obtaining and validating OpenMC and all nuclear-data
libraries used by their calculations.
