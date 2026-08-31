// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A regular patient-coordinate voxel grid.
///
/// `direction` is row-major and maps voxel axes into the patient coordinate
/// frame. Geometry importers must preserve the original DICOM frame of
/// reference outside this numerical representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridGeometry {
    pub shape: [u32; 3],
    pub spacing_mm: [f64; 3],
    pub origin_mm: [f64; 3],
    pub direction: [f64; 9],
}

impl GridGeometry {
    pub fn voxel_count(&self) -> Result<usize, ValidationError> {
        if self.shape.contains(&0) {
            return Err(ValidationError::EmptyGrid);
        }
        if self
            .spacing_mm
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err(ValidationError::InvalidSpacing);
        }
        if self.origin_mm.iter().any(|value| !value.is_finite())
            || self.direction.iter().any(|value| !value.is_finite())
        {
            return Err(ValidationError::NonFiniteGeometry);
        }
        let axes = [
            [self.direction[0], self.direction[3], self.direction[6]],
            [self.direction[1], self.direction[4], self.direction[7]],
            [self.direction[2], self.direction[5], self.direction[8]],
        ];
        let dot = |left: [f64; 3], right: [f64; 3]| {
            left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
        };
        let determinant = self.direction[0]
            * (self.direction[4] * self.direction[8] - self.direction[5] * self.direction[7])
            - self.direction[1]
                * (self.direction[3] * self.direction[8] - self.direction[5] * self.direction[6])
            + self.direction[2]
                * (self.direction[3] * self.direction[7] - self.direction[4] * self.direction[6]);
        const TOLERANCE: f64 = 1.0e-6;
        if axes
            .iter()
            .any(|axis| (dot(*axis, *axis) - 1.0).abs() > TOLERANCE)
            || dot(axes[0], axes[1]).abs() > TOLERANCE
            || dot(axes[0], axes[2]).abs() > TOLERANCE
            || dot(axes[1], axes[2]).abs() > TOLERANCE
            || (determinant - 1.0).abs() > TOLERANCE
        {
            return Err(ValidationError::InvalidDirection);
        }

        self.shape.iter().try_fold(1_usize, |count, extent| {
            count
                .checked_mul(*extent as usize)
                .ok_or(ValidationError::GridTooLarge)
        })
    }

    /// Return the patient-coordinate center of a voxel in millimetres.
    pub fn voxel_center_lps_mm(&self, voxel: [u32; 3]) -> Result<[f64; 3], ValidationError> {
        self.voxel_count()?;
        if voxel
            .into_iter()
            .zip(self.shape)
            .any(|(index, extent)| index >= extent)
        {
            return Err(ValidationError::VoxelOutOfBounds {
                voxel,
                shape: self.shape,
            });
        }
        let local = [
            f64::from(voxel[0]) * self.spacing_mm[0],
            f64::from(voxel[1]) * self.spacing_mm[1],
            f64::from(voxel[2]) * self.spacing_mm[2],
        ];
        Ok([
            self.origin_mm[0]
                + self.direction[0].mul_add(
                    local[0],
                    self.direction[1].mul_add(local[1], self.direction[2] * local[2]),
                ),
            self.origin_mm[1]
                + self.direction[3].mul_add(
                    local[0],
                    self.direction[4].mul_add(local[1], self.direction[5] * local[2]),
                ),
            self.origin_mm[2]
                + self.direction[6].mul_add(
                    local[0],
                    self.direction[7].mul_add(local[1], self.direction[8] * local[2]),
                ),
        ])
    }
}

/// The four physical dose groups retained before biological weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseComponent {
    Boron,
    Nitrogen,
    Hydrogen,
    Photon,
}

impl DoseComponent {
    pub const REQUIRED: [Self; 4] = [Self::Boron, Self::Nitrogen, Self::Hydrogen, Self::Photon];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseUnit {
    Gray,
    GrayPerSourceParticle,
}

/// Immutable identity of a scientific input artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    pub id: String,
    pub sha256: String,
}

impl ContentReference {
    pub fn validate(&self) -> Result<(), ContentReferenceError> {
        if self.id.trim().is_empty() {
            return Err(ContentReferenceError::EmptyId);
        }
        if !is_canonical_sha256(&self.sha256) {
            return Err(ContentReferenceError::InvalidSha256);
        }
        Ok(())
    }
}

/// Content reference used specifically for the component-definition profile.
pub type ComponentProfileReference = ContentReference;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoseVolume {
    pub component: DoseComponent,
    pub unit: DoseUnit,
    pub values: Vec<f64>,
    /// One-sigma absolute standard uncertainty in the same unit as `values`.
    pub absolute_standard_uncertainty: Option<Vec<f64>>,
}

impl DoseVolume {
    /// Derive relative uncertainty for one voxel.
    ///
    /// Relative uncertainty is deliberately absent when no absolute
    /// uncertainty exists or when the mean is zero.
    pub fn relative_standard_uncertainty(
        &self,
        voxel_index: usize,
    ) -> Result<Option<f64>, ValidationError> {
        let mean = self
            .values
            .get(voxel_index)
            .ok_or(ValidationError::DoseIndexOutOfBounds {
                index: voxel_index,
                length: self.values.len(),
            })?;
        let Some(uncertainty) = &self.absolute_standard_uncertainty else {
            return Ok(None);
        };
        let absolute = uncertainty
            .get(voxel_index)
            .ok_or(ValidationError::UncertaintyLength {
                component: self.component,
                expected: self.values.len(),
                actual: uncertainty.len(),
            })?;
        if *mean == 0.0 {
            Ok(None)
        } else {
            Ok(Some(*absolute / *mean))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TotalUncertaintyMethod {
    /// Uncertainty comes from a dedicated physical-total estimator.
    DedicatedEstimator,
    /// Uncertainty was calculated from batch-level component covariance.
    BatchCovariance,
    /// No defensible total uncertainty is available.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalTotalDoseVolume {
    pub unit: DoseUnit,
    pub values: Vec<f64>,
    pub absolute_standard_uncertainty: Option<Vec<f64>>,
    pub uncertainty_method: TotalUncertaintyMethod,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDoseBundle {
    pub schema_version: String,
    pub case_id: String,
    pub frame_of_reference_uid: Option<String>,
    pub geometry: GridGeometry,
    pub component_profile: ComponentProfileReference,
    /// Material- and nuclear-data-specific neutron response curves.
    pub response_set: ContentReference,
    pub components: Vec<DoseVolume>,
    /// A dedicated physical total, retained separately from component means.
    pub physical_total: PhysicalTotalDoseVolume,
    /// Identifier of the run manifest that binds inputs, engine, data, and logs.
    pub provenance_id: String,
}

impl PhysicalDoseBundle {
    pub fn validate(&self) -> Result<(), ValidationError> {
        let voxel_count = self.geometry.voxel_count()?;
        for (label, value) in [
            ("schema_version", self.schema_version.as_str()),
            ("case_id", self.case_id.as_str()),
            ("provenance_id", self.provenance_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::EmptyIdentifier(label));
            }
        }
        self.component_profile
            .validate()
            .map_err(|_| ValidationError::InvalidContentReference("component_profile"))?;
        self.response_set
            .validate()
            .map_err(|_| ValidationError::InvalidContentReference("response_set"))?;

        let mut observed = BTreeSet::new();

        for volume in &self.components {
            if !observed.insert(volume.component) {
                return Err(ValidationError::DuplicateComponent(volume.component));
            }
            if volume.values.len() != voxel_count {
                return Err(ValidationError::DoseLength {
                    component: volume.component,
                    expected: voxel_count,
                    actual: volume.values.len(),
                });
            }
            if volume
                .values
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            {
                return Err(ValidationError::InvalidDose(volume.component));
            }
            if volume.unit != self.physical_total.unit {
                return Err(ValidationError::DoseUnitMismatch {
                    component: volume.component,
                    component_unit: volume.unit,
                    total_unit: self.physical_total.unit,
                });
            }
            if let Some(uncertainty) = &volume.absolute_standard_uncertainty {
                if uncertainty.len() != voxel_count {
                    return Err(ValidationError::UncertaintyLength {
                        component: volume.component,
                        expected: voxel_count,
                        actual: uncertainty.len(),
                    });
                }
                if uncertainty
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0)
                {
                    return Err(ValidationError::InvalidUncertainty(volume.component));
                }
            }
        }

        for required in DoseComponent::REQUIRED {
            if !observed.contains(&required) {
                return Err(ValidationError::MissingComponent(required));
            }
        }

        if self.physical_total.values.len() != voxel_count {
            return Err(ValidationError::TotalDoseLength {
                expected: voxel_count,
                actual: self.physical_total.values.len(),
            });
        }
        if self
            .physical_total
            .values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(ValidationError::InvalidTotalDose);
        }
        if let Some(uncertainty) = &self.physical_total.absolute_standard_uncertainty {
            if uncertainty.len() != voxel_count {
                return Err(ValidationError::TotalUncertaintyLength {
                    expected: voxel_count,
                    actual: uncertainty.len(),
                });
            }
            if uncertainty
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            {
                return Err(ValidationError::InvalidTotalUncertainty);
            }
        }
        match (
            self.physical_total.absolute_standard_uncertainty.is_some(),
            self.physical_total.uncertainty_method,
        ) {
            (false, TotalUncertaintyMethod::Unavailable)
            | (true, TotalUncertaintyMethod::DedicatedEstimator)
            | (true, TotalUncertaintyMethod::BatchCovariance) => {}
            _ => return Err(ValidationError::InconsistentTotalUncertainty),
        }

        Ok(())
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("voxel grid contains an empty dimension")]
    EmptyGrid,
    #[error("voxel spacing must be finite and greater than zero")]
    InvalidSpacing,
    #[error("voxel geometry contains a non-finite value")]
    NonFiniteGeometry,
    #[error("voxel direction matrix must be right-handed and orthonormal")]
    InvalidDirection,
    #[error("voxel count overflows the addressable platform size")]
    GridTooLarge,
    #[error("voxel {voxel:?} is outside grid shape {shape:?}")]
    VoxelOutOfBounds { voxel: [u32; 3], shape: [u32; 3] },
    #[error("physical dose component {0:?} is missing")]
    MissingComponent(DoseComponent),
    #[error("physical dose component {0:?} occurs more than once")]
    DuplicateComponent(DoseComponent),
    #[error("{component:?} has {actual} dose values; expected {expected}")]
    DoseLength {
        component: DoseComponent,
        expected: usize,
        actual: usize,
    },
    #[error("{0:?} contains a negative or non-finite physical dose")]
    InvalidDose(DoseComponent),
    #[error("{component:?} has {actual} uncertainty values; expected {expected}")]
    UncertaintyLength {
        component: DoseComponent,
        expected: usize,
        actual: usize,
    },
    #[error("{0:?} contains a negative or non-finite uncertainty")]
    InvalidUncertainty(DoseComponent),
    #[error("dose index {index} is outside volume length {length}")]
    DoseIndexOutOfBounds { index: usize, length: usize },
    #[error("required identifier {0} is empty")]
    EmptyIdentifier(&'static str),
    #[error("{0} must have a nonempty ID and canonical lowercase SHA-256 digest")]
    InvalidContentReference(&'static str),
    #[error("{component:?} uses {component_unit:?}, but the physical total uses {total_unit:?}")]
    DoseUnitMismatch {
        component: DoseComponent,
        component_unit: DoseUnit,
        total_unit: DoseUnit,
    },
    #[error("physical total has {actual} dose values; expected {expected}")]
    TotalDoseLength { expected: usize, actual: usize },
    #[error("physical total contains a negative or non-finite dose")]
    InvalidTotalDose,
    #[error("physical total has {actual} uncertainty values; expected {expected}")]
    TotalUncertaintyLength { expected: usize, actual: usize },
    #[error("physical total contains a negative or non-finite uncertainty")]
    InvalidTotalUncertainty,
    #[error("physical-total uncertainty and its method are inconsistent")]
    InconsistentTotalUncertainty,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContentReferenceError {
    #[error("content reference ID is empty")]
    EmptyId,
    #[error("content reference SHA-256 must be 64 lowercase hexadecimal characters")]
    InvalidSha256,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> GridGeometry {
        GridGeometry {
            shape: [2, 2, 1],
            spacing_mm: [1.0, 1.0, 2.0],
            origin_mm: [0.0; 3],
            direction: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    fn volume(component: DoseComponent) -> DoseVolume {
        DoseVolume {
            component,
            unit: DoseUnit::GrayPerSourceParticle,
            values: vec![1.0; 4],
            absolute_standard_uncertainty: Some(vec![0.1; 4]),
        }
    }

    fn valid_bundle() -> PhysicalDoseBundle {
        PhysicalDoseBundle {
            schema_version: "nctforge.physical-dose-bundle/0.2.0".into(),
            case_id: "synthetic".into(),
            frame_of_reference_uid: None,
            geometry: geometry(),
            component_profile: ComponentProfileReference {
                id: "nctforge.macroscopic-absorbed-dose.v1".into(),
                sha256: "a".repeat(64),
            },
            response_set: ContentReference {
                id: "nctforge.synthetic-response-set.v1".into(),
                sha256: "b".repeat(64),
            },
            components: DoseComponent::REQUIRED.into_iter().map(volume).collect(),
            physical_total: PhysicalTotalDoseVolume {
                unit: DoseUnit::GrayPerSourceParticle,
                values: vec![4.0; 4],
                absolute_standard_uncertainty: Some(vec![0.2; 4]),
                uncertainty_method: TotalUncertaintyMethod::DedicatedEstimator,
            },
            provenance_id: "manifest-sha256:synthetic".into(),
        }
    }

    #[test]
    fn rejects_incomplete_component_bundle() {
        let mut bundle = valid_bundle();
        bundle.components.clear();

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::MissingComponent(DoseComponent::Boron))
        );
    }

    #[test]
    fn rejects_non_orthonormal_grid_direction() {
        let mut invalid = geometry();
        invalid.direction[4] = 2.0;
        assert_eq!(
            invalid.voxel_count(),
            Err(ValidationError::InvalidDirection)
        );
    }

    #[test]
    fn validates_complete_physical_dose_contract() {
        assert_eq!(valid_bundle().validate(), Ok(()));
    }

    #[test]
    fn serializes_only_canonical_component_names() {
        assert_eq!(
            serde_json::to_string(&DoseComponent::Hydrogen).unwrap(),
            "\"hydrogen\""
        );
        assert!(serde_json::from_str::<DoseComponent>("\"hydrogen_recoil\"").is_err());
    }

    #[test]
    fn derives_relative_uncertainty_but_not_for_zero_mean() {
        let volume = DoseVolume {
            component: DoseComponent::Boron,
            unit: DoseUnit::Gray,
            values: vec![0.0, 2.0],
            absolute_standard_uncertainty: Some(vec![0.1, 0.2]),
        };

        assert_eq!(volume.relative_standard_uncertainty(0), Ok(None));
        let relative = volume.relative_standard_uncertainty(1).unwrap().unwrap();
        assert!((relative - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_mixed_component_and_total_units() {
        let mut bundle = valid_bundle();
        bundle.components[0].unit = DoseUnit::Gray;

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::DoseUnitMismatch {
                component: DoseComponent::Boron,
                component_unit: DoseUnit::Gray,
                total_unit: DoseUnit::GrayPerSourceParticle,
            })
        );
    }

    #[test]
    fn requires_consistent_total_uncertainty_state() {
        let mut bundle = valid_bundle();
        bundle.physical_total.uncertainty_method = TotalUncertaintyMethod::Unavailable;

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::InconsistentTotalUncertainty)
        );
    }

    #[test]
    fn rejects_noncanonical_component_profile_hash() {
        let mut bundle = valid_bundle();
        bundle.component_profile.sha256 = "A".repeat(64);

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::InvalidContentReference(
                "component_profile"
            ))
        );
    }

    #[test]
    fn rejects_noncanonical_response_set_hash() {
        let mut bundle = valid_bundle();
        bundle.response_set.sha256 = "B".repeat(64);

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::InvalidContentReference("response_set"))
        );
    }
}
