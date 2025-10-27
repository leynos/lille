//! Convenience wrappers for driving the DBSP circuit in tests and callers.

use super::DbspCircuit;

/// Advances the circuit by one tick, panicking on error.
///
/// This wrapper is convenient for tests but will abort the current task when
/// evaluation fails; prefer [`try_step`] when you need to handle errors.
///
/// # Examples
/// ```rust,no_run
/// use lille::dbsp_circuit::{DbspCircuit, step};
///
/// # fn demo() -> Result<(), dbsp::Error> {
/// let mut circuit = DbspCircuit::new()?; // push inputs as needed
/// step(&mut circuit); // panics if the underlying evaluation fails
/// # Ok(())
/// # }
/// ```
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
/// # Examples
/// ```rust,no_run
/// use lille::dbsp_circuit::{DbspCircuit, step_named};
///
/// # fn demo() -> Result<(), dbsp::Error> {
/// let mut circuit = DbspCircuit::new()?; // feed frame inputs here
/// step_named(&mut circuit, "physics update"); // panic message includes context
/// # Ok(())
/// # }
/// ```
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
/// # Examples
/// ```rust,no_run
/// use lille::dbsp_circuit::{DbspCircuit, try_step};
///
/// # fn simulate() -> Result<(), dbsp::Error> {
/// let mut circuit = DbspCircuit::new()?; // input preparation elided
/// try_step(&mut circuit)?; // propagate evaluation failure to the caller
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Propagates any error emitted by [`DbspCircuit::step`].
pub fn try_step(circuit: &mut DbspCircuit) -> Result<(), dbsp::Error> {
    circuit.step()
}
