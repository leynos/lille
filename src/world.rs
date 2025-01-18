use std::time::{Instant, Duration};
use hashbrown::HashMap;
use crate::entity::{Entity, CausesFear, BadGuy};
use crate::actor::Actor;
use crate::log;

const TICK_DURATION: Duration = Duration::from_millis(500);

pub struct GameWorld {
    pub entities: Vec<Entity>,
    pub actors: Vec<Actor>,
    pub bad_guys: Vec<BadGuy>,
    pub tick_count: u64,
    last_tick: Instant,
}

impl GameWorld {
    pub fn new() -> Self {
        let mut world = Self {
            entities: Vec::new(),
            actors: Vec::new(),
            bad_guys: Vec::new(),
            tick_count: 0,
            last_tick: Instant::now(),
        };

        // Create initial actor
        world.actors.push(Actor::new(
            (199.0, 199.0, 0.0),  // starting position
            (202.0, 200.0, 0.0),    // target position
            5.0,                     // speed (units per tick)
            1.0,                // fraidiness factor
        ));

        // Create BadGuy - positioned between actor and target
        world.bad_guys.push(BadGuy {
            entity: Entity { position: (200.0, 199.5, 0.0) },
            meanness: 10.0,
        });

        world
    }

    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= TICK_DURATION {
            self.tick_count += 1;
            log!("\nTick {}", self.tick_count);
            
            // Collect threats and their positions
            let mut threats: Vec<&dyn CausesFear> = Vec::with_capacity(self.bad_guys.len());
            for bad_guy in &self.bad_guys {
                threats.push(bad_guy as &dyn CausesFear);
            }
            
            let threat_positions: Vec<(f32, f32, f32)> = self.bad_guys.iter()
                .map(|bg| bg.entity.position)
                .collect();
            
            // Update all actors
            for actor in &mut self.actors {
                actor.update(&threats, &threat_positions);
            }
            
            self.last_tick = Instant::now();
        }
    }

    pub fn get_all_positions(&self) -> HashMap<(i32, i32, i32), u32> {
        let mut positions = HashMap::new();
        
        // Add regular entities
        for entity in &self.entities {
            let grid_pos = (
                entity.position.0.round() as i32,
                entity.position.1.round() as i32,
                entity.position.2.round() as i32,
            );
            *positions.entry(grid_pos).or_insert(0) += 1;
        }
        
        // Add actors
        for actor in &self.actors {
            let grid_pos = (
                actor.entity.position.0.round() as i32,
                actor.entity.position.1.round() as i32,
                actor.entity.position.2.round() as i32,
            );
            *positions.entry(grid_pos).or_insert(0) += 1;
        }

        // Add bad guys (in red)
        for bad_guy in &self.bad_guys {
            let grid_pos = (
                bad_guy.entity.position.0.round() as i32,
                bad_guy.entity.position.1.round() as i32,
                bad_guy.entity.position.2.round() as i32,
            );
            // Use a large count to make them appear bright red
            *positions.entry(grid_pos).or_insert(0) += 5;
        }
        
        positions
    }
}