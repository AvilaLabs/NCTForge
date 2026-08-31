// SPDX-License-Identifier: Apache-2.0

//! Strict DICOM import and synthetic benchmark support.

#![forbid(unsafe_code)]

mod benchmark;
mod ct;
mod error;
mod rtstruct;
pub mod synthetic;

pub use benchmark::{BenchmarkReport, RoiReport, verify_nf_bnct_001};
pub use ct::{CtVolume, import_ct_series};
pub use error::{DicomError, Result};
pub use rtstruct::{RoiMask, StructureSet, import_rtstruct};
