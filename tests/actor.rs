use glam::Vec3;
use lille::actor::Actor;
use lille::entity::{BadGuy, CausesFear};

#[test]
fn update_maintains_fear_radius() {
    // Create an actor that starts at origin
    let mut actor = Actor::new(
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0), // Target is 10 units along x-axis
        5.0,                       // Speed
        1.0,                       // Fraidiness factor
    );

    // Create a badguy at position (5, 0, 0) with meanness 1.0
    let badguy = BadGuy::new(5.0, 0.0, 0.0, 1.0);

    // The fear radius should be: fraidiness (1.0) * meanness (1.0) * 2.0 = 2.0 units
    let fear_radius = 2.0;

    // Update the actor's position
    actor.update(&[&badguy], &[badguy.entity.position]);

    // Calculate distances after update
    let distance_to_badguy = (actor.entity.position - badguy.entity.position).length();

    // Assert that the actor maintains at least the fear radius distance
    assert!(
        distance_to_badguy >= fear_radius,
        "Actor is too close to badguy. Distance: {}, Required minimum: {}",
        distance_to_badguy,
        fear_radius
    );

    // Verify the actor has moved around the badguy (should have some Y displacement)
    assert!(
        actor.entity.position.y != 0.0,
        "Actor should move around the badguy, not just stop. Position: {:?}",
        actor.entity.position
    );
}

#[test]
fn avoids_multiple_threats() {
    // Actor heading towards +X axis
    let mut actor = Actor::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 5.0, 1.0);

    // Two threats close to the actor
    let bad1 = BadGuy::new(4.0, 0.0, 0.0, 1.0);
    let bad2 = BadGuy::new(5.0, 0.5, 0.0, 1.0);
    let threats = [&bad1 as &dyn CausesFear, &bad2 as &dyn CausesFear];
    let positions = [bad1.entity.position, bad2.entity.position];

    actor.update(&threats, &positions);

    // Fear radius = 2.0
    for pos in positions {
        let dist = (actor.entity.position - pos).length();
        assert!(dist >= 2.0, "Actor too close to threat: {}", dist);
    }

    // Should have deviated from a straight line towards the target
    assert!(actor.entity.position.y.abs() > 0.0);
}

#[test]
fn walks_towards_target() {
    // Create an actor at origin with target 5 units away on x-axis
    let mut actor = Actor::new(
        Vec3::ZERO,
        Vec3::new(5.0, 0.0, 0.0),
        5.0, // Speed
        1.0, // Fraidiness factor (irrelevant for this test)
    );

    // Update position (no threats)
    actor.update(&[], &[]);

    // Should have moved by its speed towards target
    assert!(
        (actor.entity.position.x - actor.speed).abs() < f32::EPSILON,
        "Actor should move exactly {} units towards target, but moved to {:?}",
        actor.speed,
        actor.entity.position
    );

    // Y and Z coordinates should remain unchanged
    assert!(
        actor.entity.position.y.abs() < f32::EPSILON
            && actor.entity.position.z.abs() < f32::EPSILON,
        "Actor should only move along X axis, but position is {:?}",
        actor.entity.position
    );
}

#[test]
fn stationary_when_at_target() {
    let target_pos = Vec3::new(3.0, 2.0, 1.0);

    // Create actor already at target position
    let mut actor = Actor::new(
        target_pos, target_pos, 1.0, // Speed (irrelevant since we shouldn't move)
        1.0, // Fraidiness factor (irrelevant for this test)
    );

    // Store initial position
    let initial_pos = actor.entity.position;

    // Update position (no threats)
    actor.update(&[], &[]);

    // Position should remain unchanged
    assert!(
        (actor.entity.position - initial_pos).length() < f32::EPSILON,
        "Actor should not move when at target. Started at {:?}, moved to {:?}",
        initial_pos,
        actor.entity.position
    );
}
