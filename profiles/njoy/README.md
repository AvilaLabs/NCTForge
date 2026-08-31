# NJOY evaluated-data profiles

These profiles identify publisher artifacts that may be screened as alternate
evaluated-neutron inputs to the frozen NJOY response-generation method. A
matched acquisition proves artifact identity only. It does not qualify a
library, a generated KERMA table, or the BNCT dose method.

- `jeff40-neutron-evaluations.json` pins the complete JEFF-4.0 incident-neutron
  archive published by the OECD Nuclear Energy Agency Data Bank. The profile
  binds the publisher-reported byte count and MD5 digest.

Candidate selections use
`nctforge.evaluated-neutron-source-selection/0.2.0` and must remain
`response_treatment_candidate_unreviewed` until their controlled NJOY
execution, transported-photon suitability, and independent review gates pass.
