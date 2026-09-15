# Python worked example

`workflow.py` exercises the Python parity surface end to end — the same
Rust engines the CLI calls, driven through `import openbnct`:

- **Case pipeline** — `generate_case` → `verify_case` → `load_case`,
  then geometry/structure inspection, contract-driven source aiming, and
  `export_mcnp_deck` for the reproduction path.
- **Dose analysis** — `load_physical_dose_bundle` → `compute_dvh` →
  `compute_metrics` → `apply_model` → `compare_dose_bundles`, all on the
  committed 2-voxel conformance bundle and its matching mask/model.

The transport run itself is external (OpenMC or a licensed engine) and is
not part of the example; everything else runs against committed
artifacts. All outputs live under a temporary directory.

```text
pip install bindings/python/dist/openbnct-*.whl   # or: maturin develop
python examples/python/workflow.py
```

CI runs the same script against the built wheel. Research use only —
the biological weights and all outputs carry no clinical claim.
