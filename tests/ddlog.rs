use glam::{Vec2, Vec3};
use lille::{ddlog_handle::DdlogEntity, DdlogHandle, UnitType};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

const DL_SRC: &str = include_str!("../src/lille.dl");

static REL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^\s*(?:input\s+(?:relation|stream)|output\s+relation|relation)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap()
});

fn capture_set(re: &Regex) -> HashSet<String> {
    re.captures_iter(DL_SRC)
        .filter_map(|c| {
            let text = c.get(0).unwrap().as_str();
            if text.trim_start().starts_with("//") {
                None
            } else {
                Some(c[1].to_string())
            }
        })
        .collect()
}

fn parsed_relations() -> HashSet<String> {
    capture_set(&REL_RE)
}

static CONST_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*const\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap());

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
fn ddlog_program_has_floor_height_rules() {
    let relations = parsed_relations();
    let constants = parsed_constants();

    for name in ["FloorHeightAt", "IsUnsupported", "IsStanding"] {
        assert!(relations.contains(name), "{} rule missing", name);
    }

    assert!(
        constants.contains("GRACE_DISTANCE"),
        "GRACE_DISTANCE constant missing"
    );

    for token in ["Velocity", "Force", "NewVelocity", "FrictionalDeceleration"] {
        assert!(
            relations.contains(token),
            "{} rule or relation missing",
            token
        );
    }
}
