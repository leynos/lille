use glam::Vec2;
use lille::{ddlog_handle::DdlogEntity, DdlogHandle, UnitType};

const DL_SRC: &str = include_str!("../src/lille.dl");

#[test]
fn ddlog_moves_towards_target() {
    let mut handle = DdlogHandle::default();
    handle.entities.insert(
        1,
        DdlogEntity {
            position: Vec2::ZERO,
            unit: UnitType::Civvy { fraidiness: 1.0 },
            health: 100,
            target: Some(Vec2::new(5.0, 0.0)),
        },
    );

    handle.infer_movement();
    let ent = handle.entities.get(&1).unwrap();
    assert!(
        ent.position.x > 0.1,
        "Entity did not advance towards target: x={}",
        ent.position.x
    );
}

#[test]
/// Tests that a civilian entity flees away from a nearby baddie after movement inference.
///
/// This test sets up a civilian at the origin with a target position and a baddie nearby. After calling `infer_movement()`, it asserts that the civilian's x-position is negative, confirming it moved away from the threat.
///
/// # Examples
///
/// ```
/// ddlog_flees_from_baddie();
/// ```
fn ddlog_flees_from_baddie() {
    let mut handle = DdlogHandle::default();
    handle.entities.insert(
        1,
        DdlogEntity {
            position: Vec2::ZERO,
            unit: UnitType::Civvy { fraidiness: 1.0 },
            health: 100,
            target: Some(Vec2::new(10.0, 0.0)),
        },
    );
    handle.entities.insert(
        2,
        DdlogEntity {
            position: Vec2::new(1.0, 0.0),
            unit: UnitType::Baddie { meanness: 1.0 },
            health: 100,
            target: None,
        },
    );

    handle.infer_movement();
    let ent = handle.entities.get(&1).unwrap();
    assert!(
        ent.position.x < -0.1,
        "Civvy did not flee from nearby baddie: x={}",
        ent.position.x
    );
}

#[test]
fn ddlog_program_has_floor_height_rules() {
    assert!(
        DL_SRC
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "FloorHeightAt"),
        "FloorHeightAt rule missing",
    );
    assert!(
        DL_SRC
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "IsUnsupported"),
        "IsUnsupported rule missing",
    );
    assert!(
        DL_SRC
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "IsStanding"),
        "IsStanding rule missing",
    );
    assert!(
        DL_SRC
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "GRACE_DISTANCE"),
        "GRACE_DISTANCE constant missing",
    );
}
