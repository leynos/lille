//! Health aggregation streams.
//!
//! This module wires together domain-specific helpers that derive
//! authoritative [`HealthDelta`] records and fall-damage events within the
//! DBSP circuit.

mod aggregate;
mod fall;
#[cfg(test)]
mod tests;

pub use aggregate::health_delta_stream;
pub use fall::fall_damage_stream;
