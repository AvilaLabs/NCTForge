// SPDX-License-Identifier: Apache-2.0

//! Biological dose interpretation layer.
//!
//! A `BiologicalModel` is a separately versioned research artifact that
//! assigns dimensionless effectiveness weights to the four physical dose
//! components, optionally overridden per named region. Applying it to a
//! `PhysicalDoseBundle` produces a `BiologicalDoseBundle` whose weighted
//! values never alias physical dose: the bundle carries its own schema, unit
//! label, and qualification boundary, and the physical bundle remains
//! separately inspectable.
//!
//! This is a research-only modeling layer. It does not assert clinical CBE,
//! RBE, or Gy-Eq values for any real treatment, and the benchmark
//! specification's exclusion of weighted dose from NF-BNCT-001 is preserved:
//! biological bundles are produced only when a model artifact is supplied.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use nctforge_core::{ContentReference, DoseComponent, DoseUnit, GridGeometry, PhysicalDoseBundle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const BIOLOGICAL_MODEL_SCHEMA: &str = "nctforge.biological-model/0.1.0";
pub const BIOLOGICAL_DOSE_BUNDLE_SCHEMA: &str = "nctforge.biological-dose-bundle/0.1.0";

/// Dimensionless effectiveness weight applied to one physical component.
pub type WeightMap = BTreeMap<String, f64>;

/// Weight semantics currently supported. `fixed_per_component` applies one
/// constant weight per dose component; per-region overrides replace the
/// default weight inside a named region mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightSemantics {
    FixedPerComponent,
}

/// A separately versioned biological model artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiologicalModel {
    pub schema_version: String,
    pub id: String,
    pub weight_semantics: WeightSemantics,
    /// The physical-bundle unit this model consumes.
    pub input_unit: DoseUnit,
    /// Dimensionless weight per dose component (`boron`, `nitrogen`,
    /// `hydrogen`, `photon`).
    pub component_weights: WeightMap,
    /// Optional per-region weight overrides keyed by region name. Region
    /// masks are supplied at application time; a region listed here without a
    /// matching mask is an error.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub region_weights: BTreeMap<String, WeightMap>,
    /// Evidence reference for where the weight values came from.
    pub derivation: Option<ContentReference>,
}

impl BiologicalModel {
    pub fn validate(&self) -> Result<(), BioError> {
        if self.schema_version != BIOLOGICAL_MODEL_SCHEMA {
            return Err(BioError::UnsupportedSchema(self.schema_version.clone()));
        }
        if self.id.trim().is_empty() {
            return Err(BioError::Invalid("model id is empty".into()));
        }
        let check = |weights: &WeightMap, label: &str| -> Result<(), BioError> {
            let required: BTreeSet<&str> = DoseComponent::REQUIRED
                .iter()
                .map(|component| component_name(*component))
                .collect();
            let present: BTreeSet<&str> = weights.keys().map(String::as_str).collect();
            if present != required {
                return Err(BioError::Invalid(format!(
                    "{label} must define exactly the four dose components {required:?}; observed {present:?}"
                )));
            }
            for (name, weight) in weights {
                if !weight.is_finite() || *weight < 0.0 {
                    return Err(BioError::Invalid(format!(
                        "{label} weight for {name} must be finite and non-negative"
                    )));
                }
            }
            Ok(())
        };
        check(&self.component_weights, "component_weights")?;
        for (region, weights) in &self.region_weights {
            if region.trim().is_empty() {
                return Err(BioError::Invalid("region name is empty".into()));
            }
            check(weights, "region_weights")?;
        }
        if let Some(derivation) = &self.derivation {
            derivation
                .validate()
                .map_err(|_| BioError::Invalid("derivation reference is invalid".into()))?;
        }
        Ok(())
    }

    fn weight_for(&self, component: DoseComponent, region: Option<&str>) -> f64 {
        let table = region
            .and_then(|name| self.region_weights.get(name))
            .unwrap_or(&self.component_weights);
        *table
            .get(component_name(component))
            .expect("validated weights cover every component")
    }
}

fn component_name(component: DoseComponent) -> &'static str {
    match component {
        DoseComponent::Boron => "boron",
        DoseComponent::Nitrogen => "nitrogen",
        DoseComponent::Hydrogen => "hydrogen",
        DoseComponent::Photon => "photon",
    }
}

/// Named voxel mask in the bundle's grid order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionMask {
    pub name: String,
    pub voxels: Vec<bool>,
}

/// One component's biologically weighted dose volume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedDoseVolume {
    pub component: DoseComponent,
    /// `weighted_gray_per_source_particle` or `weighted_gray`, mirroring the
    /// input bundle's unit.
    pub unit: String,
    pub values: Vec<f64>,
    pub absolute_standard_uncertainty: Option<Vec<f64>>,
}

/// How the biological total's uncertainty was formed. Component
/// uncertainties share transport histories, so the reported total sigma is
/// the fully-correlated linear sum — a conservative upper bound that does not
/// pretend the components are independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiologicalUncertaintyMethod {
    CorrelatedComponentSum,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiologicalTotal {
    pub unit: String,
    pub values: Vec<f64>,
    pub absolute_standard_uncertainty: Option<Vec<f64>>,
    pub uncertainty_method: BiologicalUncertaintyMethod,
}

/// The biological interpretation of one physical dose bundle under one model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiologicalDoseBundle {
    pub schema_version: String,
    pub case_id: String,
    pub geometry: GridGeometry,
    /// `provenance_id` of the physical bundle this was derived from.
    pub physical_bundle_provenance: String,
    /// Content binding of the exact model JSON that was applied.
    pub model: ContentReference,
    pub weight_semantics: WeightSemantics,
    /// Unit label of the weighted values — deliberately never `gray`.
    pub unit: String,
    pub components: Vec<WeightedDoseVolume>,
    pub total: BiologicalTotal,
    /// Region names whose override weights were actually applied.
    pub regions_applied: Vec<String>,
    pub qualification: String,
}

impl BiologicalDoseBundle {
    pub fn validate(&self) -> Result<(), BioError> {
        if self.schema_version != BIOLOGICAL_DOSE_BUNDLE_SCHEMA {
            return Err(BioError::UnsupportedSchema(self.schema_version.clone()));
        }
        if self.case_id.trim().is_empty() {
            return Err(BioError::Invalid("case_id is empty".into()));
        }
        self.model
            .validate()
            .map_err(|_| BioError::Invalid("model reference is invalid".into()))?;
        let voxel_count = self
            .geometry
            .voxel_count()
            .map_err(|e| BioError::Invalid(format!("geometry: {e}")))?;
        let mut observed = BTreeSet::new();
        for volume in &self.components {
            if !observed.insert(volume.component) {
                return Err(BioError::Invalid(format!(
                    "duplicate component {:?}",
                    volume.component
                )));
            }
            if volume.values.len() != voxel_count
                || volume.values.iter().any(|v| !v.is_finite() || *v < 0.0)
            {
                return Err(BioError::Invalid(format!(
                    "component {:?} values are not finite non-negative voxel values",
                    volume.component
                )));
            }
            if let Some(sigma) = &volume.absolute_standard_uncertainty
                && (sigma.len() != voxel_count || sigma.iter().any(|v| !v.is_finite() || *v < 0.0))
            {
                return Err(BioError::Invalid(format!(
                    "component {:?} uncertainty is malformed",
                    volume.component
                )));
            }
        }
        for required in DoseComponent::REQUIRED {
            if !observed.contains(&required) {
                return Err(BioError::Invalid(format!(
                    "component {required:?} is missing"
                )));
            }
        }
        if self.total.values.len() != voxel_count {
            return Err(BioError::Invalid("total dose length mismatch".into()));
        }
        Ok(())
    }
}

fn weighted_unit(unit: DoseUnit) -> String {
    match unit {
        DoseUnit::GrayPerSourceParticle => "weighted_gray_per_source_particle".to_string(),
        DoseUnit::Gray => "weighted_gray".to_string(),
    }
}

/// Apply a biological model to a physical dose bundle.
///
/// `regions` supplies the masks named by `model.region_weights`; a model
/// region without a matching mask is rejected, as is a mask that does not
/// cover the bundle grid. When a voxel belongs to several named regions the
/// first matching region in `model.region_weights` order applies — call this
/// out explicitly because overlapping ROI masks are a real possibility.
pub fn apply_biological_model(
    model: &BiologicalModel,
    model_bytes: &[u8],
    physical: &PhysicalDoseBundle,
    regions: &[RegionMask],
) -> Result<BiologicalDoseBundle, BioError> {
    model.validate()?;
    physical
        .validate()
        .map_err(|e| BioError::Invalid(format!("physical bundle: {e}")))?;
    let voxel_count = physical
        .geometry
        .voxel_count()
        .map_err(|e| BioError::Invalid(format!("geometry: {e}")))?;
    if physical
        .components
        .first()
        .is_some_and(|c| c.unit != model.input_unit)
    {
        return Err(BioError::Invalid(format!(
            "model input_unit {:?} does not match the physical bundle",
            model.input_unit
        )));
    }

    // Region-name -> voxel lookup; reject model regions without masks and
    // masks that do not cover this grid.
    let masks: BTreeMap<&str, &Vec<bool>> = regions
        .iter()
        .map(|mask| (mask.name.as_str(), &mask.voxels))
        .collect();
    for name in model.region_weights.keys() {
        let mask = masks.get(name.as_str()).ok_or_else(|| {
            BioError::Invalid(format!("model region {name} has no supplied mask"))
        })?;
        if mask.len() != voxel_count {
            return Err(BioError::Invalid(format!(
                "region mask {name} covers {} voxels, grid needs {voxel_count}",
                mask.len()
            )));
        }
    }

    // Per-voxel effective region: first matching mask in region_weights order.
    let region_of = |voxel: usize| -> Option<&str> {
        model
            .region_weights
            .keys()
            .find(|name| masks[name.as_str()][voxel])
            .map(String::as_str)
    };

    let unit = weighted_unit(model.input_unit);
    let mut components = Vec::new();
    for volume in &physical.components {
        let mut values = Vec::with_capacity(voxel_count);
        let mut sigmas = volume
            .absolute_standard_uncertainty
            .as_ref()
            .map(|_| Vec::with_capacity(voxel_count));
        for voxel in 0..voxel_count {
            let weight = model.weight_for(volume.component, region_of(voxel));
            values.push(volume.values[voxel] * weight);
            if let (Some(sigmas), Some(source)) = (
                sigmas.as_mut(),
                volume.absolute_standard_uncertainty.as_ref(),
            ) {
                sigmas.push(source[voxel] * weight);
            }
        }
        components.push(WeightedDoseVolume {
            component: volume.component,
            unit: unit.clone(),
            values,
            absolute_standard_uncertainty: sigmas,
        });
    }

    // Biological total: sum of weighted components; uncertainty is the
    // fully-correlated linear sum of component sigmas (conservative, since
    // components share transport histories).
    let mut total_values = vec![0.0; voxel_count];
    let mut have_sigma = true;
    let mut total_sigma = vec![0.0; voxel_count];
    for component in &components {
        for voxel in 0..voxel_count {
            total_values[voxel] += component.values[voxel];
            if let Some(sigma) = &component.absolute_standard_uncertainty {
                total_sigma[voxel] += sigma[voxel];
            } else {
                have_sigma = false;
            }
        }
    }

    let mut regions_applied: Vec<String> = model.region_weights.keys().cloned().collect();
    regions_applied.retain(|name| masks.contains_key(name.as_str()));
    let bundle = BiologicalDoseBundle {
        schema_version: BIOLOGICAL_DOSE_BUNDLE_SCHEMA.into(),
        case_id: physical.case_id.clone(),
        geometry: physical.geometry.clone(),
        physical_bundle_provenance: physical.provenance_id.clone(),
        model: ContentReference {
            id: model.id.clone(),
            sha256: format!("{:x}", Sha256::digest(model_bytes)),
        },
        weight_semantics: model.weight_semantics,
        unit,
        components,
        total: BiologicalTotal {
            unit: weighted_unit(model.input_unit),
            values: total_values,
            absolute_standard_uncertainty: have_sigma.then_some(total_sigma),
            uncertainty_method: if have_sigma {
                BiologicalUncertaintyMethod::CorrelatedComponentSum
            } else {
                BiologicalUncertaintyMethod::Unavailable
            },
        },
        regions_applied,
        qualification: "synthetic_research_only_not_clinical".into(),
    };
    bundle.validate()?;
    Ok(bundle)
}

#[derive(Debug, Error)]
pub enum BioError {
    #[error("unsupported biological-model schema {0:?}")]
    UnsupportedSchema(String),
    #[error("invalid biological artifact: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use nctforge_core::{DoseVolume, PhysicalTotalDoseVolume, TotalUncertaintyMethod};

    fn geometry() -> GridGeometry {
        GridGeometry {
            shape: [2, 1, 1],
            spacing_mm: [5.0; 3],
            origin_mm: [-2.5; 3],
            direction: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    fn physical_bundle() -> PhysicalDoseBundle {
        let reference = |id: &str| ContentReference {
            id: id.into(),
            sha256: "a".repeat(64),
        };
        let components = [
            (DoseComponent::Boron, 1.0e-12, 1.0e-14),
            (DoseComponent::Nitrogen, 2.0e-13, 2.0e-15),
            (DoseComponent::Hydrogen, 5.0e-14, 5.0e-16),
            (DoseComponent::Photon, 3.0e-13, 3.0e-15),
        ]
        .into_iter()
        .map(|(component, mean, sigma)| DoseVolume {
            component,
            unit: DoseUnit::GrayPerSourceParticle,
            values: vec![mean, mean],
            absolute_standard_uncertainty: Some(vec![sigma, sigma]),
        })
        .collect();
        PhysicalDoseBundle {
            schema_version: nctforge_core::PHYSICAL_DOSE_BUNDLE_SCHEMA.into(),
            case_id: "synthetic-case".into(),
            frame_of_reference_uid: None,
            geometry: geometry(),
            component_profile: reference("profile"),
            response_set: reference("response"),
            components,
            physical_total: PhysicalTotalDoseVolume {
                unit: DoseUnit::GrayPerSourceParticle,
                values: vec![1.75e-12, 1.75e-12],
                absolute_standard_uncertainty: Some(vec![1.1e-14, 1.1e-14]),
                uncertainty_method: TotalUncertaintyMethod::DedicatedEstimator,
            },
            provenance_id: "test-provenance".into(),
        }
    }

    fn model() -> BiologicalModel {
        let mut weights = WeightMap::new();
        weights.insert("boron".into(), 3.8);
        weights.insert("nitrogen".into(), 2.5);
        weights.insert("hydrogen".into(), 1.0);
        weights.insert("photon".into(), 1.0);
        BiologicalModel {
            schema_version: BIOLOGICAL_MODEL_SCHEMA.into(),
            id: "test.model.v1".into(),
            weight_semantics: WeightSemantics::FixedPerComponent,
            input_unit: DoseUnit::GrayPerSourceParticle,
            component_weights: weights,
            region_weights: BTreeMap::new(),
            derivation: None,
        }
    }

    #[test]
    fn applies_component_weights_and_correlated_total() {
        let model = model();
        let bytes = serde_json::to_vec_pretty(&model).unwrap();
        let bundle = apply_biological_model(&model, &bytes, &physical_bundle(), &[]).unwrap();
        assert_eq!(bundle.schema_version, BIOLOGICAL_DOSE_BUNDLE_SCHEMA);
        assert_eq!(bundle.unit, "weighted_gray_per_source_particle");
        let boron = bundle
            .components
            .iter()
            .find(|c| c.component == DoseComponent::Boron)
            .unwrap();
        assert_eq!(boron.values[0], 3.8e-12);
        // Total = sum of weighted components; sigma = correlated linear sum.
        let expected = 3.8e-12 + 2.5 * 2.0e-13 + 1.0 * 5.0e-14 + 1.0 * 3.0e-13;
        assert!((bundle.total.values[0] - expected).abs() / expected < 1.0e-12);
        let sigma = 3.8e-14 + 2.5 * 2.0e-15 + 5.0e-16 + 3.0e-15;
        let total_sigma = bundle.total.absolute_standard_uncertainty.as_ref().unwrap()[0];
        assert!((total_sigma - sigma).abs() / sigma < 1.0e-12);
        assert_eq!(
            bundle.total.uncertainty_method,
            BiologicalUncertaintyMethod::CorrelatedComponentSum
        );
        assert_eq!(bundle.model.sha256, format!("{:x}", Sha256::digest(&bytes)));
    }

    #[test]
    fn region_weights_override_defaults_inside_the_mask() {
        let mut model = model();
        let mut tumor = WeightMap::new();
        tumor.insert("boron".into(), 5.0);
        tumor.insert("nitrogen".into(), 2.5);
        tumor.insert("hydrogen".into(), 1.0);
        tumor.insert("photon".into(), 1.0);
        model.region_weights.insert("tumor".into(), tumor);
        let bytes = serde_json::to_vec_pretty(&model).unwrap();
        let mask = RegionMask {
            name: "tumor".into(),
            voxels: vec![true, false],
        };
        let bundle = apply_biological_model(&model, &bytes, &physical_bundle(), &[mask]).unwrap();
        let boron = bundle
            .components
            .iter()
            .find(|c| c.component == DoseComponent::Boron)
            .unwrap();
        assert_eq!(boron.values, vec![5.0e-12, 3.8e-12]);
        assert_eq!(bundle.regions_applied, vec!["tumor"]);
    }

    #[test]
    fn rejects_missing_region_mask_and_bad_weights() {
        let mut regioned = model();
        regioned
            .region_weights
            .insert("tumor".into(), regioned.component_weights.clone());
        let bytes = serde_json::to_vec_pretty(&regioned).unwrap();
        assert!(apply_biological_model(&regioned, &bytes, &physical_bundle(), &[]).is_err());

        let mut bad = model();
        bad.component_weights.insert("boron".into(), f64::NAN);
        assert!(bad.validate().is_err());
        let mut incomplete = model();
        incomplete.component_weights.remove("photon");
        assert!(incomplete.validate().is_err());
    }

    #[test]
    fn weighted_bundle_never_aliases_physical_unit() {
        let model = model();
        let bytes = serde_json::to_vec_pretty(&model).unwrap();
        let bundle = apply_biological_model(&model, &bytes, &physical_bundle(), &[]).unwrap();
        assert_ne!(bundle.unit, "gray");
        assert_ne!(bundle.unit, "gray_per_source_particle");
        assert_eq!(bundle.qualification, "synthetic_research_only_not_clinical");
        assert_eq!(bundle.physical_bundle_provenance, "test-provenance");
    }
}
