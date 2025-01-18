use crate::entity::{Entity, CausesFear};
use crate::log;

pub struct Actor {
    pub entity: Entity,
    pub target: (f32, f32, f32),
    pub speed: f32,
    pub fraidiness_factor: f32,
}

impl Actor {
    pub fn new(position: (f32, f32, f32), target: (f32, f32, f32), speed: f32, fraidiness: f32) -> Self {
        log!("Creating actor at {:?} targeting {:?} with speed {}", position, target, speed);
        Self {
            entity: Entity { position },
            target,
            speed,
            fraidiness_factor: fraidiness,
        }
    }

    fn calculate_fear_vector(&self, threats: &[&dyn CausesFear], threat_positions: &[(f32, f32, f32)]) -> (f32, f32, f32) {
        let mut fear_x = 0.0;
        let mut fear_y = 0.0;
        let mut fear_z = 0.0;

        for (threat, &pos) in threats.iter().zip(threat_positions) {
            let (tx, ty, tz) = pos;
            let (x, y, z) = self.entity.position;
            
            // Calculate distance to threat
            let dx = x - tx;
            let dy = y - ty;
            let dz = z - tz;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            
            // Calculate fear radius
            let fear_radius = self.fraidiness_factor * threat.meanness_factor() * 2.0;
            
            log!("Distance to threat: {:.2}, Fear radius: {:.2}", distance, fear_radius);
            
            // If within fear radius, add to fear vector
            if distance < fear_radius {
                // Normalize direction and scale by how close we are to the threat
                let fear_scale = (fear_radius - distance) / fear_radius;
                fear_x += dx / distance * fear_scale;
                fear_y += dy / distance * fear_scale;
                fear_z += dz / distance * fear_scale;
                
                log!("Fear vector: ({:.2}, {:.2}, {:.2})", fear_x, fear_y, fear_z);
            }
        }
        
        (fear_x, fear_y, fear_z)
    }

    pub fn update(&mut self, threats: &[&dyn CausesFear], threat_positions: &[(f32, f32, f32)]) {
        let (x, y, z) = self.entity.position;
        let (target_x, target_y, target_z) = self.target;
        
        // Calculate direction vector to target
        let dx = target_x - x;
        let dy = target_y - y;
        let dz = target_z - z;
        
        // Get fear vector
        let (fear_x, fear_y, fear_z) = self.calculate_fear_vector(threats, threat_positions);
        
        // Combine target direction with fear vector
        let final_dx = dx + fear_x * self.speed * 2.0; // Fear is a stronger motivator
        let final_dy = dy + fear_y * self.speed * 2.0;
        let final_dz = dz + fear_z * self.speed * 2.0;
        
        // Calculate total distance for final movement vector
        let distance = (final_dx * final_dx + final_dy * final_dy + final_dz * final_dz).sqrt();
        
        log!("Actor at ({:.2}, {:.2}, {:.2}) distance to move: {:.2}", x, y, z, distance);
        
        if distance > 0.0 {
            // If we're closer than our max movement distance, just move to the target
            let movement_distance = distance.min(self.speed);
            
            // Calculate actual movement
            let move_x = final_dx / distance * movement_distance;
            let move_y = final_dy / distance * movement_distance;
            let move_z = final_dz / distance * movement_distance;
            
            let new_pos = (
                x + move_x,
                y + move_y,
                z + move_z,
            );
            
            log!("Moving by ({:.2}, {:.2}, {:.2}) to {:?}", 
                move_x, move_y, move_z, new_pos);
            
            self.entity.position = new_pos;
        }
    }
}