//! How well a model's answer is known.

/// A one-standard-deviation uncertainty, in the unit of the quantity it
/// accompanies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uncertainty {
    /// One standard deviation between the model's value and a measurement.
    pub one_sigma: f64,
}
