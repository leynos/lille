//! Cucumber test runner for BDD integration tests.
//!
//! This module sets up and executes Cucumber-based behaviour-driven
//! development tests for physics simulation scenarios.

use cucumber::World;
use steps::gravity_steps::PhysicsWorld;

mod steps;

#[tokio::main]
async fn main() {
    PhysicsWorld::run("tests/features").await;
}
