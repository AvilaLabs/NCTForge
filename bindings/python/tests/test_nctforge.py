"""Cross-language parity tests for the nctforge Python boundary.

Every expectation here is also enforced by the Rust workspace tests; this
suite proves the Python layer observes identical acceptance, rejection,
canonical serialization, and content identity on shared inputs (ADR 0015).
"""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import nctforge
from nctforge import NctForgeError

REPO_ROOT = Path(__file__).resolve().parents[3]
TRANSPORT_DIR = REPO_ROOT / "benchmarks" / "synthetic" / "nf-bnct-001" / "transport"


def _response_set_json(qualification: str, with_review: bool) -> str:
    """The same synthetic fixture used by the Rust contract tests."""
    reference = lambda seed: {"id": seed, "sha256": seed * 64}
    document = {
        "schema_version": "nctforge.neutron-response-set/0.1.0",
        "id": "nctforge.synthetic-response-set.v1",
        "qualification": qualification,
        "component_profile": reference("a"),
        "material": reference("b"),
        "nuclear_data_manifest": reference("c"),
        "generation_method": reference("d"),
        "independent_review": reference("e") if with_review else None,
        "transport_energy_range_ev": [0.0, 20.0],
        "energy_ev": [0.0, 1.0, 20.0],
        "unit": "gray_square_centimeter",
        "interpolation": "linear_linear",
        "boron_gy_cm2": [1.0, 2.0, 3.0],
        "nitrogen_gy_cm2": [2.0, 3.0, 4.0],
        "hydrogen_gy_cm2": [3.0, 4.0, 5.0],
        "total_neutron_gy_cm2": [6.0, 9.0, 12.0],
    }
    return json.dumps(document)


class CaseLifecycleTest(unittest.TestCase):
    def test_generate_verify_load_roundtrip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "nf-bnct-001"
            generated = nctforge.generate_case(root)
            self.assertEqual(generated.ct_file_count, 40)
            self.assertTrue(Path(generated.manifest_file).is_file())

            report = nctforge.verify_case(root)
            self.assertEqual(report.case_id, "NF-BNCT-001")
            self.assertEqual(report.shape, (40, 40, 40))
            self.assertEqual(report.spacing_mm, (5.0, 5.0, 5.0))
            self.assertEqual(report.origin_mm, (-97.5, -97.5, -97.5))
            self.assertEqual(report.ct_slice_count, 40)
            self.assertGreater(report.verified_artifact_count, 40)

            case = nctforge.load_case(root)
            self.assertEqual(case.geometry.shape, (40, 40, 40))
            self.assertEqual(case.geometry.voxel_count, 64_000)
            self.assertEqual(
                case.geometry.direction,
                (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0),
            )
            self.assertEqual(
                case.geometry.voxel_center_lps_mm(0, 0, 0), (-97.5, -97.5, -97.5)
            )
            self.assertEqual(case.ct_value(0, 0, 0), 0.0)

            rois = {roi.name: roi for roi in case.structures}
            self.assertEqual(
                set(rois),
                {
                    "PHANTOM",
                    "CORE",
                    "LEFT_ANTERIOR_MARKER",
                    "RIGHT_POSTERIOR_MARKER",
                    "CENTRAL_AXIS_2CM",
                },
            )
            self.assertEqual(rois["PHANTOM"].voxel_count, 64_000)
            self.assertEqual(rois["PHANTOM"].volume_cm3, 8_000.0)
            self.assertEqual(rois["CORE"].centroid_lps_mm, (0.0, 0.0, 0.0))
            self.assertEqual(
                rois["LEFT_ANTERIOR_MARKER"].centroid_lps_mm, (70.0, -70.0, -70.0)
            )

            mask = case.structure_mask("CORE")
            self.assertEqual(len(mask), 64_000)
            self.assertEqual(sum(mask), 512)

    def test_generate_refuses_existing_destination(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "nf-bnct-001"
            nctforge.generate_case(root)
            with self.assertRaises(NctForgeError):
                nctforge.generate_case(root)

    def test_verify_rejects_corrupted_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "nf-bnct-001"
            nctforge.generate_case(root)
            target = next((root / "ct").glob("*.dcm"))
            payload = bytearray(target.read_bytes())
            payload[-1] ^= 0xFF
            target.write_bytes(payload)
            with self.assertRaises(NctForgeError):
                nctforge.verify_case(root)
            with self.assertRaises(NctForgeError):
                nctforge.load_case(root)

    def test_verify_rejects_missing_case(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(NctForgeError):
                nctforge.verify_case(Path(tmp) / "absent")


class ManifestTest(unittest.TestCase):
    def test_manifest_binds_verified_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "nf-bnct-001"
            generated = nctforge.generate_case(root)
            manifest = nctforge.read_manifest(generated.manifest_file)

            self.assertEqual(manifest.case_id, "NF-BNCT-001")
            self.assertEqual(manifest.coordinate_system, "dicom_lps_millimeters")
            self.assertEqual(manifest.qualification, "synthetic_research_only")
            self.assertEqual(manifest.geometry.shape, (40, 40, 40))
            self.assertEqual(
                manifest.verify_artifacts(root), len(manifest.artifacts)
            )

            for artifact in manifest.artifacts:
                self.assertEqual(
                    artifact.sha256, nctforge.file_sha256(root / artifact.path)
                )

            # Canonical serialization round-trips through the same contract.
            again = nctforge.read_manifest(
                _write(tmp, "manifest-again.json", manifest.to_json())
            )
            self.assertEqual(again.to_json(), manifest.to_json())

    def test_manifest_rejects_tampered_hash(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "nf-bnct-001"
            generated = nctforge.generate_case(root)
            manifest = nctforge.read_manifest(generated.manifest_file)
            target = next((root / "ct").glob("*.dcm"))
            payload = bytearray(target.read_bytes())
            payload[-1] ^= 0xFF
            target.write_bytes(payload)
            with self.assertRaises(NctForgeError):
                manifest.verify_artifacts(root)


class ContractTest(unittest.TestCase):
    def test_frozen_transport_contracts_load(self) -> None:
        material = nctforge.load_material(TRANSPORT_DIR / "material.json")
        self.assertEqual(material.id, "nctforge.nf-bnct-001.material.v1")

        source = nctforge.load_fixed_source(TRANSPORT_DIR / "source.json")
        self.assertEqual(
            source.schema_version, "nctforge.fixed-source-definition/0.1.0"
        )

        profile = nctforge.load_component_profile(
            TRANSPORT_DIR / "component-profile.json"
        )
        self.assertEqual(profile.id, "nctforge.macroscopic-absorbed-dose.v1")

        method = nctforge.load_response_generation_method(
            TRANSPORT_DIR / "response-generation-method.json"
        )
        self.assertEqual(method.id, "nctforge.nf-bnct-001.response-generation.v1")

    def test_canonical_serialization_is_stable(self) -> None:
        material = nctforge.load_material(TRANSPORT_DIR / "material.json")
        first = material.to_json()
        self.assertEqual(json.loads(first)["id"], material.id)
        with tempfile.TemporaryDirectory() as tmp:
            again = nctforge.load_material(_write(tmp, "material.json", first))
            self.assertEqual(again.to_json(), first)

    def test_contract_rejection_matches_rust(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            document = json.loads(
                (TRANSPORT_DIR / "material.json").read_text()
            )
            document["nuclides"][0]["mass_fraction"] += 1.0
            broken = _write(tmp, "material.json", json.dumps(document))
            with self.assertRaises(NctForgeError):
                nctforge.load_material(broken)

            document = json.loads(
                (TRANSPORT_DIR / "material.json").read_text()
            )
            document["unexpected"] = True
            denied = _write(tmp, "material-denied.json", json.dumps(document))
            with self.assertRaises(NctForgeError):
                nctforge.load_material(denied)


class ResponseSetGateTest(unittest.TestCase):
    def test_unreviewed_set_loads_but_cannot_fold(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = _write(
                tmp,
                "response-set.json",
                _response_set_json("generated_unreviewed", False),
            )
            response_set = nctforge.load_response_set(path)
            self.assertEqual(response_set.qualification, "generated_unreviewed")
            self.assertFalse(response_set.folding_ready)
            self.assertEqual(response_set.energy_knot_count, 3)

    def test_reviewed_set_requires_review_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            missing = _write(
                tmp,
                "response-set.json",
                _response_set_json("independently_reviewed", False),
            )
            with self.assertRaises(NctForgeError):
                nctforge.load_response_set(missing)

            reviewed = _write(
                tmp,
                "response-set-reviewed.json",
                _response_set_json("independently_reviewed", True),
            )
            response_set = nctforge.load_response_set(reviewed)
            self.assertTrue(response_set.folding_ready)


class BackendHonestyTest(unittest.TestCase):
    def test_openmc_capabilities_match_configuration(self) -> None:
        observed = {backend.id: backend for backend in nctforge.backends()}
        self.assertIn("openmc", observed)
        openmc = observed["openmc"]
        # The default backend executes and imports; preparation requires a
        # configured artifact set and stays closed.
        self.assertFalse(openmc.can_prepare)
        self.assertTrue(openmc.can_execute)
        self.assertTrue(openmc.can_import)


def _physical_bundle_json() -> str:
    """The same synthetic fixture used by the Rust bio/evidence tests."""
    reference = lambda seed: {"id": seed, "sha256": seed * (64 // len(seed))}
    component = lambda name, mean, sigma: {
        "component": name,
        "unit": "gray_per_source_particle",
        "values": [mean, mean],
        "absolute_standard_uncertainty": [sigma, sigma],
    }
    return json.dumps(
        {
            "schema_version": "nctforge.physical-dose-bundle/0.2.0",
            "case_id": "synthetic-case",
            "frame_of_reference_uid": None,
            "geometry": {
                "shape": [2, 1, 1],
                "spacing_mm": [5.0, 5.0, 5.0],
                "origin_mm": [-2.5, -2.5, -2.5],
                "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            },
            "component_profile": reference("ab"),
            "response_set": reference("cd"),
            "components": [
                component("boron", 1.0e-12, 1.0e-14),
                component("nitrogen", 2.0e-13, 2.0e-15),
                component("hydrogen", 5.0e-14, 5.0e-16),
                component("photon", 3.0e-13, 3.0e-15),
            ],
            "physical_total": {
                "unit": "gray_per_source_particle",
                "values": [1.75e-12, 1.75e-12],
                "absolute_standard_uncertainty": [1.1e-14, 1.1e-14],
                "uncertainty_method": "dedicated_estimator",
            },
            "provenance_id": "test-provenance",
        }
    )


def _model_json() -> str:
    return json.dumps(
        {
            "schema_version": "nctforge.biological-model/0.2.0",
            "id": "nctforge.tests.fixed-weights.v1",
            "weight_semantics": "fixed_per_component",
            "input_unit": "gray_per_source_particle",
            "component_weights": {
                "boron": 3.8,
                "nitrogen": 2.5,
                "hydrogen": 1.0,
                "photon": 1.0,
            },
        }
    )


class DoseBundleTest(unittest.TestCase):
    def test_load_validate_and_histogram(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = _write(tmp, "dose.json", _physical_bundle_json())
            bundle = nctforge.load_physical_dose_bundle(path)
            self.assertEqual(bundle.case_id, "synthetic-case")
            self.assertEqual(bundle.provenance_id, "test-provenance")
            self.assertEqual(len(bundle.components), 4)
            self.assertEqual(bundle.physical_total.values, [1.75e-12, 1.75e-12])

            dvh = nctforge.compute_dvh(
                bundle, "component:boron", "all", [True, True], 4
            )
            self.assertEqual(dvh.unit, "gray_per_source_particle")
            self.assertEqual(dvh.region_voxel_count, 2)
            self.assertAlmostEqual(
                sum(dvh.differential_volume_fraction), 1.0, places=9
            )
            self.assertEqual(dvh.cumulative_volume_fraction[0], 1.0)

    def test_rejects_broken_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            document = json.loads(_physical_bundle_json())
            document["components"][0]["values"] = [1.0]
            with self.assertRaises(NctForgeError):
                nctforge.load_physical_dose_bundle(
                    _write(tmp, "bad.json", json.dumps(document))
                )


class BiologicalLayerTest(unittest.TestCase):
    def test_apply_and_histogram_biological_total(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle_path = _write(tmp, "dose.json", _physical_bundle_json())
            model_path = _write(tmp, "model.json", _model_json())
            bundle = nctforge.load_physical_dose_bundle(bundle_path)
            model = nctforge.load_biological_model(model_path)
            biological = nctforge.apply_model(model, bundle, [])
            self.assertEqual(biological.unit, "weighted_gray_per_source_particle")
            self.assertEqual(
                biological.qualification, "synthetic_research_only_not_clinical"
            )
            self.assertEqual(
                biological.physical_bundle_provenance, "test-provenance"
            )
            boron = next(
                c for c in biological.components if c.component == "boron"
            )
            self.assertEqual(boron.values, [3.8e-12, 3.8e-12])
            # Total sigma is the correlated sum of weighted component sigmas.
            expected_sigma = 3.8e-14 + 2.5 * 2.0e-15 + 5.0e-16 + 3.0e-15
            self.assertAlmostEqual(
                biological.biological_total.absolute_standard_uncertainty[0],
                expected_sigma,
            )

            dvh = nctforge.compute_dvh_biological(
                biological, "biological_total", "all", [True, True], 4
            )
            self.assertEqual(dvh.unit, "weighted_gray_per_source_particle")

            out = Path(tmp) / "bio.json"
            biological.write(out)
            reloaded = json.loads(out.read_text())
            self.assertEqual(
                reloaded["schema_version"],
                "nctforge.biological-dose-bundle/0.2.0",
            )

    def test_region_mask_name_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle = nctforge.load_physical_dose_bundle(
                _write(tmp, "dose.json", _physical_bundle_json())
            )
            model_document = json.loads(_model_json())
            model_document["region_weights"] = {
                "core": model_document["component_weights"]
            }
            model = nctforge.load_biological_model(
                _write(tmp, "model.json", json.dumps(model_document))
            )
            mask = _write(
                tmp,
                "mask.json",
                json.dumps({"name": "other", "voxels": [True, False]}),
            )
            with self.assertRaises(NctForgeError):
                nctforge.apply_model(model, bundle, [("core", mask)])

    def test_photon_isoeffective_fractionated_total(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle = nctforge.load_physical_dose_bundle(
                _write(tmp, "dose.json", _physical_bundle_json())
            )
            model_document = json.loads(_model_json())
            model_document["weight_semantics"] = "photon_isoeffective"
            model_document["fractionation"] = {
                "fraction_count": 30,
                "source_particles_per_fraction": 1.0e12,
                "default_alpha_beta": 3.0,
            }
            model = nctforge.load_biological_model(
                _write(tmp, "model.json", json.dumps(model_document))
            )
            biological = nctforge.apply_model(model, bundle, [])
            self.assertEqual(biological.weight_semantics, "photon_isoeffective")
            # Weighted per-particle total w = 3.8e-12 + 2.5*2e-13 + 1*5e-14 +
            # 1*3e-13 = 4.65e-12; d = w * 1e12 = 4.65; EQD2 with n=30, r=3.
            d = 4.65
            expected = 30.0 * d * (1.0 + d / 3.0) / (1.0 + 2.0 / 3.0)
            self.assertAlmostEqual(
                biological.biological_total.values[0],
                expected,
                delta=expected * 1e-12,
            )
            self.assertEqual(
                biological.biological_total.unit, "weighted_eqd2"
            )

    def test_photon_isoeffective_rejects_non_unit_photon_weight(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model_document = json.loads(_model_json())
            model_document["weight_semantics"] = "photon_isoeffective"
            model_document["component_weights"]["photon"] = 1.4
            with self.assertRaises(NctForgeError):
                nctforge.load_biological_model(
                    _write(tmp, "model.json", json.dumps(model_document))
                )


class EvidenceBundleTest(unittest.TestCase):
    def test_verify_detects_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            payload = root / "payload.json"
            payload.write_text('{"ok": true}')
            digest = hashlib.sha256(payload.read_bytes()).hexdigest()
            (root / "artifact-manifest.json").write_text(
                json.dumps(
                    {
                        "schema_version": "nctforge.evidence-bundle-manifest/0.1.0",
                        "case_id": "synthetic-case",
                        "qualification": "synthetic_research_only",
                        "artifacts": [
                            {
                                "role": "payload",
                                "path": "payload.json",
                                "sha256": digest,
                                "media_type": None,
                            }
                        ],
                    }
                )
            )
            case_id, count = nctforge.verify_evidence_bundle(root)
            self.assertEqual((case_id, count), ("synthetic-case", 1))
            payload.write_text('{"ok": false}')
            with self.assertRaises(NctForgeError):
                nctforge.verify_evidence_bundle(root)


def _write(directory: str, name: str, content: str) -> Path:
    path = Path(directory) / name
    path.write_text(content)
    return path


if __name__ == "__main__":
    unittest.main()
