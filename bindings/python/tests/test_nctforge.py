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


class MetricsAndEndpointTest(unittest.TestCase):
    def _endpoint_model_json(self) -> str:
        return json.dumps(
            {
                "schema_version": "nctforge.endpoint-model/0.1.0",
                "id": "nctforge.tests.logistic-tcp.v1",
                "endpoint": "tcp",
                "function": {
                    "kind": "logistic",
                    "d50": 2.0e-12,
                    "gamma50": 2.0,
                },
                "dose_statistic": {"statistic": "mean"},
            }
        )

    def test_metrics_match_rust_semantics(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle = nctforge.load_physical_dose_bundle(
                _write(tmp, "dose.json", _physical_bundle_json())
            )
            metrics = nctforge.compute_metrics(
                bundle,
                "physical_total",
                "all",
                [True, True],
                [50.0, 100.0],
                [1.75e-12, 2.0e-12],
                [1.0, 10.0],
            )
            self.assertEqual(metrics.unit, "gray_per_source_particle")
            self.assertEqual(metrics.minimum_dose, metrics.maximum_dose)
            self.assertEqual(metrics.mean_dose, 1.75e-12)
            # Uniform two-voxel volume: D100 = D50 = the single dose level.
            self.assertEqual(metrics.dx, [(50.0, 1.75e-12), (100.0, 1.75e-12)])
            self.assertEqual(metrics.vx, [(1.75e-12, 1.0), (2.0e-12, 0.0)])
            self.assertEqual(metrics.eud[0][1], 1.75e-12)  # a=1 is the mean
            self.assertEqual(metrics.schema_version,
                             "nctforge.dose-metrics/0.1.0")

    def test_endpoint_evaluate_and_utcp(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle = nctforge.load_physical_dose_bundle(
                _write(tmp, "dose.json", _physical_bundle_json())
            )
            model = nctforge.load_endpoint_model(
                _write(tmp, "model.json", self._endpoint_model_json())
            )
            tcp = nctforge.evaluate_endpoint(
                model, bundle, "physical_total", "all", [True, True]
            )
            self.assertEqual(tcp.endpoint, "tcp")
            # Mean dose 1.75e-12 with d50 = 2e-12, gamma50 = 2:
            # P = 1/(1 + (2/1.75)^8).
            expected = 1.0 / (1.0 + (2.0 / 1.75) ** 8)
            self.assertAlmostEqual(tcp.probability, expected, delta=1e-9)
            self.assertEqual(tcp.dose_statistic.kind, "mean")

            ntcp_document = json.loads(self._endpoint_model_json())
            ntcp_document["endpoint"] = "ntcp"
            ntcp_document["function"] = {
                "kind": "probit",
                "td50": 1.75e-12,
                "m": 0.3,
            }
            ntcp_model = nctforge.load_endpoint_model(
                _write(tmp, "ntcp.json", json.dumps(ntcp_document))
            )
            ntcp = nctforge.evaluate_endpoint(
                ntcp_model, bundle, "physical_total", "all", [True, True]
            )
            # Mean dose = td50 -> probit is ~0.5 within approximation bound.
            self.assertAlmostEqual(ntcp.probability, 0.5, delta=1e-6)

            utcp = nctforge.combine_utcp(tcp, ntcp, "p_plus")
            self.assertEqual(utcp.endpoint, "utcp")
            self.assertAlmostEqual(
                utcp.probability,
                tcp.probability * (1.0 - ntcp.probability),
                delta=1e-12,
            )
            self.assertIsNone(utcp.dose_statistic)
            self.assertEqual(
                utcp.qualification, "synthetic_research_only_not_clinical"
            )

            # Two TCP evaluations cannot be combined.
            with self.assertRaises(NctForgeError):
                nctforge.combine_utcp(tcp, tcp, "p_plus")


class ExposurePlanTest(unittest.TestCase):
    def test_table_round_trip_and_accumulate(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            bundle_path = _write(tmp, "dose.json", _physical_bundle_json())
            digest = nctforge.file_sha256(bundle_path)
            exposure = {
                "name": "field-a",
                "dose_bundle": {
                    "id": "dose",
                    "sha256": digest,
                    "path": "dose.json",
                },
                "weight": 1.5,
                "weight_basis": "delivered_fraction",
                "duration_s": 600.0,
                "boron_assumption": "10 ppm B-10",
            }
            plan = {
                "schema_version": "nctforge.exposure-plan/0.1.0",
                "id": "nctforge.test.plan.v1",
                "case_id": "accumulated-case",
                "covariance": "independent_exposures",
                "exposures": [exposure],
            }
            plan_path = _write(tmp, "plan.json", json.dumps(plan))
            loaded = nctforge.load_exposure_plan(plan_path)
            self.assertEqual(loaded.id, "nctforge.test.plan.v1")
            self.assertEqual(loaded.exposures[0].weight_basis, "delivered_fraction")
            self.assertEqual(loaded.validate_diagnostics(), [])

            # Accumulation through Python returns the same bundle the CLI
            # `accumulate` command writes (1.5x dose on the fixture).
            accumulated = nctforge.accumulate_exposures(plan_path)
            self.assertEqual(accumulated.case_id, "accumulated-case")
            self.assertAlmostEqual(
                accumulated.physical_total.values[0], 1.75e-12 * 1.5, places=18
            )

            # Table export -> import round-trips through both surfaces.
            csv_path = Path(tmp) / "schedule.csv"
            xlsx_path = Path(tmp) / "schedule.xlsx"
            nctforge.plan_table_write(plan_path, csv_path)
            nctforge.plan_table_write(plan_path, xlsx_path)
            from_csv = Path(tmp) / "from-csv.json"
            from_xlsx = Path(tmp) / "from-xlsx.json"
            nctforge.plan_table_read(csv_path, from_csv)
            nctforge.plan_table_read(xlsx_path, from_xlsx)
            self.assertEqual(
                json.loads(from_csv.read_text()), json.loads(plan_path.read_text())
            )
            self.assertEqual(
                json.loads(from_xlsx.read_text()), json.loads(plan_path.read_text())
            )

    def test_diagnostics_report_every_issue(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            plan = {
                "schema_version": "nctforge.exposure-plan/0.1.0",
                "id": "  ",
                "case_id": "case",
                "covariance": "independent_exposures",
                "exposures": [
                    {
                        "name": "a",
                        "dose_bundle": {"id": "x", "sha256": "bad", "path": "x"},
                        "weight": -1.0,
                        "weight_basis": "manual",
                        "duration_s": None,
                        "boron_assumption": None,
                    }
                ],
            }
            path = _write(tmp, "bad.json", json.dumps(plan))
            issues = nctforge.exposure_plan_diagnostics(path)
            # empty id, negative weight, malformed sha256 — all reported.
            self.assertEqual(len(issues), 3)
            # The strict loader still refuses the same document.
            with self.assertRaises(NctForgeError):
                nctforge.load_exposure_plan(path)


class InterchangeTest(unittest.TestCase):
    def test_imports_external_component_dose(self) -> None:
        document = (
            REPO_ROOT / "examples" / "interchange" / "phits-synthetic-dose.json"
        )
        bundle = nctforge.import_component_dose(document)
        self.assertEqual(bundle.case_id, "nf-bnct-001-phits-synthetic")
        self.assertEqual(bundle.physical_total.unit, "gray_per_source_particle")
        # component_sum imports never claim a total uncertainty.
        self.assertIsNone(bundle.physical_total.absolute_standard_uncertainty)
        self.assertIn("interchange:phits:sha256:", bundle.provenance_id)
        self.assertEqual(len(bundle.components), 4)
        # The component sum equals the physical total (tolerance for f64 sum).
        for component in bundle.components:
            self.assertEqual(len(component.values), 64000)
        summed = [sum(c.values[i] for c in bundle.components) for i in (0, 1000)]
        for index, expected in zip((0, 1000), summed):
            self.assertAlmostEqual(
                bundle.physical_total.values[index], expected, places=18
            )

    def test_rejects_malformed_interchange(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = json.loads(
                (
                    REPO_ROOT
                    / "examples"
                    / "interchange"
                    / "phits-synthetic-dose.json"
                ).read_text()
            )
            doc["components"].pop()  # drop photon — a required component
            path = _write(tmp, "broken.json", json.dumps(doc))
            with self.assertRaises(NctForgeError):
                nctforge.import_component_dose(path)


MESHTAL_FIXTURE = """mcnp   version 6.2 ld=01/01/20  probid =  01/01/20 00:00:00
 parity fixture
 Number of histories used for normalizing tallies =    1000.00

 Mesh Tally Number        4
 neutron  mesh tally.

 Tally bin boundaries:
    X direction:      -1.00      1.00
    Y direction:      -1.00      0.00      1.00
    Z direction:      -1.00      1.00
    Energy bin boundaries: 0.00E+00 2.00E+01

   Energy         X         Y         Z     Result     Rel Error
  2.000E+01     0.000   -0.500     0.000 1.00000E-03 1.00000E-02
  2.000E+01     0.000    0.500     0.000 2.00000E-03 1.00000E-02
"""

PHITS_FIXTURE = """[ T - D e p o s i t ]
    mesh =  xyz
  x-type =    2
    xmin =  -1.0
    xmax =   1.0
      nx =    2
  y-type =    2
    ymin =  -1.0
    ymax =   1.0
      ny =    2
  z-type =    2
    zmin =  -1.0
    zmax =   1.0
      nz =    2
    unit =    0
    axis =   xy
    file = d.out
   output =  dose

#newpage:
# no. = 1  iz = 1
x: x [cm]
y: y [cm]
h: x n n y n y(all),hh0l n
# x-lower      x-upper      y-lower      y-upper      all        r.err
 -1.0000E+00   0.0000E+00  -1.0000E+00   0.0000E+00   1.0000E-03 1.0000E-02
  0.0000E+00   1.0000E+00  -1.0000E+00   0.0000E+00   2.0000E-03 1.0000E-02
 -1.0000E+00   0.0000E+00   0.0000E+00   1.0000E+00   4.0000E-03 1.0000E-02
  0.0000E+00   1.0000E+00   0.0000E+00   1.0000E+00   5.0000E-03 1.0000E-02

#newpage:
# no. = 2  iz = 2
x: x [cm]
y: y [cm]
h: x n n y n y(all),hh0l n
# x-lower      x-upper      y-lower      y-upper      all        r.err
 -1.0000E+00   0.0000E+00  -1.0000E+00   0.0000E+00   7.0000E-03 2.0000E-02
  0.0000E+00   1.0000E+00  -1.0000E+00   0.0000E+00   8.0000E-03 2.0000E-02
 -1.0000E+00   0.0000E+00   0.0000E+00   1.0000E+00   9.0000E-03 2.0000E-02
  0.0000E+00   1.0000E+00   0.0000E+00   1.0000E+00   1.0000E-02 2.0000E-02
"""


class ExternalAdapterTest(unittest.TestCase):
    def test_mcnp_meshtal_import(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = _write(tmp, "meshtal", MESHTAL_FIXTURE)
            bundle = nctforge.import_mcnp_meshtal(
                {
                    "boron": (path, 4),
                    "nitrogen": (path, 4),
                    "hydrogen": (path, 4),
                    "photon": (path, 4),
                },
                case_id="py-mcnp",
                unit="gray_per_source_particle",
                normalization="per source particle; fixture",
            )
            self.assertEqual(bundle.case_id, "py-mcnp")
            self.assertIn("interchange:mcnp:sha256:", bundle.provenance_id)
            # 1x2x1 grid: values = 4 x 1e-3 at index 0, 4 x 2e-3 at index 1.
            self.assertAlmostEqual(bundle.physical_total.values[0], 4.0e-3)
            self.assertAlmostEqual(bundle.physical_total.values[1], 8.0e-3)
            self.assertIsNone(bundle.physical_total.absolute_standard_uncertainty)

    def test_phits_import(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = _write(tmp, "d.out", PHITS_FIXTURE)
            bundle = nctforge.import_phits(
                {
                    "boron": path,
                    "nitrogen": path,
                    "hydrogen": path,
                    "photon": path,
                },
                case_id="py-phits",
                unit="gray_per_source_particle",
                normalization="unit=0; fixture",
                producer_version="3.34",
            )
            self.assertIn("interchange:phits:sha256:", bundle.provenance_id)
            # 2x2x2: index 7 = (1,1,1) = 4 x 1e-2.
            self.assertAlmostEqual(bundle.physical_total.values[7], 4.0e-2)

    def test_adapters_reject_bad_specs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = _write(tmp, "meshtal", MESHTAL_FIXTURE)
            with self.assertRaises(NctForgeError):
                nctforge.import_mcnp_meshtal(
                    {"boron": (path, 99), "nitrogen": (path, 4),
                     "hydrogen": (path, 4), "photon": (path, 4)},
                    "c", "gray_per_source_particle", "x",
                )
            with self.assertRaises(NctForgeError):
                nctforge.import_phits(
                    {"boron": path, "nitrogen": path,
                     "hydrogen": path, "photon": path},
                    "c", "gray_per_source_particle", "x", " ",
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
