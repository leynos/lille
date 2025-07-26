use lille::dbsp_circuit::DbspCircuit;

/// Create a new `DbspCircuit` for tests.
///
/// Panics if the circuit cannot be constructed.
#[allow(dead_code)]
pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

/// Convenience constructor for [`Position`] records used in tests.
#[allow(dead_code)]
pub fn pos(entity: i64, x: f64, y: f64, z: f64) -> lille::dbsp_circuit::Position {
    lille::dbsp_circuit::Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}
