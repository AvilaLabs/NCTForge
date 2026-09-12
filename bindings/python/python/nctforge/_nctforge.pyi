"""Checked type surface for the compiled ``nctforge._nctforge`` extension.

Every object is immutable. Argument and return shapes mirror the versioned
Rust contracts; ``to_json`` methods return the canonical pretty-printed JSON
serialization produced by the Rust ``serde`` implementation.
"""

from os import PathLike

__version__: str

class NctForgeError(Exception):
    """An NCTForge contract, verification, or evidence check failed."""

class Backend:
    """Transport-backend descriptor with current capability flags."""

    @property
    def id(self) -> str: ...
    @property
    def display_name(self) -> str: ...
    @property
    def version(self) -> str | None: ...
    @property
    def can_prepare(self) -> bool: ...
    @property
    def can_execute(self) -> bool: ...
    @property
    def can_import(self) -> bool: ...

class Structure:
    """Verified ROI summary for a structure in the frozen benchmark case."""

    @property
    def number(self) -> int: ...
    @property
    def name(self) -> str: ...
    @property
    def voxel_count(self) -> int: ...
    @property
    def volume_cm3(self) -> float: ...
    @property
    def centroid_lps_mm(self) -> tuple[float, float, float]: ...

class CaseVerification:
    """Result of the independent ``NF-BNCT-001`` verification oracle."""

    @property
    def case_id(self) -> str: ...
    @property
    def shape(self) -> tuple[int, int, int]:
        """[columns, rows, slices]."""
    @property
    def spacing_mm(self) -> tuple[float, float, float]: ...
    @property
    def origin_mm(self) -> tuple[float, float, float]:
        """LPS position of the centre of voxel [0, 0, 0]."""
    @property
    def ct_slice_count(self) -> int: ...
    @property
    def verified_artifact_count(self) -> int: ...
    @property
    def structures(self) -> list[Structure]: ...

class Geometry:
    """Validated CT lattice geometry in the DICOM LPS patient frame."""

    @property
    def shape(self) -> tuple[int, int, int]: ...
    @property
    def spacing_mm(self) -> tuple[float, float, float]: ...
    @property
    def origin_mm(self) -> tuple[float, float, float]: ...
    @property
    def direction(self) -> tuple[float, ...]: ...
    @property
    def voxel_count(self) -> int: ...
    def voxel_center_lps_mm(self, column: int, row: int, slice: int) -> tuple[float, float, float]: ...

class VerifiedCase:
    """A ``NF-BNCT-001`` case loaded only after every verification gate passed."""

    @property
    def report(self) -> CaseVerification: ...
    @property
    def geometry(self) -> Geometry: ...
    @property
    def frame_of_reference_uid(self) -> str: ...
    @property
    def structures(self) -> list[Structure]: ...
    def ct_value(self, column: int, row: int, slice: int) -> float:
        """Modality value at voxel [column, row, slice], after rescale."""
    def structure_mask(self, name: str) -> list[bool]:
        """Boolean mask for the named structure, columns fastest then rows, slices."""

class GeneratedCase:
    """Summary of a generated ``NF-BNCT-001`` case directory."""

    @property
    def root(self) -> str: ...
    @property
    def ct_file_count(self) -> int: ...
    @property
    def rtstruct_file(self) -> str: ...
    @property
    def manifest_file(self) -> str: ...

class Artifact:
    """One artifact binding inside a verified case manifest."""

    @property
    def role(self) -> str: ...
    @property
    def path(self) -> str: ...
    @property
    def sha256(self) -> str: ...
    @property
    def media_type(self) -> str | None: ...

class CaseManifest:
    """A parsed and schema-validated ``case.json`` manifest."""

    @property
    def schema_version(self) -> str: ...
    @property
    def case_id(self) -> str: ...
    @property
    def qualification(self) -> str: ...
    @property
    def coordinate_system(self) -> str: ...
    @property
    def frame_of_reference_uid(self) -> str: ...
    @property
    def material_model_id(self) -> str: ...
    @property
    def source_model_id(self) -> str: ...
    @property
    def geometry(self) -> Geometry: ...
    @property
    def structures(self) -> list[Structure]: ...
    @property
    def artifacts(self) -> list[Artifact]: ...
    def to_json(self) -> str:
        """Canonical JSON serialization produced by the Rust contract."""
    def verify_artifacts(self, case_root: str | PathLike[str]) -> int:
        """Re-verify every bound artifact; returns the verified count."""

class Material:
    """A validated explicit-nuclide material contract."""

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    def to_json(self) -> str: ...

class FixedSource:
    """A validated backend-neutral fixed-source contract."""

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    def to_json(self) -> str: ...

class ComponentProfile:
    """A validated four-component dose-definition profile."""

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    def to_json(self) -> str: ...

class ResponseGenerationMethod:
    """A validated, versioned response-generation recipe."""

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    def to_json(self) -> str: ...

class ResponseSet:
    """A material-specific neutron response set after base validation.

    ``folding_ready`` additionally requires the independent-review gate used
    before dose folding.
    """

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    @property
    def qualification(self) -> str: ...
    @property
    def folding_ready(self) -> bool: ...
    @property
    def transport_energy_range_ev(self) -> tuple[float, float]: ...
    @property
    def energy_knot_count(self) -> int: ...
    def to_json(self) -> str: ...

def backends() -> list[Backend]:
    """Descriptors for every compiled-in transport backend."""

def file_sha256(path: str | PathLike[str]) -> str:
    """SHA-256 of a file's exact bytes, lowercase hex."""

def generate_case(destination: str | PathLike[str]) -> GeneratedCase:
    """Generate the deterministic synthetic case; refuses existing output."""

def verify_case(root: str | PathLike[str]) -> CaseVerification:
    """Verify a generated case against the independent frozen oracle."""

def load_case(root: str | PathLike[str]) -> VerifiedCase:
    """Load a case only after all geometry and artifact gates pass."""

def read_manifest(path: str | PathLike[str]) -> CaseManifest: ...
def load_material(path: str | PathLike[str]) -> Material: ...
def load_fixed_source(path: str | PathLike[str]) -> FixedSource: ...
def load_component_profile(path: str | PathLike[str]) -> ComponentProfile: ...
def load_response_generation_method(path: str | PathLike[str]) -> ResponseGenerationMethod: ...
def load_response_set(path: str | PathLike[str]) -> ResponseSet: ...

class DoseVolume:
    """One component's per-voxel dose values in grid order."""

    @property
    def component(self) -> str: ...
    @property
    def unit(self) -> str: ...
    @property
    def values(self) -> list[float]: ...
    @property
    def absolute_standard_uncertainty(self) -> list[float] | None: ...

class PhysicalDoseBundle:
    """A validated ``nctforge.physical-dose-bundle/0.2.0`` artifact."""

    @property
    def schema_version(self) -> str: ...
    @property
    def case_id(self) -> str: ...
    @property
    def geometry(self) -> Geometry: ...
    @property
    def components(self) -> list[DoseVolume]: ...
    @property
    def physical_total(self) -> DoseVolume:
        """Dedicated physical-total volume, separate from component sums."""
    @property
    def provenance_id(self) -> str: ...
    def to_json(self) -> str: ...

class BiologicalModel:
    """A validated ``nctforge.biological-model/0.1.0`` artifact."""

    @property
    def schema_version(self) -> str: ...
    @property
    def id(self) -> str: ...
    def to_json(self) -> str: ...

class BiologicalDoseBundle:
    """A validated biological dose bundle; never aliases physical dose."""

    @property
    def schema_version(self) -> str: ...
    @property
    def case_id(self) -> str: ...
    @property
    def unit(self) -> str:
        """Weighted unit label, deliberately never ``gray``."""
    @property
    def geometry(self) -> Geometry: ...
    @property
    def components(self) -> list[DoseVolume]: ...
    @property
    def biological_total(self) -> DoseVolume:
        """Weighted total with correlated component-sum uncertainty."""
    @property
    def physical_bundle_provenance(self) -> str: ...
    @property
    def regions_applied(self) -> list[str]: ...
    @property
    def qualification(self) -> str: ...
    def to_json(self) -> str: ...
    def write(self, output: str | PathLike[str]) -> None:
        """Write the bundle JSON; refuses to overwrite an existing file."""

class DoseVolumeHistogram:
    """A deterministic ``nctforge.dose-volume-histogram/0.1.0`` artifact."""

    @property
    def schema_version(self) -> str: ...
    @property
    def region(self) -> str: ...
    @property
    def quantity(self) -> str: ...
    @property
    def unit(self) -> str: ...
    @property
    def dose_edges(self) -> list[float]: ...
    @property
    def differential_volume_fraction(self) -> list[float]: ...
    @property
    def cumulative_volume_fraction(self) -> list[float]:
        """V(d): fraction of the region receiving at least each edge dose."""
    @property
    def region_voxel_count(self) -> int: ...
    @property
    def region_volume_mm3(self) -> float: ...
    def to_json(self) -> str: ...

def load_physical_dose_bundle(path: str | PathLike[str]) -> PhysicalDoseBundle: ...
def collect_run(working_directory: str | PathLike[str]) -> PhysicalDoseBundle:
    """Collect a completed OpenMC run directory into a dose bundle."""
def load_biological_model(path: str | PathLike[str]) -> BiologicalModel: ...
def apply_model(
    model: BiologicalModel,
    physical: PhysicalDoseBundle,
    region_masks: list[tuple[str, str | PathLike[str]]],
) -> BiologicalDoseBundle:
    """Apply a biological model; region_masks maps region names to mask JSON."""
def compute_dvh(
    physical: PhysicalDoseBundle,
    quantity: str,
    mask_name: str,
    mask_voxels: list[bool],
    bins: int,
) -> DoseVolumeHistogram:
    """Histogram ``component:NAME`` or ``physical_total`` over the mask."""
def compute_dvh_biological(
    bundle: BiologicalDoseBundle,
    quantity: str,
    mask_name: str,
    mask_voxels: list[bool],
    bins: int,
) -> DoseVolumeHistogram:
    """Histogram ``component:NAME`` or ``biological_total`` over the mask."""
def verify_evidence_bundle(root: str | PathLike[str]) -> tuple[str, int]:
    """Re-hash every manifest artifact; returns (case_id, artifact count)."""
