# world

> [!WARNING]
> **Experimental Prototype — Not for Navigation or Flight.**
> This repository and its crates are an **experimental software prototype** for research, visualization, and simulation only. They have **not** been certified, verified, or approved by any aviation authority, and **must never** be used for real-world navigation, flight planning, traffic avoidance, weather decision-making, or operational flight control.

Models of the Earth and its moving operational environment for vehicles and
the tools around them: the questions a flight-control prototype, a simulator
and a ground station all need answered the same way, from the same data
contracts.

## Two-Tier Workspace

### Tier 0 — Physical Baseline (`no_std`, zero `alloc`, `libm` only)

Enabled by default in the [`world`](crates/world) umbrella crate (`default = ["atmosphere", "geomag"]`):

- `world-core`: the shared vocabulary — geodetic position and datum,
  [`LocalTangentPlane`] (`Ned`), Somigliana [`normal_gravity`], decimal-year
  epoch, dataset identity, provenance, validity window and uncertainty, plus
  the `WorldInstant` (monotonic and UTC readings of one instant) and the
  bounded route sampling the Tier 1 queries share.
- `world-geomag`: the World Magnetic Model (`WMM2025`): main field,
  declination and inclination at a position and date, with published one-sigma
  uncertainty. Verified against NOAA's official test values.
- `world-atmosphere`: the ISO 2533 International Standard Atmosphere (`Isa`),
  behind the `AtmosphereModel` trait a weather field implements in turn.
- `world-potential` (a direct dependency; not re-exported by `world`):
  spherical-harmonic gravity and magnetic potential fields of any body —
  potential, field vector and gradient tensor from one allocation-free
  synthesis core, composable as `dyn VectorField` with gridded anomaly
  overlays. Its `alloc` feature reads ICGEM `.gfc`, PDS SHADR and
  WMM `.COF` and IGRF table files and embeds IGRF-14 and EGM96 to degree 36.
- `world-thermosphere` (a direct dependency; not re-exported by `world`):
  the NRLMSISE-00 empirical atmosphere (`Nrlmsise00`) — species number
  densities, total and drag mass density and temperature from the ground
  to the exosphere, as a pure function of position, time and the solar
  (F10.7) and geomagnetic (Ap) indices the caller supplies. Verified
  against the reference distribution's test values and against the NRL
  Fortran original.

### Tier 1 — Moving Operational World (`no_std` + `alloc`, opt-in `moving`)

Enabled via `features = ["moving"]` (or individually via `surveillance`,
`airmass`, `aloft`, `notices`) on [`world`](crates/world).

Traffic and weather state belong to two dedicated libraries, and `world` is
the umbrella a control stack or a mission planner reaches them through. It
re-exports them unchanged, pinned by git revision in the workspace manifest,
and keeps no second copy of what they own:

- `world::surveillance` is
  [`surveillance-core`](https://github.com/luofang34/Surveillance): canonical
  `TrafficObservation` ingress, the single-writer `SurveillanceEngine`, fused
  `TrackSnapshot`s with per-field freshness and provenance, deltas and the
  exported schema.
- `world::airmass` is [`airmass-core`](https://github.com/luofang34/Airmass):
  `WeatherIngress`, the `WeatherStore` lifecycle (revisions, validity, expiry,
  source selection), typed reports and advisories, and immutable
  `WeatherSnapshotHandle`s.

What this workspace adds is queries and models over those snapshots:

- `world-traffic` (`world::traffic`): where a track is relative to ownship,
  the nearest tracks and those within a radius, linear closest approach, and
  traffic near a route through space and time. A track is placed with
  Surveillance's own projection and the bounds stamped on its snapshot, and is
  refused outside them. That placement is short-horizon dead reckoning, not a
  prediction of where traffic will be.
- `world-weather` (`world::weather`): what a station reports and which
  advisories apply at a point or along a route, read from one Airmass
  snapshot. An altitude band on a reference the caller gave no altitude for
  comes back unresolved, and station positions come from the caller through
  `StationLocator`.
- `world-aloft` (`world::aloft`): the physical model of the air —
  `SoundingProfile`s giving wind and temperature at a point
  (`world_atmosphere::AtmosphereModel`, `WindModel`) and along a route, and
  `CloudLayer`s. It is an atmosphere model, not a weather-product store.
- `world-notices` (`world::notices`): perishable operational restrictions —
  Temporary Flight Restrictions (`TfrNotice`), aerodrome/runway closure NOTAMs
  (`SurfaceClosureNotice`) and Special Use Airspace hot/cold activation
  schedules (`SuaActivationNotice`) referencing static `Flightbag` facility,
  runway and airspace keys without duplicating cycle-dated static charts.
- `MovingWorld` composes the engine, the store, the notices and the aloft
  model, and advances them from one `WorldInstant` carrying both the host's
  monotonic clock (track ageing) and civil UTC (validity). Each `MovingWorld`
  is one knowledge state with no process-global state, threads, sockets or
  system clocks, so a simulator holds one for physical truth and another for
  each vehicle's delayed radio or datalink belief.

CI builds both Tier 0 and Tier 1 for `thumbv7em-none-eabihf` and
`wasm32-unknown-unknown`. Anything that needs `std` or a transport — a radio
decoder, an internet ADS-B or weather provider, a simulator bridge, a replay
reader — is a host adapter that turns its source into a `TrafficObservation`
or a `WeatherIngress` and feeds the engine or the store. The models never
depend on one.

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

AGPL-3.0-or-later; see [LICENSE](LICENSE). `surveillance-core` and
`airmass-core` are MIT-licensed dependencies. The datasets keep their own
terms, recorded in each model's provenance: the World Magnetic Model is
in the public domain. NRLMSISE-00 is a work of the U.S. Government, and the C
implementation its parameters are taken from was placed in the public
domain by its author.
