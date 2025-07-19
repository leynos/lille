use cucumber::World;
use steps::gravity_steps::PhysicsWorld;

mod steps {
    pub mod gravity_steps;
}

#[tokio::main]
async fn main() {
    PhysicsWorld::run("tests/features").await;
}
