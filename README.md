# world

Models of the Earth and its moving operational environment for vehicles and
the tools around them: the questions a flight controller, a simulator and a
ground station all need answered the same way, from the same data contracts.

## Two-Tier Workspace

### Tier 0 — Physical Baseline (`no_std`, zero `alloc`, `libm` only)

Enabled by default in the [`world`](crates/world) umbrella crate (`default = ["atmosphere", "geomag"]`):

- `world-core`: the shared vocabulary — geodetic position and datum,
  [`LocalTangentPlane`] (`Ned`), Somigliana [`normal_gravity`], decimal-year
  epoch, dataset identity, provenance, validity window and uncertainty.
- `world-geomag`: the World Magnetic Model (`WMM2025`): main field,
  declination and inclination at a position and date, with published one-sigma
  uncertainty. Verified against NOAA's official test values.
- `world-atmosphere`: the ISO 2533 International Standard Atmosphere (`Isa`),
  behind the `AtmosphereModel` trait a weather field implements in turn.

### Tier 1 — Moving Operational World (`no_std` + `alloc`, opt-in `moving`)

Enabled via `features = ["moving"]` (or individually via `airmass`,
`surveillance`, `notices`) on [`world`](crates/world). Every store is a
value-typed, single-writer state machine with no process-global state, threads,
sockets or system clocks — so a simulator holds one `MovingWorld` for physical
truth and separate `MovingWorld` instances for each vehicle's delayed radio or
datalink belief:

- `world-airmass`: multi-altitude winds and temperatures aloft (`SoundingProfile`),
  surface station observations (`SurfaceObservation`, `FlightCategory`),
  regional/global cloud layers (`CloudLayer`) and convective/icing/turbulence
  hazard volumes (`WeatherHazard`), implementing `world_atmosphere::AtmosphereModel`
  and `WindModel`.
- `world-surveillance`: multi-source airborne and surface traffic store
  (`SurveillanceStore`) spanning ADS-B (`1090ES`, `UAT 978`), ADS-R, TIS-B,
  Mode-S/MLAT, FLARM, ASTM F3411 Remote ID, C2 networks, radar and simulated
  traffic, with source-authority ranking, coasting/expiration, bounded dead
  reckoning and ownship-relative CPA geometry.
- `world-notices`: perishable operational restrictions — Temporary Flight
  Restrictions (`TfrNotice`), aerodrome/runway closure NOTAMs
  (`SurfaceClosureNotice`) and Special Use Airspace hot/cold activation
  schedules (`SuaActivationNotice`) referencing static `Flightbag` facility,
  runway and airspace keys without duplicating cycle-dated static charts.

CI builds both Tier 0 and Tier 1 for `thumbv7em-none-eabihf` and
`wasm32-unknown-unknown`. Anything that needs `std` (fetching, caching or
streaming datasets over networks) belongs in a separate data-plane adapter
that the models never depend on.

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
