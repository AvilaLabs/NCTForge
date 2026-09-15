# openbnct-transport

Transport-backend contract layer for
[OpenBNCT](https://github.com/AvilaLabs/OpenBNCT). Defines the
`TransportCase`, fixed-source and material definitions, facility beam
descriptions (`openbnct.beam-description`), beam-quality reports, measurement
import/comparison records, positioning reports, neutron response sets, and the
versioned variance-reduction specification (`openbnct.variance-reduction` /
`openbnct.weight-windows`) that backends resolve into transport-native form.

Transport backends (OpenMC first) consume these contracts; the contracts never
name a backend.

Research software — no clinical, commissioning, or regulatory qualification.
