/// Entry point for custom DDlog helpers.
///
/// The generated DDlog crate expects these functions to be in scope when it is
/// built. Re-export them so the main crate and tests can call the same
/// implementations that DDlog uses.
pub use types__physics::{sign, vec_mag, vec_normalize};
