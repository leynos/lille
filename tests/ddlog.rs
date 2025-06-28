//! Integration tests for the DDlog interface.
//! Ensures that relations defined in the Datalog schema match expectations.
#![cfg(feature = "ddlog")]
use bevy::prelude::*;
use glam::Vec2;
use lille::{
    apply_ddlog_deltas_system, cache_state_for_ddlog_system, init_ddlog_system, DdlogHandle,
    DdlogId, Health, Target, UnitType,
};
use once_cell::sync::Lazy;
use regex::Regex;
use rstest::{fixture, rstest};
use serial_test::serial;
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

#[fixture]
fn ddlog_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, init_ddlog_system);
    app.add_systems(
        Update,
        (cache_state_for_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    app
}

#[rstest]
#[serial]
fn ddlog_moves_towards_target(ddlog_app: App) {
    let mut app = ddlog_app;
    let _e = app
        .world
        .spawn((
            DdlogId(1),
            Transform::default(),
            Health(100),
            UnitType::Civvy { fraidiness: 1.0 },
            Target(Vec2::new(5.0, 0.0)),
        ))
        .id();

    app.update();
    let handle = app.world.resource::<DdlogHandle>();
    let ent = handle.entities.get(&1).unwrap();
    assert!(
        ent.position.x > 0.1,
        "Entity did not advance towards target: x={}",
        ent.position.x
    );
}

/// Tests that a civilian entity flees away from a nearby baddie after movement inference.
///
/// This test sets up a civilian at the origin with a target position and a baddie nearby. After calling `step()`, it asserts that the civilian's x-position is negative, confirming it moved away from the threat.
///
/// # Examples
///
/// ```
/// ddlog_flees_from_baddie();
/// ```
#[rstest]
#[serial]
fn ddlog_flees_from_baddie(ddlog_app: App) {
    let mut app = ddlog_app;
    let _civvy = app
        .world
        .spawn((
            DdlogId(1),
            Transform::default(),
            Health(100),
            UnitType::Civvy { fraidiness: 1.0 },
            Target(Vec2::new(10.0, 0.0)),
        ))
        .id();
    app.world.spawn((
        DdlogId(2),
        Transform::from_xyz(1.0, 0.0, 0.0),
        Health(100),
        UnitType::Baddie { meanness: 1.0 },
    ));

    app.update();
    let handle = app.world.resource::<DdlogHandle>();
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
        DL_SRC.contains("not Mass(e, _)") && DL_SRC.contains("var mass = default_mass()"),
        "default mass rule missing",
    );
}
