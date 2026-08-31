// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use nctforge_core::DoseComponent;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const JOULE_PER_EV: f64 = 1.602_176_634e-19;

/// A content-addressed artifact used as a scientific input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentHashReference {
    pub id: String,
    pub sha256: String,
}

impl ContentHashReference {
    fn validate(&self, label: &'static str) -> Result<(), ResponseMethodError> {
        validate_identifier(label, &self.id)?;
        if !is_canonical_sha256(&self.sha256) {
            return Err(ResponseMethodError::InvalidContentHash(label));
        }
        Ok(())
    }
}

/// Stable semantic and contributor profile for the four physical components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDefinitionProfile {
    pub schema_version: String,
    pub id: String,
    pub spatial_model: SpatialDoseModel,
    pub source_normalization: SourceNormalization,
    pub neutron_response: NeutronResponseSemantics,
    pub components: Vec<ComponentRule>,
    pub physical_total: PhysicalTotalEstimator,
}

impl ComponentDefinitionProfile {
    pub fn validate(&self) -> Result<(), ResponseMethodError> {
        validate_identifier("component_profile.schema_version", &self.schema_version)?;
        validate_identifier("component_profile.id", &self.id)?;

        let mut observed = BTreeSet::new();
        for rule in &self.components {
            if !observed.insert(rule.component) {
                return Err(ResponseMethodError::DuplicateComponent(rule.component));
            }
            validate_component_rule(rule)?;
        }
        for component in DoseComponent::REQUIRED {
            if !observed.contains(&component) {
                return Err(ResponseMethodError::MissingComponent(component));
            }
        }
        if self.components.len() != DoseComponent::REQUIRED.len() {
            return Err(ResponseMethodError::UnexpectedComponentCount(
                self.components.len(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpatialDoseModel {
    MacroscopicLocalChargedParticleKerma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNormalization {
    PerUnitWeightSourceNeutron,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeutronResponseSemantics {
    pub unit: ResponseUnit,
    pub interpolation: ResponseInterpolation,
    pub fold_normalization: FoldNormalization,
    pub outside_domain: OutsideDomainPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseUnit {
    GraySquareCentimeter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseInterpolation {
    LinearLinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldNormalization {
    DivideTrackLengthByScoringVolumeCm3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutsideDomainPolicy {
    RejectRun,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentRule {
    pub component: DoseComponent,
    pub estimator: ComponentEstimator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentEstimator {
    NjoyPartialKermaFluenceFold {
        nuclide: String,
        reaction_mt: u16,
        heatr_partial_kerma_mt: u16,
        photon_energy: PhotonEnergyTreatment,
    },
    ResidualNeutronKermaFluenceFold {
        heatr_total_kerma_mt: u16,
        subtract_components: Vec<DoseComponent>,
    },
    CoupledPhotonHeating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotonEnergyTreatment {
    ExcludedAndTransported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalTotalEstimator {
    CoupledHeatingWithoutParticleFilter,
}

/// Frozen recipe for generating a material-specific neutron response set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseGenerationMethod {
    pub schema_version: String,
    pub id: String,
    pub qualification: MethodQualification,
    pub component_profile: ContentHashReference,
    pub material: ContentHashReference,
    pub evaluated_data_release: String,
    pub processor: ToolIdentity,
    pub temperature_k: f64,
    pub reconstruction_tolerance_fraction: f64,
    pub heatr: HeatrMethod,
    pub atom_density_basis: AtomDensityBasis,
    pub grid_policy: GridPolicy,
    pub response_unit: ResponseUnit,
    pub interpolation: ResponseInterpolation,
    pub joule_per_ev: f64,
}

impl ResponseGenerationMethod {
    pub fn validate(&self) -> Result<(), ResponseMethodError> {
        validate_identifier("response_method.schema_version", &self.schema_version)?;
        validate_identifier("response_method.id", &self.id)?;
        validate_identifier(
            "response_method.evaluated_data_release",
            &self.evaluated_data_release,
        )?;
        self.component_profile.validate("component_profile")?;
        self.material.validate("material")?;
        self.processor.validate()?;
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(ResponseMethodError::InvalidTemperature);
        }
        if !self.reconstruction_tolerance_fraction.is_finite()
            || self.reconstruction_tolerance_fraction <= 0.0
            || self.reconstruction_tolerance_fraction > 1.0e-3
        {
            return Err(ResponseMethodError::InvalidReconstructionTolerance);
        }
        self.heatr.validate()?;
        if !self.joule_per_ev.is_finite()
            || (self.joule_per_ev - JOULE_PER_EV).abs() > JOULE_PER_EV * f64::EPSILON
        {
            return Err(ResponseMethodError::InvalidElectronVoltConversion);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodQualification {
    MethodFrozenTablesPending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    pub name: String,
    pub version: String,
    pub source_commit: String,
}

impl ToolIdentity {
    fn validate(&self) -> Result<(), ResponseMethodError> {
        validate_identifier("processor.name", &self.name)?;
        validate_identifier("processor.version", &self.version)?;
        if self.source_commit.len() != 40
            || !self
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ResponseMethodError::InvalidSourceCommit);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatrMethod {
    pub local_photon_deposition: bool,
    pub kinematic_checks: bool,
    pub allow_q_value_overrides: bool,
    pub total_kerma_mt: u16,
    pub kinematic_total_mt: u16,
    pub partials: Vec<PartialKermaChannel>,
    pub residual_component: DoseComponent,
    pub residual_subtract_components: Vec<DoseComponent>,
}

impl HeatrMethod {
    fn validate(&self) -> Result<(), ResponseMethodError> {
        if self.local_photon_deposition {
            return Err(ResponseMethodError::LocalPhotonDoubleCounting);
        }
        if !self.kinematic_checks {
            return Err(ResponseMethodError::KinematicChecksDisabled);
        }
        if self.allow_q_value_overrides {
            return Err(ResponseMethodError::QValueOverridesEnabled);
        }
        if self.total_kerma_mt != 301 || self.kinematic_total_mt != 443 {
            return Err(ResponseMethodError::InvalidHeatrTotalMt);
        }

        let mut components = BTreeSet::new();
        for partial in &self.partials {
            if !components.insert(partial.component) {
                return Err(ResponseMethodError::DuplicatePartialComponent(
                    partial.component,
                ));
            }
            if u32::from(partial.heatr_partial_kerma_mt) != u32::from(partial.reaction_mt) + 300 {
                return Err(ResponseMethodError::InvalidPartialKermaMt {
                    reaction_mt: partial.reaction_mt,
                    partial_mt: partial.heatr_partial_kerma_mt,
                });
            }
            match partial.component {
                DoseComponent::Boron
                    if partial.nuclide == "B10"
                        && partial.reaction_mt == 107
                        && partial.heatr_partial_kerma_mt == 407 => {}
                DoseComponent::Nitrogen
                    if partial.nuclide == "N14"
                        && partial.reaction_mt == 103
                        && partial.heatr_partial_kerma_mt == 403 => {}
                _ => {
                    return Err(ResponseMethodError::InvalidPartialChannel(
                        partial.component,
                    ));
                }
            }
        }
        if components != BTreeSet::from([DoseComponent::Boron, DoseComponent::Nitrogen])
            || self.residual_component != DoseComponent::Hydrogen
            || self
                .residual_subtract_components
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != components
            || self.residual_subtract_components.len() != components.len()
        {
            return Err(ResponseMethodError::InvalidResidualClassification);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialKermaChannel {
    pub component: DoseComponent,
    pub nuclide: String,
    pub reaction_mt: u16,
    pub heatr_partial_kerma_mt: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomDensityBasis {
    OpenMcHdf5AtomicWeightRatios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridPolicy {
    FullPointwiseUnionWithoutDownsampling,
}

fn validate_component_rule(rule: &ComponentRule) -> Result<(), ResponseMethodError> {
    match (rule.component, &rule.estimator) {
        (
            DoseComponent::Boron,
            ComponentEstimator::NjoyPartialKermaFluenceFold {
                nuclide,
                reaction_mt: 107,
                heatr_partial_kerma_mt: 407,
                ..
            },
        ) if nuclide == "B10" => Ok(()),
        (
            DoseComponent::Nitrogen,
            ComponentEstimator::NjoyPartialKermaFluenceFold {
                nuclide,
                reaction_mt: 103,
                heatr_partial_kerma_mt: 403,
                ..
            },
        ) if nuclide == "N14" => Ok(()),
        (
            DoseComponent::Hydrogen,
            ComponentEstimator::ResidualNeutronKermaFluenceFold {
                heatr_total_kerma_mt: 301,
                subtract_components,
            },
        ) if subtract_components.len() == 2
            && subtract_components.iter().copied().collect::<BTreeSet<_>>()
                == BTreeSet::from([DoseComponent::Boron, DoseComponent::Nitrogen]) =>
        {
            Ok(())
        }
        (DoseComponent::Photon, ComponentEstimator::CoupledPhotonHeating) => Ok(()),
        (component, _) => Err(ResponseMethodError::InvalidComponentRule(component)),
    }
}

fn validate_identifier(label: &'static str, value: &str) -> Result<(), ResponseMethodError> {
    if value.trim().is_empty() {
        Err(ResponseMethodError::EmptyIdentifier(label))
    } else {
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
pub enum ResponseMethodError {
    #[error("required identifier {0} is empty")]
    EmptyIdentifier(&'static str),
    #[error("{0} must have a canonical lowercase SHA-256 digest")]
    InvalidContentHash(&'static str),
    #[error("component {0:?} occurs more than once")]
    DuplicateComponent(DoseComponent),
    #[error("component {0:?} is missing")]
    MissingComponent(DoseComponent),
    #[error("component profile contains {0} rules; expected exactly four")]
    UnexpectedComponentCount(usize),
    #[error("component {0:?} has an estimator inconsistent with the canonical profile")]
    InvalidComponentRule(DoseComponent),
    #[error("processor source commit must be 40 lowercase hexadecimal characters")]
    InvalidSourceCommit,
    #[error("response-generation temperature must be finite and greater than zero kelvin")]
    InvalidTemperature,
    #[error("NJOY reconstruction tolerance must be in (0, 1e-3]")]
    InvalidReconstructionTolerance,
    #[error("local photon deposition would double count energy in coupled transport")]
    LocalPhotonDoubleCounting,
    #[error("NJOY HEATR kinematic consistency checks must be enabled")]
    KinematicChecksDisabled,
    #[error("unreviewed Q-value overrides are forbidden")]
    QValueOverridesEnabled,
    #[error("NJOY HEATR total and kinematic-total MT values must be 301 and 443")]
    InvalidHeatrTotalMt,
    #[error("partial KERMA channel for {0:?} occurs more than once")]
    DuplicatePartialComponent(DoseComponent),
    #[error("partial KERMA channel for {0:?} does not match the canonical profile")]
    InvalidPartialChannel(DoseComponent),
    #[error("partial KERMA MT {partial_mt} is not reaction MT {reaction_mt} plus 300")]
    InvalidPartialKermaMt { reaction_mt: u16, partial_mt: u16 },
    #[error("residual neutron KERMA classification is inconsistent")]
    InvalidResidualClassification,
    #[error("electronvolt-to-joule conversion must use the exact SI constant")]
    InvalidElectronVoltConversion,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const PROFILE_JSON: &str =
        include_str!("../../../benchmarks/synthetic/nf-bnct-001/transport/component-profile.json");
    const MATERIAL_JSON: &str =
        include_str!("../../../benchmarks/synthetic/nf-bnct-001/transport/material.json");
    const METHOD_JSON: &str = include_str!(
        "../../../benchmarks/synthetic/nf-bnct-001/transport/response-generation-method.json"
    );

    fn profile() -> ComponentDefinitionProfile {
        serde_json::from_str(PROFILE_JSON).unwrap()
    }

    fn method() -> ResponseGenerationMethod {
        serde_json::from_str(METHOD_JSON).unwrap()
    }

    fn sha256(content: &str) -> String {
        format!("{:x}", Sha256::digest(content.as_bytes()))
    }

    #[test]
    fn frozen_component_profile_is_valid() {
        let profile = profile();
        assert_eq!(profile.id, "nctforge.macroscopic-absorbed-dose.v1");
        assert_eq!(profile.validate(), Ok(()));
    }

    #[test]
    fn frozen_generation_method_is_valid() {
        let method = method();
        assert_eq!(method.id, "nctforge.nf-bnct-001.response-generation.v1");
        assert_eq!(method.component_profile.sha256, sha256(PROFILE_JSON));
        assert_eq!(method.material.sha256, sha256(MATERIAL_JSON));
        assert_eq!(method.validate(), Ok(()));
    }

    #[test]
    fn rejects_partial_kerma_that_is_not_mt_plus_300() {
        let mut method = method();
        method.heatr.partials[0].heatr_partial_kerma_mt = 107;
        assert_eq!(
            method.validate(),
            Err(ResponseMethodError::InvalidPartialKermaMt {
                reaction_mt: 107,
                partial_mt: 107,
            })
        );
    }

    #[test]
    fn rejects_local_photon_double_counting() {
        let mut method = method();
        method.heatr.local_photon_deposition = true;
        assert_eq!(
            method.validate(),
            Err(ResponseMethodError::LocalPhotonDoubleCounting)
        );
    }
}
