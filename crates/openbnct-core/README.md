# openbnct-core

Backend-neutral BNCT geometry, grid, and physical-dose contracts for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT). Owns the canonical
`PhysicalDoseBundle`, `GridGeometry`, dose-component semantics, rigid
registration (`openbnct.registration`), systematic-uncertainty
(`openbnct.systematic-uncertainty`), exposure-plan, external-dose, and
component-dose interchange contracts, plus the contract-namespace helpers
(`schema_matches`, `normalize_contract_id`) that keep pre-rename `nctforge.*`
artifacts readable.

All other OpenBNCT crates depend on this crate's types; nothing in it knows
about any transport backend.

Research software — no clinical, commissioning, or regulatory qualification.
