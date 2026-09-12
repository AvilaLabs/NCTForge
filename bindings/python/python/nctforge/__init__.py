"""NCTForge — transport-neutral BNCT research verification workbench.

Research software only: not a medical device, not commissioned for any
facility, and not a dose calculator. All validation, geometry, contract, and
evidence behavior below is executed by the authoritative Rust implementation;
this package adds no scientific logic of its own.

Transport actions remain unavailable until the Rust capability and evidence
gates pass; :func:`backends` reports those flags honestly.
"""

from nctforge._nctforge import (
    AppliedFractionation,
    Artifact,
    Backend,
    BiologicalDoseBundle,
    BiologicalModel,
    CaseManifest,
    CaseVerification,
    ComponentProfile,
    DoseVolume,
    DoseVolumeHistogram,
    FixedSource,
    GeneratedCase,
    Geometry,
    Material,
    NctForgeError,
    PhysicalDoseBundle,
    ResponseGenerationMethod,
    ResponseSet,
    Structure,
    VerifiedCase,
    apply_model,
    backends,
    collect_run,
    compute_dvh,
    compute_dvh_biological,
    file_sha256,
    generate_case,
    load_biological_model,
    load_case,
    load_component_profile,
    load_fixed_source,
    load_material,
    load_physical_dose_bundle,
    load_response_generation_method,
    load_response_set,
    read_manifest,
    verify_case,
    verify_evidence_bundle,
)
from nctforge._nctforge import __version__ as _extension_version

__version__ = _extension_version

__all__ = [
    "AppliedFractionation",
    "Artifact",
    "Backend",
    "BiologicalDoseBundle",
    "BiologicalModel",
    "CaseManifest",
    "CaseVerification",
    "ComponentProfile",
    "DoseVolume",
    "DoseVolumeHistogram",
    "FixedSource",
    "GeneratedCase",
    "Geometry",
    "Material",
    "NctForgeError",
    "PhysicalDoseBundle",
    "ResponseGenerationMethod",
    "ResponseSet",
    "Structure",
    "VerifiedCase",
    "__version__",
    "apply_model",
    "backends",
    "collect_run",
    "compute_dvh",
    "compute_dvh_biological",
    "file_sha256",
    "generate_case",
    "load_biological_model",
    "load_case",
    "load_component_profile",
    "load_fixed_source",
    "load_material",
    "load_physical_dose_bundle",
    "load_response_generation_method",
    "load_response_set",
    "read_manifest",
    "verify_case",
    "verify_evidence_bundle",
]
