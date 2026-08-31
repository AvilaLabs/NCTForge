// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use nctforge_core::{GridGeometry, ValidationError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MASS_FRACTION_TOLERANCE: f64 = 1.0e-12;
const UNIT_VECTOR_TOLERANCE: f64 = 1.0e-12;

/// A transport-ready material with no backend-dependent element expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDefinition {
    pub schema_version: String,
    pub id: String,
    pub density_g_cm3: f64,
    pub temperature_k: f64,
    pub nuclides: Vec<NuclideMassFraction>,
    pub neutron_thermal_treatment: NeutronThermalTreatment,
}

impl MaterialDefinition {
    pub fn validate(&self) -> Result<(), TransportModelError> {
        validate_identifier("material.schema_version", &self.schema_version)?;
        validate_identifier("material.id", &self.id)?;
        if !self.density_g_cm3.is_finite() || self.density_g_cm3 <= 0.0 {
            return Err(TransportModelError::InvalidDensity);
        }
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(TransportModelError::InvalidTemperature);
        }
        if self.nuclides.is_empty() {
            return Err(TransportModelError::EmptyComposition);
        }

        let mut names = BTreeSet::new();
        let mut sum = 0.0;
        for nuclide in &self.nuclides {
            if !is_nuclide_name(&nuclide.name) {
                return Err(TransportModelError::InvalidNuclideName(
                    nuclide.name.clone(),
                ));
            }
            if !names.insert(nuclide.name.as_str()) {
                return Err(TransportModelError::DuplicateNuclide(nuclide.name.clone()));
            }
            if !nuclide.mass_fraction.is_finite() || nuclide.mass_fraction <= 0.0 {
                return Err(TransportModelError::InvalidMassFraction(
                    nuclide.name.clone(),
                ));
            }
            sum += nuclide.mass_fraction;
        }
        if (sum - 1.0).abs() > MASS_FRACTION_TOLERANCE {
            return Err(TransportModelError::MassFractionsDoNotSumToOne { sum });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuclideMassFraction {
    /// GNDS-style nuclide name such as `H1`, `B10`, or `Am242_m1`.
    pub name: String,
    pub mass_fraction: f64,
}

/// Treatment below the resolved-resonance range for this material.
///
/// R2 intentionally supports only free-gas scattering. Bound-atom tables will
/// require a new, content-bound contract rather than an untracked string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeutronThermalTreatment {
    FreeGas,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedSourceDefinition {
    pub schema_version: String,
    pub id: String,
    pub particle: ParticleType,
    /// Number of source sites sampled for each source history.
    pub source_sites_per_history: u32,
    /// Statistical weight assigned to each sampled source site.
    pub statistical_weight_per_site: f64,
    pub space: SourceSpatialDistribution,
    pub angle: AngularDistribution,
    pub energy: EnergyDistribution,
}

impl FixedSourceDefinition {
    pub fn validate(&self) -> Result<(), TransportModelError> {
        validate_identifier("source.schema_version", &self.schema_version)?;
        validate_identifier("source.id", &self.id)?;
        if self.source_sites_per_history != 1 {
            return Err(TransportModelError::UnsupportedSourceSitesPerHistory(
                self.source_sites_per_history,
            ));
        }
        if self.statistical_weight_per_site != 1.0 {
            return Err(TransportModelError::UnsupportedSourceWeight(
                self.statistical_weight_per_site,
            ));
        }

        match &self.space {
            SourceSpatialDistribution::UniformCartesianPlane {
                x_range_cm,
                y_range_cm,
                z_cm,
                ..
            } => {
                if !valid_interval(*x_range_cm) || !valid_interval(*y_range_cm) || !z_cm.is_finite()
                {
                    return Err(TransportModelError::InvalidSourceSpace);
                }
            }
        }
        match &self.angle {
            AngularDistribution::Monodirectional { unit_vector } => {
                if unit_vector.iter().any(|value| !value.is_finite()) {
                    return Err(TransportModelError::InvalidSourceDirection);
                }
                let norm_squared = unit_vector[0].mul_add(
                    unit_vector[0],
                    unit_vector[1].mul_add(unit_vector[1], unit_vector[2] * unit_vector[2]),
                );
                if (norm_squared - 1.0).abs() > UNIT_VECTOR_TOLERANCE {
                    return Err(TransportModelError::InvalidSourceDirection);
                }
            }
        }
        match self.energy {
            EnergyDistribution::Monoenergetic { energy_ev }
                if !energy_ev.is_finite() || energy_ev <= 0.0 =>
            {
                return Err(TransportModelError::InvalidSourceEnergy);
            }
            EnergyDistribution::Monoenergetic { .. } => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticleType {
    Neutron,
    Photon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceSpatialDistribution {
    /// Uniform sampling in x and y at a fixed z in Cartesian centimetres.
    UniformCartesianPlane {
        x_range_cm: [f64; 2],
        y_range_cm: [f64; 2],
        z_cm: f64,
        interval_convention: IntervalConvention,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalConvention {
    HalfOpen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AngularDistribution {
    Monodirectional { unit_vector: [f64; 3] },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnergyDistribution {
    Monoenergetic { energy_ev: f64 },
}

/// Complete backend-neutral input to one transport preparation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportCase {
    pub schema_version: String,
    pub case_id: String,
    pub geometry: GridGeometry,
    pub material: MaterialDefinition,
    pub source: FixedSourceDefinition,
    /// Requested independent source histories, not batches or source weight.
    pub requested_histories: u64,
}

impl TransportCase {
    pub fn validate(&self) -> Result<(), TransportModelError> {
        validate_identifier("transport_case.schema_version", &self.schema_version)?;
        validate_identifier("transport_case.case_id", &self.case_id)?;
        self.geometry.voxel_count()?;
        self.material.validate()?;
        self.source.validate()?;
        if self.requested_histories == 0 {
            return Err(TransportModelError::ZeroRequestedHistories);
        }
        Ok(())
    }
}

fn validate_identifier(label: &'static str, value: &str) -> Result<(), TransportModelError> {
    if value.trim().is_empty() {
        Err(TransportModelError::EmptyIdentifier(label))
    } else {
        Ok(())
    }
}

fn valid_interval(interval: [f64; 2]) -> bool {
    interval.iter().all(|value| value.is_finite()) && interval[0] < interval[1]
}

fn is_nuclide_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_uppercase() {
        return false;
    }
    let mut index = 1;
    if index < bytes.len() && bytes[index].is_ascii_lowercase() {
        index += 1;
    }
    let mass_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if mass_start == index || bytes[mass_start] == b'0' {
        return false;
    }
    if index == bytes.len() {
        return true;
    }
    if !bytes[index..].starts_with(b"_m") {
        return false;
    }
    index += 2;
    let state_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    state_start < index && bytes[state_start] != b'0' && index == bytes.len()
}

#[derive(Debug, Error, PartialEq)]
pub enum TransportModelError {
    #[error(transparent)]
    Geometry(#[from] ValidationError),
    #[error("required identifier {0} is empty")]
    EmptyIdentifier(&'static str),
    #[error("material density must be finite and greater than zero g/cm3")]
    InvalidDensity,
    #[error("material temperature must be finite and greater than zero kelvin")]
    InvalidTemperature,
    #[error("material nuclide composition is empty")]
    EmptyComposition,
    #[error("invalid GNDS-style nuclide name {0:?}")]
    InvalidNuclideName(String),
    #[error("nuclide {0} has a non-positive or non-finite mass fraction")]
    InvalidMassFraction(String),
    #[error("nuclide {0} occurs more than once")]
    DuplicateNuclide(String),
    #[error("material mass fractions sum to {sum}; expected one within 1e-12")]
    MassFractionsDoNotSumToOne { sum: f64 },
    #[error("this normalization profile requires one source site per history, observed {0}")]
    UnsupportedSourceSitesPerHistory(u32),
    #[error("this normalization profile requires unit source weight, observed {0}")]
    UnsupportedSourceWeight(f64),
    #[error("source spatial distribution has an invalid interval or coordinate")]
    InvalidSourceSpace,
    #[error("source direction must be a finite unit vector")]
    InvalidSourceDirection,
    #[error("source energy must be finite and greater than zero eV")]
    InvalidSourceEnergy,
    #[error("transport case requests zero source histories")]
    ZeroRequestedHistories,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATERIAL_JSON: &str =
        include_str!("../../../benchmarks/synthetic/nf-bnct-001/transport/material.json");
    const SOURCE_JSON: &str =
        include_str!("../../../benchmarks/synthetic/nf-bnct-001/transport/source.json");

    fn material() -> MaterialDefinition {
        serde_json::from_str(MATERIAL_JSON).unwrap()
    }

    fn source() -> FixedSourceDefinition {
        serde_json::from_str(SOURCE_JSON).unwrap()
    }

    #[test]
    fn frozen_material_is_exact_and_valid() {
        let material = material();
        assert_eq!(material.id, "nctforge.nf-bnct-001.material.v1");
        assert_eq!(material.nuclides.len(), 10);
        assert_eq!(material.validate(), Ok(()));
    }

    #[test]
    fn frozen_source_is_exact_and_valid() {
        let source = source();
        assert_eq!(source.id, "nctforge.nf-bnct-001.source.v1");
        assert_eq!(source.particle, ParticleType::Neutron);
        assert_eq!(source.validate(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_nuclide() {
        let mut material = material();
        material.nuclides.push(material.nuclides[0].clone());
        assert_eq!(
            material.validate(),
            Err(TransportModelError::DuplicateNuclide("H1".into()))
        );
    }

    #[test]
    fn rejects_nonunit_source_direction() {
        let mut source = source();
        source.angle = AngularDistribution::Monodirectional {
            unit_vector: [0.0, 0.0, 0.5],
        };
        assert_eq!(
            source.validate(),
            Err(TransportModelError::InvalidSourceDirection)
        );
    }

    #[test]
    fn validates_representative_nuclide_names() {
        assert!(is_nuclide_name("B10"));
        assert!(is_nuclide_name("Am242_m1"));
        assert!(!is_nuclide_name("B-10"));
        assert!(!is_nuclide_name("H01"));
        assert!(!is_nuclide_name("h1"));
    }
}
