# ADR 0005: OpenMC 0.16 Estimator Boundary

**Status:** Accepted for R2 input preparation

**Date:** 2026-08-31

## Context

OpenMC 0.16.0 is the current release selected for NCTForge's first candidate
transport input. It adds reaction and secondary-particle-production filters,
enables track-length neutron-heating tallies, and changes how energy from
photon-induced electrons and positrons is attributed in heating tallies.

Those additions are useful diagnostics, but a reaction filter does not make the
neutron `heating` score reaction-specific. OpenMC's 0.16.0 scoring code states
that no reaction-wise heating cross section is available. A reaction filter
changes a track-length estimator to collision and assigns the total heating
response to bins selected by the sampled event. That is not the same quantity
as the KERMA coefficient for that reaction.

## Decision

The first candidate adapter targets OpenMC 0.16.0 exactly. Every prepared or
executed run records the semantic version, executable hash, and source commit
when available. A later OpenMC release requires a new reviewed adapter profile;
it does not silently replace this target.

The reported macroscopic components remain material-specific neutron-response
folds for `boron`, `nitrogen`, and `hydrogen`, plus coupled photon heating for
`photon`:

- `boron` assigns the charged-particle portion of B-10 MT=107 and excludes
  energy transported by emitted photons;
- `nitrogen` assigns the charged-particle portion of N-14 MT=103 and excludes
  photon energy;
- `hydrogen` is the residual non-photon neutron KERMA of the complete material
  after the boron and nitrogen assignments, not an H-1-only shortcut; and
- `photon` is photon heating at the deposition location, regardless of photon
  origin.

The response functions, interpolation policy, dimensional reduction, source
normalization, and hashes are versioned inputs. NCTForge will retain the
energy-binned neutron flux needed to repeat the fold outside the primary tally
path.

The audit set contains:

1. B-10 MT=107 and N-14 MT=103 reaction rates;
2. neutron-only `heating` with an explicit neutron particle filter;
3. photon-only `heating` with an explicit photon particle filter;
4. coupled total `heating` without a particle filter;
5. energy-binned neutron and photon flux; and
6. particle-resolved surface leakage.

`ReactionFilter` may be used for diagnostics and event classification. Its
neutron-heating bins are not accepted as the four reported component doses or
as independent evidence for their partition.

## Acceptance boundary

Input preparation may be enabled once deterministic XML, material/source
invariants, schema validation, and nuclear-data capability preflight pass.
Execution and result-import capabilities remain disabled until a real OpenMC
smoke run and statepoint checks pass. No response table or reference dose is
qualified by this decision.

## Consequences

- OpenMC 0.16.0 features improve diagnostics without changing NCTForge's
  transport-neutral component semantics.
- A convenient reaction-filtered heating tally cannot substitute for the
  independently reviewed response-generation pipeline.
- The component sum is compared with a dedicated total estimator; component
  uncertainties are not combined as if independent.
- Updating OpenMC becomes an explicit evidence event with regression data.

## Primary sources

- [OpenMC 0.16.0 release](https://github.com/openmc-dev/openmc/releases/tag/v0.16.0)
- [OpenMC ReactionFilter documentation](https://docs.openmc.org/en/v0.16.0/pythonapi/generated/openmc.ReactionFilter.html)
- [OpenMC 0.16.0 estimator selection](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/tallies/tally.cpp)
- [OpenMC 0.16.0 neutron-heating implementation](https://github.com/openmc-dev/openmc/blob/v0.16.0/src/tallies/tally_scoring.cpp)
- [OpenMC heating and energy deposition](https://docs.openmc.org/en/v0.16.0/methods/energy_deposition.html)
