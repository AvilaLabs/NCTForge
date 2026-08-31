// SPDX-License-Identifier: Apache-2.0

//! Independent oracle for the frozen `NF-BNCT-001` DICOM geometry case.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{DicomError, Result, import_ct_series, import_rtstruct};

const EXPECTED_FRAME_UID: &str = "2.25.240883953911088373736134884257182446642";
const EXPECTED_STUDY_UID: &str = "2.25.149214599444245138262873740736845471752";
const EXPECTED_SERIES_UID: &str = "2.25.337319594251465962942344971245692083782";
const EXPECTED_FIRST_SLICE_UID: &str = "2.25.43546999367060429143037900891741988095";
const EXPECTED_LAST_SLICE_UID: &str = "2.25.224181827055039319855832006853907618875";

#[derive(Debug, Clone, PartialEq)]
pub struct RoiReport {
    pub number: i32,
    pub name: String,
    pub voxel_count: usize,
    pub volume_cm3: f64,
    pub centroid_lps_mm: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub case_id: &'static str,
    pub shape: [u32; 3],
    pub spacing_mm: [f64; 3],
    pub origin_mm: [f64; 3],
    pub ct_slice_count: usize,
    pub rois: Vec<RoiReport>,
}

struct ExpectedRoi {
    number: i32,
    name: &'static str,
    voxel_count: usize,
    volume_cm3: f64,
    centroid_lps_mm: [f64; 3],
}

const EXPECTED_ROIS: [ExpectedRoi; 5] = [
    ExpectedRoi {
        number: 1,
        name: "PHANTOM",
        voxel_count: 64_000,
        volume_cm3: 8_000.0,
        centroid_lps_mm: [0.0, 0.0, 0.0],
    },
    ExpectedRoi {
        number: 2,
        name: "CORE",
        voxel_count: 512,
        volume_cm3: 64.0,
        centroid_lps_mm: [0.0, 0.0, 0.0],
    },
    ExpectedRoi {
        number: 3,
        name: "LEFT_ANTERIOR_MARKER",
        voxel_count: 64,
        volume_cm3: 8.0,
        centroid_lps_mm: [70.0, -70.0, -70.0],
    },
    ExpectedRoi {
        number: 4,
        name: "RIGHT_POSTERIOR_MARKER",
        voxel_count: 64,
        volume_cm3: 8.0,
        centroid_lps_mm: [-70.0, 70.0, 70.0],
    },
    ExpectedRoi {
        number: 5,
        name: "CENTRAL_AXIS_2CM",
        voxel_count: 8,
        volume_cm3: 1.0,
        centroid_lps_mm: [0.0, 0.0, -80.0],
    },
];

/// Verify generated `NF-BNCT-001` files against an independent frozen oracle.
pub fn verify_nf_bnct_001(root: &Path) -> Result<BenchmarkReport> {
    let ct_files = dicom_files(&root.join("ct"))?;
    if ct_files.len() != 40 {
        return mismatch(format!(
            "CT directory contains {} DICOM files; expected 40",
            ct_files.len()
        ));
    }
    let ct = import_ct_series(&ct_files)?;
    expect_equal("shape", &ct.geometry.shape, &[40, 40, 40])?;
    expect_equal("spacing", &ct.geometry.spacing_mm, &[5.0, 5.0, 5.0])?;
    expect_equal("origin", &ct.geometry.origin_mm, &[-97.5, -97.5, -97.5])?;
    expect_equal(
        "direction",
        &ct.geometry.direction,
        &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    )?;
    expect_equal(
        "Frame of Reference UID",
        &ct.frame_of_reference_uid,
        &EXPECTED_FRAME_UID.to_owned(),
    )?;
    expect_equal(
        "Study Instance UID",
        &ct.study_instance_uid,
        &EXPECTED_STUDY_UID.to_owned(),
    )?;
    expect_equal(
        "CT Series Instance UID",
        &ct.series_instance_uid,
        &EXPECTED_SERIES_UID.to_owned(),
    )?;
    expect_equal(
        "first CT SOP Instance UID",
        &ct.slice_sop_instance_uids[0],
        &EXPECTED_FIRST_SLICE_UID.to_owned(),
    )?;
    expect_equal(
        "last CT SOP Instance UID",
        &ct.slice_sop_instance_uids[39],
        &EXPECTED_LAST_SLICE_UID.to_owned(),
    )?;
    if ct.rescale_slope != 1.0
        || ct.rescale_intercept != 0.0
        || ct.stored_pixels.iter().any(|pixel| *pixel != 0)
    {
        return mismatch("CT pixel or rescale values differ from the frozen all-zero HU volume");
    }

    let rtstruct_path = root.join("rtstruct.dcm");
    let structures = import_rtstruct(&rtstruct_path, &ct)?;
    if structures.rois.len() != EXPECTED_ROIS.len() {
        return mismatch(format!(
            "RT Structure Set contains {} ROIs; expected {}",
            structures.rois.len(),
            EXPECTED_ROIS.len()
        ));
    }

    let mut reports = Vec::with_capacity(EXPECTED_ROIS.len());
    for expected in EXPECTED_ROIS {
        let roi = structures
            .roi(expected.name)
            .ok_or_else(|| DicomError::Benchmark(format!("missing ROI {:?}", expected.name)))?;
        let voxel_count = roi.voxel_count();
        let volume_cm3 = roi.volume_cm3(&ct);
        let centroid_lps_mm = roi
            .centroid_lps_mm(&ct)
            .ok_or_else(|| DicomError::Benchmark(format!("ROI {:?} is empty", expected.name)))?;
        if roi.number != expected.number
            || voxel_count != expected.voxel_count
            || volume_cm3 != expected.volume_cm3
            || centroid_lps_mm != expected.centroid_lps_mm
        {
            return mismatch(format!(
                "ROI {:?} differs: number={}, voxels={}, volume_cm3={}, centroid={centroid_lps_mm:?}",
                expected.name, roi.number, voxel_count, volume_cm3
            ));
        }
        reports.push(RoiReport {
            number: roi.number,
            name: roi.name.clone(),
            voxel_count,
            volume_cm3,
            centroid_lps_mm,
        });
    }

    Ok(BenchmarkReport {
        case_id: "NF-BNCT-001",
        shape: ct.geometry.shape,
        spacing_mm: ct.geometry.spacing_mm,
        origin_mm: ct.geometry.origin_mm,
        ct_slice_count: ct.slice_sop_instance_uids.len(),
        rois: reports,
    })
}

fn dicom_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(directory).map_err(|source| DicomError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DicomError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let is_file = entry
            .file_type()
            .map_err(|source| DicomError::Io {
                path: path.clone(),
                source,
            })?
            .is_file();
        let is_dicom = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dcm"));
        if is_file && is_dicom {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn expect_equal<T: PartialEq + std::fmt::Debug>(
    label: &str,
    observed: &T,
    expected: &T,
) -> Result<()> {
    if observed != expected {
        return mismatch(format!(
            "{label} differs: observed {observed:?}, expected {expected:?}"
        ));
    }
    Ok(())
}

fn mismatch<T>(message: impl Into<String>) -> Result<T> {
    Err(DicomError::Benchmark(message.into()))
}
