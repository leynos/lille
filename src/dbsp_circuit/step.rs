//! Convenience wrappers for driving the DBSP circuit in tests and callers.

use super::DbspCircuit;

/// Advances the circuit by one tick, panicking on error.
///
/// # Panics
/// Panics when [`DbspCircuit::step`] returns an error.
#[track_caller]
pub fn step(circuit: &mut DbspCircuit) {
    if let Err(err) = circuit.step() {
        panic!("DbspCircuit::step failed: {err}");
    }
}

/// Advances the circuit and annotates failures with contextual information.
///
/// # Panics
/// Panics when [`DbspCircuit::step`] returns an error for the provided `ctx`.
#[track_caller]
pub fn step_named(circuit: &mut DbspCircuit, ctx: &str) {
    if let Err(err) = circuit.step() {
        panic!("DbspCircuit::step failed: {ctx}: {err}");
    }
}

/// Attempts to advance the circuit by one tick, returning any evaluation error.
///
/// # Errors
/// Propagates any error emitted by [`DbspCircuit::step`].
pub fn try_step(circuit: &mut DbspCircuit) -> Result<(), dbsp::Error> {
    circuit.step()
}
