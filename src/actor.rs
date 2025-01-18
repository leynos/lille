use glam::Vec3;
use crate::entity::{Entity, CausesFear};
use crate::log;

pub struct Actor {
    pub entity: Entity,
    pub target: Vec3,
    pub speed: f32,
    pub fraidiness_factor: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::BadGuy;

    #[test]
    fn update_maintains_fear_radius() {
        // Create an actor that starts at origin
        let mut actor = Actor::new(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0), // Target is 10 units along x-axis
            5.0,  // Speed
            1.0,  // Fraidiness factor
        );

        // Create a badguy at position (5, 0, 0) with meanness 1.0
        let badguy = BadGuy::new(5.0, 0.0, 0.0, 1.0);
        
        // The fear radius should be: fraidiness (1.0) * meanness (1.0) * 2.0 = 2.0 units
        let fear_radius = 2.0;

        // Update the actor's position
        actor.update(&[&badguy], &[badguy.entity.position]);

        // Calculate distance between actor and badguy after update
        let distance = (actor.entity.position - badguy.entity.position).length();

        // Assert that the actor maintains at least the fear radius distance
        assert!(
            distance >= fear_radius,
            "Actor is too close to badguy. Distance: {}, Required minimum: {}",
            distance,
            fear_radius
        );
    }
}

impl Actor {
    pub fn new(position: Vec3, target: Vec3, speed: f32, fraidiness: f32) -> Self {
        log!("Creating actor at {:?} targeting {:?} with speed {}", position, target, speed);
        Self {
            entity: Entity::new(position.x, position.y, position.z),
            target,
            speed,
            fraidiness_factor: fraidiness,
        }
    }

    fn calculate_fear_vector(&self, threats: &[&dyn CausesFear], threat_positions: &[Vec3]) -> Vec3 {
        let mut fear_vector = Vec3::ZERO;

        for (threat, &threat_pos) in threats.iter().zip(threat_positions) {
            let to_actor = self.entity.position - threat_pos;
            let distance = to_actor.length();
            
            // Calculate fear radius
            let fear_radius = self.fraidiness_factor * threat.meanness_factor() * 2.0;
            
            log!("Distance to threat: {:.2}, Fear radius: {:.2}", distance, fear_radius);
            
            // If within fear radius, add to fear vector
            if distance < fear_radius {
                // Normalize direction and scale by how close we are to the threat
                let fear_scale = (fear_radius - distance) / fear_radius;
                fear_vector += to_actor.normalize() * fear_scale;
                
                log!("Fear vector: {:?}", fear_vector);
            }
        }
        
        fear_vector
    }

    pub fn update(&mut self, threats: &[&dyn CausesFear], threat_positions: &[Vec3]) {
        // Calculate direction vector to target
        let to_target = self.target - self.entity.position;
        
        // Get fear vector
        let fear_vector = self.calculate_fear_vector(threats, threat_positions);
        
        // Combine target direction with fear vector (fear is a stronger motivator)
        let final_direction = to_target + fear_vector * self.speed * 2.0;
        
        // Calculate total distance for final movement vector
        let distance = final_direction.length();
        
        log!("Actor at {:?} distance to move: {:.2}", self.entity.position, distance);
        
        if distance > 0.0 {
            // If we're closer than our max movement distance, just move to the target
            let movement_distance = distance.min(self.speed);
            
            // Calculate actual movement
            let movement = final_direction.normalize() * movement_distance;
            
            let new_pos = self.entity.position + movement;
            
            log!("Moving by {:?} to {:?}", movement, new_pos);
            
            self.entity.position = new_pos;
        }
    }
}