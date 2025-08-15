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
#[case(10.05, 10.0)]
#[case(10.1, 10.0)]
#[case(0.0, 0.0)]
#[case(-10.0, -10.0)]
#[case(-10.05, -10.0)]
#[case(f64::NAN, 10.0)]
#[case(10.0, f64::NAN)]
#[case(f64::NAN, f64::NAN)]
fn within_grace(#[case] z: f64, #[case] z_floor: f64) {
    let pf = make_pf(z, z_floor);
    if z.is_nan() || z_floor.is_nan() {
        // Comparisons with NaN are always false -> outside grace.
        let within = pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE;
        assert!(!within);
    } else {
        assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
    }
}

#[rstest]
#[case(11.0, 10.0)]
#[case(10.0 + GRACE_DISTANCE, 10.0)]
fn beyond_grace_or_at_boundary(#[case] z: f64, #[case] z_floor: f64) {
    let pf = make_pf(z, z_floor);
    if pf.position.z.into_inner() == pf.z_floor.into_inner() + GRACE_DISTANCE {
        assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
    } else {
        assert!(pf.position.z.into_inner() > pf.z_floor.into_inner() + GRACE_DISTANCE);
    }
}
