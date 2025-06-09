use crate::actor::Actor;
use crate::entity::{BadGuy, CausesFear, Entity};
use crate::log;
use bevy::prelude::{ResMut, Resource};
use glam::Vec3;
use hashbrown::HashMap;
use std::time::{Duration, Instant};

const TICK_DURATION: Duration = Duration::from_millis(500);

/// Collection of entities and state for the legacy Lille world.
#[derive(Resource)]
pub struct GameWorld {
    pub entities: Vec<Entity>,
    pub actors: Vec<Actor>,
    pub bad_guys: Vec<BadGuy>,
    pub tick_count: u64,
    last_tick: Instant,
}

impl Default for GameWorld {
    fn default() -> Self {
        let mut world = Self {
            entities: Vec::new(),
            actors: Vec::new(),
            bad_guys: Vec::new(),
            tick_count: 0,
            last_tick: Instant::now(),
        };

        // Create initial actor
        world.actors.push(Actor::new(
            Vec3::new(125.0, 125.0, 0.0),
            Vec3::new(202.0, 200.0, 0.0),
            5.0,
            1.0,
        ));

        // Create BadGuy - positioned between actor and target
        world.bad_guys.push(BadGuy::new(150.0, 150.5, 0.0, 10.0));

        world
    }
}

impl GameWorld {
    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= TICK_DURATION {
            self.tick_count += 1;
            log!("\nTick {}", self.tick_count);

            // Collect threats and their positions
            let threats: Vec<&dyn CausesFear> = self
                .bad_guys
                .iter()
                .map(|bg| bg as &dyn CausesFear)
                .collect();
            let threat_positions: Vec<Vec3> =
                self.bad_guys.iter().map(|bg| bg.entity.position).collect();

            // Update all actors
            for actor in &mut self.actors {
                actor.update(&threats, &threat_positions);
            }

            self.last_tick = Instant::now();
        }
    }

    pub fn get_all_positions(&self) -> HashMap<(i32, i32, i32), u32> {
        self.entities
            .iter()
            .map(|e| {
                let p = e.position;
                (
                    (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32),
                    1,
                )
            })
            .chain(self.actors.iter().map(|a| {
                let p = a.entity.position;
                (
                    (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32),
                    1,
                )
            }))
            .chain(self.bad_guys.iter().map(|bg| {
                let p = bg.entity.position;
                (
                    (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32),
                    5,
                )
            }))
            .fold(HashMap::new(), |mut acc, (pos, count)| {
                *acc.entry(pos).or_insert(0) += count;
                acc
            })
    }
}

/// Bevy system wrapper that updates the [`GameWorld`] each frame.
pub fn update_world_system(mut world: ResMut<GameWorld>) {
    world.update();
}
