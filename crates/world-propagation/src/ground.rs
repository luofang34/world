//! Electrical constants of the ground a wave travels over.

use crate::error::{PropagationError, non_negative, positive};

/// Electrical constants of the Earth's surface along the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    /// Effective relative permittivity ε.
    pub relative_permittivity: f64,
    /// Effective conductivity σ, S/m.
    pub conductivity_s_m: f64,
}

impl Ground {
    /// Average land, ε = 22 and σ = 0.003 S/m: the values P.452 and
    /// P.1812 apply to the land part of a path in the P.526 method.
    pub const LAND: Ground = Ground {
        relative_permittivity: 22.0,
        conductivity_s_m: 0.003,
    };

    /// Sea water, ε = 80 and σ = 5 S/m, as P.452 and P.1812 apply them.
    pub const SEA: Ground = Ground {
        relative_permittivity: 80.0,
        conductivity_s_m: 5.0,
    };
}

/// Checks the constants are physical: ε > 0 and σ ≥ 0, both finite.
pub(crate) fn validate_ground(ground: Ground) -> Result<(), PropagationError> {
    positive("relative_permittivity", ground.relative_permittivity)?;
    non_negative("conductivity_s_m", ground.conductivity_s_m)?;
    Ok(())
}
