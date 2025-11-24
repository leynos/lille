#![cfg_attr(
    feature = "render",
    doc = "Integration tests covering the world-spawning Bevy system."
)]
#![cfg_attr(
    not(feature = "render"),
    doc = "Integration tests require the `render` feature."
)]
#![cfg(feature = "render")]
//! Unit tests for the world-spawning system.
//! Verifies entity counts and component assignments after system execution.
use bevy::prelude::*;
use lille::spawn_world_system;
use lille::{DdlogId, Health, Target, UnitType};
use rstest::{fixture, rstest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CameraInfo {
    None,
    Orthographic,
}

#[derive(Clone, Debug)]
struct SpawnExpectation {
    label: &'static str,
    ddlog_id: Option<i64>,
    unit: Option<UnitType>,
    translation: Vec3,
    has_target: bool,
    has_health: bool,
    expects_camera: bool,
    expected_visibility: Option<Visibility>,
}

#[derive(Clone, Debug)]
struct ObservedEntity {
    ddlog_id: Option<i64>,
    unit: Option<UnitType>,
    translation: Vec3,
    has_target: bool,
    has_health: bool,
    health: Option<Health>,
    visibility: Option<Visibility>,
    camera: CameraInfo,
}

#[fixture]
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, spawn_world_system);
    app.update();
    app
}

fn collect_observations(app: &mut App) -> Vec<ObservedEntity> {
    let mut query = app.world_mut().query::<(
        Option<&DdlogId>,
        Option<&UnitType>,
        &Transform,
        Option<&Health>,
        Option<&Target>,
        Option<&Visibility>,
        Option<&Camera2d>,
        Option<&Projection>,
    )>();

    query
        .iter(app.world())
        .map(
            |(ddlog_id, unit, transform, health, target, visibility, camera2d, projection)| {
                let camera = if camera2d.is_some()
                    && projection.is_some_and(|proj| matches!(proj, Projection::Orthographic(_)))
                {
                    CameraInfo::Orthographic
                } else {
                    CameraInfo::None
                };
                ObservedEntity {
                    ddlog_id: ddlog_id.map(|id| id.0),
                    unit: unit.cloned(),
                    translation: transform.translation,
                    has_target: target.is_some(),
                    has_health: health.is_some(),
                    health: health.cloned(),
                    visibility: visibility.copied(),
                    camera,
                }
            },
        )
        .collect()
}

fn matches_expected(observed: &ObservedEntity, expected: &SpawnExpectation) -> bool {
    match expected.ddlog_id {
        Some(id) => observed.ddlog_id == Some(id),
        None if expected.expects_camera => observed.camera == CameraInfo::Orthographic,
        None => false,
    }
}

fn assert_translation(observed: &ObservedEntity, expected: &SpawnExpectation) {
    assert!(
        observed
            .translation
            .abs_diff_eq(expected.translation, f32::EPSILON),
        "{} should spawn at {:?} but was {:?}",
        expected.label,
        expected.translation,
        observed.translation
    );
}

fn assert_unit(observed: &ObservedEntity, expected: &SpawnExpectation) {
    match expected.unit.as_ref() {
        Some(UnitType::Civvy { fraidiness }) => match observed.unit.as_ref() {
            Some(UnitType::Civvy {
                fraidiness: observed_fraidiness,
            }) => {
                assert!(
                    (observed_fraidiness - fraidiness).abs() < f32::EPSILON,
                    "Civvy fraidiness expected {fraidiness} but was {observed_fraidiness}"
                );
                assert!(
                    observed.has_target,
                    "{} should include a Target component",
                    expected.label
                );
            }
            _ => panic!("{} should be a Civvy", expected.label),
        },
        Some(UnitType::Baddie { meanness }) => match observed.unit.as_ref() {
            Some(UnitType::Baddie {
                meanness: observed_meanness,
            }) => {
                assert!(
                    (observed_meanness - meanness).abs() < f32::EPSILON,
                    "Baddie meanness expected {meanness} but was {observed_meanness}"
                );
                assert!(
                    !observed.has_target,
                    "{} should not carry a Target component",
                    expected.label
                );
            }
            _ => panic!("{} should be a Baddie", expected.label),
        },
        None => assert!(
            observed.unit.is_none(),
            "{} should not carry a UnitType",
            expected.label
        ),
    }
}

fn assert_components(observed: &ObservedEntity, expected: &SpawnExpectation) {
    if expected.has_target {
        assert!(
            observed.has_target,
            "{} should include a Target component",
            expected.label
        );
    } else {
        assert!(
            !observed.has_target,
            "{} should not carry a Target component",
            expected.label
        );
    }

    if expected.has_health {
        assert!(
            observed.has_health,
            "{} should retain a Health component",
            expected.label
        );
    } else {
        assert!(
            !observed.has_health,
            "{} should not include a Health component",
            expected.label
        );
    }
    assert_positive_health_if_expected(observed, expected);
}

fn assert_visibility(observed: &ObservedEntity, expected: &SpawnExpectation) {
    if let Some(expected_visibility) = expected.expected_visibility {
        assert_eq!(
            observed.visibility,
            Some(expected_visibility),
            "{} should have {:?}",
            expected.label,
            expected_visibility
        );
    }
}

fn assert_camera(observed: &ObservedEntity, expected: &SpawnExpectation) {
    if expected.expects_camera {
        assert_eq!(
            observed.camera,
            CameraInfo::Orthographic,
            "{} should include the Camera2d marker",
            expected.label
        );
        assert!(
            observed.translation.z > 0.0,
            "{} camera should be positioned above the world",
            expected.label
        );
    } else {
        assert_eq!(
            observed.camera,
            CameraInfo::None,
            "{} should not be a camera entity",
            expected.label
        );
    }
}

fn assert_matches(observed: &ObservedEntity, expected: &SpawnExpectation) {
    assert_translation(observed, expected);
    assert_unit(observed, expected);
    assert_components(observed, expected);
    assert_visibility(observed, expected);
    assert_camera(observed, expected);
}

fn assert_positive_health_if_expected(observed: &ObservedEntity, expected: &SpawnExpectation) {
    if !expected.has_health {
        return;
    }
    let health = observed
        .health
        .as_ref()
        .unwrap_or_else(|| panic!("{} should include a Health component", expected.label));
    assert!(
        health.current > 0,
        "{} should spawn with positive health but had {}",
        expected.label,
        health.current
    );
}

/// Tests that the `spawn_world_system` correctly spawns Civvy, Baddie, static,
/// and camera entities with expected properties using Bevy 0.15 required
/// components.
#[rstest]
fn spawns_world_entities(mut app: App) {
    let expectations = vec![
        SpawnExpectation {
            label: "static landmark",
            ddlog_id: Some(1),
            unit: None,
            translation: Vec3::new(50.0, 50.0, 0.0),
            has_target: false,
            has_health: false,
            expects_camera: false,
            expected_visibility: Some(Visibility::Visible),
        },
        SpawnExpectation {
            label: "civilian",
            ddlog_id: Some(2),
            unit: Some(UnitType::Civvy { fraidiness: 1.0 }),
            translation: Vec3::new(125.0, 125.0, 0.0),
            has_target: true,
            has_health: true,
            expects_camera: false,
            expected_visibility: Some(Visibility::Visible),
        },
        SpawnExpectation {
            label: "baddie",
            ddlog_id: Some(3),
            unit: Some(UnitType::Baddie { meanness: 10.0 }),
            translation: Vec3::new(150.0, 150.5, 0.0),
            has_target: false,
            has_health: true,
            expects_camera: false,
            expected_visibility: Some(Visibility::Visible),
        },
        SpawnExpectation {
            label: "camera",
            ddlog_id: None,
            unit: None,
            translation: Vec3::new(0.0, 0.0, 999.9),
            has_target: false,
            has_health: false,
            expects_camera: true,
            expected_visibility: Some(Visibility::Visible),
        },
    ];

    let mut observed = collect_observations(&mut app);
    assert_eq!(
        observed.len(),
        expectations.len(),
        "Expected {} entities, found {}",
        expectations.len(),
        observed.len()
    );

    for expected in &expectations {
        let index = observed
            .iter()
            .position(|obs| matches_expected(obs, expected))
            .unwrap_or_else(|| panic!("Missing {}", expected.label));
        let candidate = observed.swap_remove(index);
        assert_matches(&candidate, expected);
    }
}
