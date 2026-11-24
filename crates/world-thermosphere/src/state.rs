//! What an upper-atmosphere model answers.

/// Number densities of the species a model resolves, per cubic meter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberDensities {
    /// Helium, He.
    pub helium_m3: f64,
    /// Atomic oxygen, O.
    pub atomic_oxygen_m3: f64,
    /// Molecular nitrogen, N2.
    pub molecular_nitrogen_m3: f64,
    /// Molecular oxygen, O2.
    pub molecular_oxygen_m3: f64,
    /// Argon, Ar.
    pub argon_m3: f64,
    /// Atomic hydrogen, H.
    pub atomic_hydrogen_m3: f64,
    /// Atomic nitrogen, N.
    pub atomic_nitrogen_m3: f64,
    /// Anomalous oxygen: hot atomic and ionized oxygen, which the drag
    /// data the model was fitted to cannot separate. Appreciable only
    /// above about 500 km.
    pub anomalous_oxygen_m3: f64,
}

/// The state of the upper atmosphere at one point and time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermosphereState {
    /// Number density of each species.
    pub number_density: NumberDensities,
    /// Total mass density of the thermal species, kg/m³: every species
    /// but anomalous oxygen (the reference `GTD7` output `D(6)`).
    pub mass_density_kg_m3: f64,
    /// Effective total mass density for drag, kg/m³: the thermal species
    /// plus anomalous oxygen (the reference `GTD7D` output `D(6)`). It
    /// differs from [`Self::mass_density_kg_m3`] only at the altitudes
    /// where anomalous oxygen is appreciable.
    pub drag_mass_density_kg_m3: f64,
    /// Exospheric temperature, kelvin.
    pub exospheric_temperature_k: f64,
    /// Temperature at the requested altitude, kelvin.
    pub temperature_k: f64,
}
