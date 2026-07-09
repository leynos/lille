//! Behavioural streams deriving movement from fear and targets.
//!
//! These helpers merge fear levels with positions, transform targets into
//! movement decisions and apply those decisions to base positions.

mod apply;
mod decide;
mod fear;
#[cfg(test)]
mod tests;

pub use apply::apply_movement;
pub use decide::movement_decision_stream;
pub use fear::fear_level_stream;
