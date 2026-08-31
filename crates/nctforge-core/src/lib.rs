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
}

/// The four physical dose groups retained before biological weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseComponent {
    Boron10,
    Nitrogen14,
    HydrogenRecoil,
    Photon,
}

impl DoseComponent {
    pub const REQUIRED: [Self; 4] = [
        Self::Boron10,
        Self::Nitrogen14,
        Self::HydrogenRecoil,
        Self::Photon,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseUnit {
    Gray,
    GrayPerSourceParticle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoseVolume {
    pub component: DoseComponent,
    pub unit: DoseUnit,
    pub values: Vec<f64>,
    /// One-sigma relative standard uncertainty per voxel, when available.
    pub relative_standard_uncertainty: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDoseBundle {
    pub schema_version: String,
    pub case_id: String,
    pub frame_of_reference_uid: Option<String>,
    pub geometry: GridGeometry,
    pub components: Vec<DoseVolume>,
    /// Identifier of the run manifest that binds inputs, engine, data, and logs.
    pub provenance_id: String,
}

impl PhysicalDoseBundle {
    pub fn validate(&self) -> Result<(), ValidationError> {
        let voxel_count = self.geometry.voxel_count()?;
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
            if let Some(uncertainty) = &volume.relative_standard_uncertainty {
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

        Ok(())
    }
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

    #[test]
    fn rejects_incomplete_component_bundle() {
        let bundle = PhysicalDoseBundle {
            schema_version: "0.1.0".into(),
            case_id: "synthetic".into(),
            frame_of_reference_uid: None,
            geometry: geometry(),
            components: Vec::new(),
            provenance_id: "not-yet-bound".into(),
        };

        assert_eq!(
            bundle.validate(),
            Err(ValidationError::MissingComponent(DoseComponent::Boron10))
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
}
