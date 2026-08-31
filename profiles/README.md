# Acquisition profiles

These reviewed JSON files describe external artifacts that NCTForge may acquire.
They pin publication locations, redirect boundaries, observed byte counts, and
publisher digests when one is available. A profile is an acquisition input, not
an assertion that the downloaded nuclear data are scientifically qualified.

Probe a profile before transferring its artifact:

```text
cargo run --bin nctforge -- openmc data probe \
  --profile profiles/openmc/openmc-endfb81-official-library.json
```

The acquisition command requires the probe's exact byte count. It writes to a
`.part` file, resumes only from a validated byte range, refuses to overwrite a
completed output, and emits a JSON receipt beside the artifact.

Profile families are separated by use: `openmc/` contains OpenMC transport and
ENDF/B-VIII.1 baseline artifacts, while `njoy/` contains evaluated-data
candidates for the response-generation investigation.
