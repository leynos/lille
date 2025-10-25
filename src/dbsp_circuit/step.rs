//! Convenience wrappers for driving the DBSP circuit in tests and callers.

use super::DbspCircuit;

/// Advances the circuit by one tick, panicking on error.
///
/// This wrapper is convenient for tests but will abort the current task when
/// evaluation fails; prefer [`try_step`] when you need to handle errors.
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
/// Like [`step`], this wrapper panics on error. Use [`try_step`] if the caller
/// needs to propagate failures instead of aborting execution.
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
