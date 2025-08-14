//! Tests for the DBSP circuit's grace distance.
use super::*;

fn make_pf(z: f64, z_floor: f64) -> PositionFloor {
    PositionFloor {
        position: Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: z.into(),
        },
        z_floor: z_floor.into(),
    }
}

#[test]
fn test_grace_distance_on_flat_surface() {
    let pf = make_pf(10.0, 10.0);
    assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
}

#[test]
fn test_grace_distance_on_slope() {
    let pf = make_pf(10.1, 10.0);
    assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
}

#[test]
fn test_grace_distance_fast_moving_entity() {
    let pf = make_pf(10.5, 10.0);
    let within_grace = pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE;
    assert_eq!(within_grace, 10.5 <= 10.0 + GRACE_DISTANCE);
}

#[test]
fn test_grace_distance_unsupported() {
    let pf = make_pf(11.0, 10.0);
    assert!(pf.position.z.into_inner() > pf.z_floor.into_inner() + GRACE_DISTANCE);
}
