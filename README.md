# world

Models of the Earth for vehicles and the tools around them: the questions
a flight controller, a simulator and a ground station all need answered
the same way, from the same published data.

- `world-core`: the shared vocabulary: geodetic position and datum,
  decimal-year epoch, dataset identity, provenance, validity window and
  uncertainty.
- `world-geomag`: the World Magnetic Model (WMM2025): the main field,
  declination and inclination at a position and date, with the model's
  published uncertainty. Verified against NOAA's official test values.
- `world-atmosphere`: the ISO 2533 International Standard Atmosphere:
  temperature, pressure and density at a typed altitude, behind the
  `AtmosphereModel` trait a weather field can implement later.

## Tiers

Models are `no_std`, allocation-free and deterministic, with `libm` for
the transcendental functions, so the same crate runs in flight-controller
firmware, in a simulator and in a browser. CI builds them for
`thumbv7em-none-eabihf` and `wasm32-unknown-unknown`. Anything that
needs `std` (fetching, caching or updating datasets) belongs in a
separate data-plane crate that the models never depend on.

## Usage

```rust
use world_core::{DecimalYear, GeodeticPosition};
use world_geomag::wmm2025;

let field = wmm2025(
    GeodeticPosition::wgs84(40.577, -74.738, 56.3),
    DecimalYear::from_unix_seconds(1_767_225_600),
)?;
let declination_rad = field.declination_rad();
```

## Licence

AGPL-3.0-or-later; see [LICENSE](LICENSE). The datasets keep their own
terms, recorded in each model's provenance: the World Magnetic Model is
in the public domain.
