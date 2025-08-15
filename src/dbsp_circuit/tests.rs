//! Tests for the DBSP circuit's grace distance.
use super::*;
use crate::GRACE_DISTANCE;
use rstest::rstest;

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

#[rstest]
#[case(10.0, 10.0)]
#[case(10.1, 10.0)]
#[case(10.5, 10.0)]
fn within_grace(#[case] z: f64, #[case] z_floor: f64) {
    let pf = make_pf(z, z_floor);
    assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
}

#[rstest]
fn unsupported() {
    let pf = make_pf(11.0, 10.0);
    assert!(pf.position.z.into_inner() > pf.z_floor.into_inner() + GRACE_DISTANCE);
}
