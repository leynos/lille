use crate::entity::{CausesFear, Entity};
use crate::log;
use glam::Vec3;

pub struct Actor {
    pub entity: Entity,
    pub target: Vec3,
    pub speed: f32,
    pub fraidiness_factor: f32,
}

impl Actor {
    pub fn new(position: Vec3, target: Vec3, speed: f32, fraidiness: f32) -> Self {
        log!(
            "Creating actor at {:?} targeting {:?} with speed {}",
            position,
            target,
            speed
        );
        Self {
            entity: Entity::new(position.x, position.y, position.z),
            target,
            speed,
            fraidiness_factor: fraidiness,
        }
    }

    fn calculate_fear_vector(
        &self,
        threats: &[&dyn CausesFear],
        threat_positions: &[Vec3],
    ) -> Vec3 {
        let mut fear_vector = Vec3::ZERO;

        for (threat, &threat_pos) in threats.iter().zip(threat_positions) {
            let to_actor = self.entity.position - threat_pos;
            let distance = to_actor.length();

            // Calculate fear radius
            let fear_radius = self.fraidiness_factor * threat.meanness_factor() * 2.0;
            let avoidance_radius = fear_radius.max(self.speed);

            log!(
                "Actor pos: {:?}, Threat pos: {:?}",
                self.entity.position,
                threat_pos
            );
            log!("To actor vector: {:?}, distance: {:.2}", to_actor, distance);
            log!(
                "Fear radius: {:.2}, Avoidance radius: {:.2}",
                fear_radius,
                avoidance_radius
            );

            // Calculate fear effect based on distance to threat
            // Start avoiding before we get too close
            if distance <= avoidance_radius {
                log!("Run away!");
                // Get perpendicular vector for sideways avoidance
                let perp = Vec3::new(-to_actor.y, to_actor.x, 0.0).normalize();

                log!("Perpendicular vector: {:?}", perp);

                // Scale fear effect based on how close we are
                let fear_scale = if distance < fear_radius {
                    // Strong avoidance when within fear radius
                    ((fear_radius - distance) / fear_radius).powi(2) * 5.0
                } else {
                    // Gentle avoidance when approaching fear radius
                    distance / avoidance_radius
                };

                log!("Fear scale: {:?}", fear_scale);

                // Combine direct avoidance with perpendicular movement
                fear_vector += to_actor.normalize() * fear_scale + perp * fear_scale;

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

        // Normalize target direction
        let target_direction = if to_target.length() > 0.0 {
            to_target.normalize()
        } else {
            Vec3::ZERO
        };

        // Normalize vectors
        let target_dir = target_direction.normalize_or_zero();
        let fear_dir = fear_vector.normalize_or_zero();

        // Calculate weights based on fear vector magnitude
        let fear_influence = fear_vector.length();
        let fear_weight = (fear_influence * 2.0).min(1.0); // Cap at 1.0
        let target_weight = 1.0 - fear_weight * 0.8; // Allow some target influence even when afraid

        // Combine vectors with weights, avoiding NaNs if the result is zero
        let move_vec = fear_dir * fear_weight + target_dir * target_weight;
        let final_direction = if move_vec.length_squared() > 0.0 {
            move_vec.normalize() * self.speed
        } else {
            Vec3::ZERO
        };

        log!(
            "Actor at {:?}, final direction: {:?}",
            self.entity.position,
            final_direction
        );

        if final_direction.length() > 0.0 {
            let new_pos = self.entity.position + final_direction;

            log!("Moving to {:?}", new_pos);
            self.entity.position = new_pos;
        }
    }
}
