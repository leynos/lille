//! Cucumber test runner for BDD integration tests.
//!
//! This module sets up and executes Cucumber-based behaviour-driven
//! development tests for physics simulation scenarios.

use cucumber::World;
use steps::gravity_steps::PhysicsWorld;

mod steps;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let features = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/features");
    let features = features.to_str().expect("valid features path");
    PhysicsWorld::cucumber()
        .filter_run_and_exit(features, |_, _, sc| sc.tags.iter().any(|t| t == "serial"))
        .await;
}
