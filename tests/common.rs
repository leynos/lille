use lille::dbsp_circuit::DbspCircuit;

/// Create a new `DbspCircuit` for tests.
///
/// Panics if the circuit cannot be constructed.
pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}
