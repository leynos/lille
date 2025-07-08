use lille::dbsp_circuit::{DbspCircuit, NewPosition, Position};

#[test]
fn entity_falls_due_to_gravity() {
    let mut circuit = DbspCircuit::new().unwrap();

    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        1,
    );
    circuit.step().unwrap();

    let output = circuit.new_position_out().consolidate();
    let results: Vec<NewPosition> = output.iter().map(|(p, _, _)| p.clone()).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity, 1);
    assert!(results[0].z.into_inner() < 1.0);
}
