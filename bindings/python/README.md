# Python bindings

`pip install nctforge` is the planned primary entry point for scientific users.
The package will use PyO3 and maturin to wrap the authoritative Rust crates; it
will not become a second production dose, geometry, evidence, or QA engine.

The intended mixed-package shape is:

```text
bindings/python/
  Cargo.toml                 PyO3 extension crate
  pyproject.toml             maturin build and package metadata
  src/lib.rs                 narrow Rust-to-Python boundary
  python/nctforge/
    __init__.py              ergonomic public API
    _nctforge.pyi            checked extension types
    py.typed                 typing marker
```

The first bounded API will target case verification, geometry inspection,
normalized contracts, and evidence reading. Prebuilt wheels and clean install
tests are required before a PyPI release so normal users do not need Rust merely
to install a supported wheel. Transport actions remain unavailable until the
same Rust capability and evidence gates used by the CLI and GUI pass.

Packaging is not implemented and no PyPI release is claimed yet. See [ADR
0015](../../docs/adr/0015-python-and-native-distribution.md) for the accepted
distribution boundary and release gates.
