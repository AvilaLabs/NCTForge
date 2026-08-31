#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0

"""Create a case-scoped NCTForge manifest by inspecting OpenMC HDF5 data.

This script intentionally uses h5py directly rather than importing OpenMC. It
therefore checks the on-disk structures that the pinned OpenMC 0.16.0 C++
reader consumes and does not depend on a mutable Python API installation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
from typing import Any
import xml.etree.ElementTree as ET

try:
    import h5py
    import numpy
except ModuleNotFoundError:
    raise SystemExit(
        "error: install scripts/requirements-openmc-data-inspector.txt first"
    ) from None


OPENMC_VERSION = "0.16.0"
OPENMC_SOURCE_COMMIT = "617d35a5063c57796b43428bc401e627d2011046"
EVALUATED_DATA_RELEASE = "ENDF/B-VIII.1"
INSPECTION_METHOD = "nctforge-openmc-data-inspector/0.2.0"
HDF5_VERSION = [3, 0]
K_BOLTZMANN_EV_PER_K = 8.617333262e-5
NUCLIDE_PATTERN = re.compile(r"^([A-Z][a-z]?)\d+(?:_m\d+)?$")
PHOTON_REACTION_GROUPS = {
    "coherent": 502,
    "incoherent": 504,
    "pair_production_electron": 515,
    "pair_production_total": 516,
    "pair_production_nuclear": 517,
    "photoelectric": 522,
    "heating": 525,
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def decode_text(value: Any) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8")
    if hasattr(value, "item"):
        return decode_text(value.item())
    return str(value)


def data_relative_path(path: Path, data_root: Path) -> str:
    resolved = path.resolve(strict=True)
    try:
        relative = resolved.relative_to(data_root)
    except ValueError as error:
        raise ValueError(f"data file escapes data root: {resolved}") from error
    if not resolved.is_file():
        raise ValueError(f"data artifact is not a regular file: {resolved}")
    return relative.as_posix()


def artifact(path: Path, data_root: Path) -> dict[str, str]:
    return {
        "relative_path": data_relative_path(path, data_root),
        "sha256": sha256_file(path),
    }


def hdf5_version(handle: h5py.File, path: Path) -> list[int]:
    if "version" not in handle.attrs:
        raise ValueError(f"HDF5 data version is absent: {path}")
    version = [int(value) for value in handle.attrs["version"]]
    if version != HDF5_VERSION:
        raise ValueError(
            f"{path} uses HDF5 data version {version}; expected {HDF5_VERSION}"
        )
    return version


def expected_root_group(handle: h5py.File, expected: str, path: Path) -> h5py.Group:
    group = handle.get(expected)
    if not isinstance(group, h5py.Group):
        observed = [value.name for value in handle.values() if isinstance(value, h5py.Group)]
        raise ValueError(f"{path} lacks root group {expected!r}; observed {observed}")
    return group


def inspect_neutron(path: Path, nuclide: str, data_root: Path) -> dict[str, Any]:
    with h5py.File(path, "r") as handle:
        if decode_text(handle.attrs.get("filetype", "")) != "data_neutron":
            raise ValueError(f"not an OpenMC incident-neutron HDF5 file: {path}")
        version = hdf5_version(handle, path)
        group = expected_root_group(handle, nuclide, path)
        atomic_weight_ratio = float(group.attrs["atomic_weight_ratio"])
        temperature_grids = []
        for label, dataset in group["kTs"].items():
            if label not in group["energy"]:
                raise ValueError(
                    f"{path} lacks an energy grid for temperature {label!r}"
                )
            energies = numpy.asarray(group["energy"][label][()], dtype=float)
            if (
                energies.ndim != 1
                or len(energies) < 2
                or not numpy.all(numpy.isfinite(energies))
                or energies[0] < 0.0
                or not numpy.all(numpy.diff(energies) > 0.0)
            ):
                raise ValueError(
                    f"{path} has an invalid neutron energy grid for {label!r}"
                )
            temperature_grids.append(
                (
                    float(dataset[()]) / K_BOLTZMANN_EV_PER_K,
                    [float(energies[0]), float(energies[-1])],
                )
            )
        temperature_grids.sort(key=lambda entry: entry[0])
        temperatures = [entry[0] for entry in temperature_grids]
        energy_ranges = [entry[1] for entry in temperature_grids]

        reactions: list[int] = []
        photon_production: list[int] = []
        for reaction in group["reactions"].values():
            if not isinstance(reaction, h5py.Group) or "mt" not in reaction.attrs:
                continue
            mt = int(reaction.attrs["mt"])
            reactions.append(mt)
            products = [
                product
                for name, product in reaction.items()
                if name.startswith("product_") and isinstance(product, h5py.Group)
            ]
            if any(
                decode_text(product.attrs.get("particle", "")) == "photon"
                for product in products
            ):
                photon_production.append(mt)

    return {
        "nuclide": nuclide,
        "artifact": artifact(path, data_root),
        "hdf5_version": version,
        "atomic_weight_ratio": atomic_weight_ratio,
        "temperatures_k": temperatures,
        "energy_ranges_ev": energy_ranges,
        "reactions_mt": sorted(set(reactions)),
        "photon_production_mts": sorted(set(photon_production)),
    }


def inspect_photon(path: Path, element: str, data_root: Path) -> dict[str, Any]:
    with h5py.File(path, "r") as handle:
        if decode_text(handle.attrs.get("filetype", "")) != "data_photon":
            raise ValueError(f"not an OpenMC incident-photon HDF5 file: {path}")
        version = hdf5_version(handle, path)
        group = expected_root_group(handle, element, path)
        reactions = sorted(
            mt for name, mt in PHOTON_REACTION_GROUPS.items() if name in group
        )

        subshells = group.get("subshells")
        designators = [] if subshells is None else list(
            subshells.attrs.get("designators", [])
        )
        has_atomic_relaxation = bool(designators) and all(
            "binding_energy" in subshells[decode_text(designator)].attrs
            and "num_electrons" in subshells[decode_text(designator)].attrs
            for designator in designators
        )

        compton = group.get("compton_profiles")
        has_compton_profiles = compton is not None and all(
            name in compton
            for name in ("num_electrons", "binding_energy", "pz", "J")
        )

    return {
        "element": element,
        "artifact": artifact(path, data_root),
        "hdf5_version": version,
        "reactions_mt": reactions,
        "has_atomic_relaxation_data": has_atomic_relaxation,
        "has_compton_profile_data": has_compton_profiles,
    }


def read_cross_sections(path: Path) -> tuple[Path, list[dict[str, Any]]]:
    root = ET.parse(path).getroot()
    if root.tag != "cross_sections":
        raise ValueError(f"unexpected cross-sections root element {root.tag!r}")
    directory = root.findtext("directory")
    if directory and directory.strip():
        candidate = Path(directory.strip())
        base = candidate if candidate.is_absolute() else path.parent / candidate
    else:
        base = path.parent

    libraries = []
    for node in root.findall("library"):
        library_path = Path(node.attrib["path"])
        resolved = (
            library_path if library_path.is_absolute() else base / library_path
        ).resolve(strict=True)
        libraries.append(
            {
                "type": node.attrib["type"],
                "materials": node.attrib["materials"].split(),
                "path": resolved,
            }
        )
    return base, libraries


def select_library(
    libraries: list[dict[str, Any]], library_type: str, material: str
) -> Path:
    matches = [
        library["path"]
        for library in libraries
        if library["type"] == library_type and material in library["materials"]
    ]
    if len(matches) != 1:
        raise ValueError(
            f"cross_sections.xml has {len(matches)} {library_type} mappings "
            f"for {material}; expected exactly one"
        )
    return matches[0]


def material_requirements(path: Path) -> tuple[list[str], list[str]]:
    document = json.loads(path.read_text(encoding="utf-8"))
    nuclides = [entry["name"] for entry in document["nuclides"]]
    if not nuclides or len(nuclides) != len(set(nuclides)):
        raise ValueError("material must contain a nonempty, unique nuclide list")
    elements = []
    for nuclide in nuclides:
        match = NUCLIDE_PATTERN.fullmatch(nuclide)
        if match is None:
            raise ValueError(f"invalid GNDS-style nuclide name {nuclide!r}")
        elements.append(match.group(1))
    return sorted(nuclides), sorted(set(elements))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", required=True, type=Path)
    parser.add_argument("--cross-sections", required=True, type=Path)
    parser.add_argument("--material", required=True, type=Path)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--archive-source-uri", required=True)
    parser.add_argument("--distribution-id", required=True)
    parser.add_argument("--manifest-id", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    data_root = arguments.data_root.resolve(strict=True)
    cross_sections = arguments.cross_sections.resolve(strict=True)
    archive = arguments.archive.resolve(strict=True)
    material = arguments.material.resolve(strict=True)
    inspector = Path(__file__).resolve(strict=True)

    data_relative_path(cross_sections, data_root)
    _, libraries = read_cross_sections(cross_sections)
    nuclides, elements = material_requirements(material)
    neutron_tables = [
        inspect_neutron(
            select_library(libraries, "neutron", nuclide), nuclide, data_root
        )
        for nuclide in nuclides
    ]
    photon_tables = [
        inspect_photon(
            select_library(libraries, "photon", element), element, data_root
        )
        for element in elements
    ]

    manifest = {
        "schema_version": "nctforge.openmc-nuclear-data-manifest/0.2.0",
        "id": arguments.manifest_id,
        "openmc_version": OPENMC_VERSION,
        "openmc_source_commit": OPENMC_SOURCE_COMMIT,
        "evaluated_data_release": EVALUATED_DATA_RELEASE,
        "inspection": {
            "method": INSPECTION_METHOD,
            "source_sha256": sha256_file(inspector),
            "python_version": platform.python_version(),
            "numpy_version": numpy.__version__,
            "h5py_version": h5py.__version__,
            "hdf5_library_version": h5py.version.hdf5_version,
        },
        "distribution": {
            "id": arguments.distribution_id,
            "source_uri": arguments.archive_source_uri,
            "archive_sha256": sha256_file(archive),
        },
        "cross_sections": artifact(cross_sections, data_root),
        "neutron_tables": neutron_tables,
        "photon_tables": photon_tables,
    }

    with arguments.output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(manifest, stream, indent=2, allow_nan=False)
        stream.write("\n")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, ValueError, ET.ParseError) as error:
        raise SystemExit(f"error: {error}") from error
