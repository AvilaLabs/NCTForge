# ADR 0001: Transport-neutral core

- **Status:** Accepted
- **Date:** 2026-08-31

## Decision

NCTForge's case, physical-dose, biological, QA, and evidence models will not
depend on OpenMC types or files. OpenMC will be implemented as the first
end-to-end transport adapter behind the `TransportBackend` contract.

## Rationale

Backend neutrality allows independent calculation, cross-code comparison,
facility-specific engines, and imported historical results. It also prevents
the GUI and scientific interpretation layers from becoming obsolete if the
preferred transport engine changes.

## Consequences

- Adapter-specific metadata must be preserved in evidence artifacts without
  leaking into the normalized physical-dose contract.
- The common contract may represent only scientifically shared semantics; it
  must not erase backend-specific qualifications.
- OpenMC convenience must not bypass the adapter boundary.
- Imported results can be inspected and compared without granting permission
  to redistribute the originating transport software.
