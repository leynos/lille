//! Integration tests for the DDlog interface.
//! Ensures that relations defined in the Datalog schema match expectations.
use glam::{Vec2, Vec3};
use lille::{ddlog_handle::DdlogEntity, DdlogHandle, UnitType};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

const DL_SRC: &str = concat!(
    include_str!("../src/ddlog/lille.dl"),
    include_str!("../src/ddlog/geometry.dl"),
    include_str!("../src/ddlog/entity_state.dl"),
    include_str!("../src/ddlog/physics.dl")
);
const CONSTANTS_SRC: &str = include_str!("../src/ddlog/constants.dl");

static REL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^\s*(?:input\s+(?:relation|stream)|output\s+relation|relation)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap()
});

fn capture_set(re: &Regex) -> HashSet<String> {
    [DL_SRC, CONSTANTS_SRC]
        .iter()
        .flat_map(|src| {
            re.captures_iter(src).filter_map(|c| {
                let text = c.get(0).unwrap().as_str();
                if text.trim_start().starts_with("//") {
                    None
                } else {
                    Some(c[1].to_string())
                }
            })
        })
        .collect()
}

fn parsed_relations() -> HashSet<String> {
    capture_set(&REL_RE)
}

static CONST_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap());

fn parsed_constants() -> HashSet<String> {
    capture_set(&CONST_RE)
}

#[test]
fn ddlog_moves_towards_target() {
    let mut handle = DdlogHandle::default();
    handle.entities.insert(
        1,
        DdlogEntity {
            position: Vec3::ZERO,
            unit: UnitType::Civvy { fraidiness: 1.0 },
            health: 100,
            target: Some(Vec2::new(5.0, 0.0)),
        },
    );

    handle.step();
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
/// This test sets up a civilian at the origin with a target position and a baddie nearby. After calling `step()`, it asserts that the civilian's x-position is negative, confirming it moved away from the threat.
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
            position: Vec3::ZERO,
            unit: UnitType::Civvy { fraidiness: 1.0 },
            health: 100,
            target: Some(Vec2::new(10.0, 0.0)),
        },
    );
    handle.entities.insert(
        2,
        DdlogEntity {
            position: Vec3::new(1.0, 0.0, 0.0),
            unit: UnitType::Baddie { meanness: 1.0 },
            health: 100,
            target: None,
        },
    );

    handle.step();
    let ent = handle.entities.get(&1).unwrap();
    assert!(
        ent.position.x < -0.1,
        "Civvy did not flee from nearby baddie: x={}",
        ent.position.x
    );
}

#[test]
/// Verifies that the Datalog source includes required rules and relations for floor height and movement.
///
/// Asserts that the source string contains the tokens "FloorHeightAt", "IsUnsupported", "IsStanding", "GRACE_DISTANCE", "Velocity", "Force", "NewVelocity", and "FrictionalDeceleration".
///
/// # Panics
///
/// Panics if any of the required rules or relations are missing from the source.
///
/// # Examples
///
/// ```
/// ddlog_program_has_floor_height_rules(); // Should not panic if all rules are present
/// ```
fn ddlog_program_has_floor_height_rules() {
    let relations = parsed_relations();
    let constants = parsed_constants();

    for name in ["FloorHeightAt", "IsUnsupported", "IsStanding"] {
        assert!(relations.contains(name), "{name} rule missing");
    }

    assert!(
        constants.contains("grace_distance"),
        "grace_distance constant missing"
    );

    assert!(
        constants.contains("default_mass"),
        "default_mass constant missing"
    );

    for token in [
        "Velocity",
        "Mass",
        "Force",
        "NewVelocity",
        "FrictionalDeceleration",
    ] {
        assert!(
            relations.contains(token),
            "{token} rule or relation missing"
        );
    }

    assert!(DL_SRC.contains("mass > 0"), "mass positivity check missing");
    assert!(
        DL_SRC.contains("mass = default_mass()") && !DL_SRC.contains("not Mass"),
        "default mass rule missing"
    );
}
