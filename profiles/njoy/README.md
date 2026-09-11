# NJOY evaluated-data profiles

These profiles identify publisher artifacts that may be screened as alternate
evaluated-neutron inputs to the frozen NJOY response-generation method. A
matched acquisition proves artifact identity only. It does not qualify a
library, a generated KERMA table, or the BNCT dose method.

- `jeff40-neutron-evaluations.json` pins the complete JEFF-4.0 incident-neutron
  archive published by the OECD Nuclear Energy Agency Data Bank. The profile
  binds the publisher-reported byte count and MD5 digest.
- `tendl2025-neutron-evaluations.json` pins the complete TENDL-2025
  incident-neutron archive published through the Imperial College London
  mirror. TENDL covers Z ≥ 3 only and distributes no publisher digest; the
  profile binds the observed byte count and records that digest absence
  explicitly.

Candidate selections use
`nctforge.evaluated-neutron-source-selection/0.2.0` or, when they bind more
than one publisher acquisition, `0.3.0` (ADR 0028). They must remain
`response_treatment_candidate_unreviewed` until their controlled NJOY
execution, transported-photon suitability, and independent review gates pass.
