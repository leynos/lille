use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{Block, DdlogId, Velocity as VelocityComp};
use crate::dbsp_circuit::{DbspCircuit, Position, Velocity};

pub struct DbspState {
    circuit: DbspCircuit,
}

impl Default for DbspState {
    fn default() -> Self {
        Self {
            circuit: DbspCircuit::new().expect("failed to build DBSP circuit"),
        }
    }
}

pub fn init_dbsp_system(world: &mut World) {
    world.insert_non_send_resource(DbspState::default());
}

pub fn cache_state_for_dbsp_system(
    state: NonSendMut<DbspState>,
    entity_query: Query<(&DdlogId, &Transform, Option<&VelocityComp>)>,
    block_query: Query<&Block>,
) {
    for block in &block_query {
        state.circuit.block_in().push(block.clone(), 1);
    }

    for (id, transform, vel) in &entity_query {
        state.circuit.position_in().push(
            Position {
                entity: id.0,
                x: (transform.translation.x as f64).into(),
                y: (transform.translation.y as f64).into(),
                z: (transform.translation.z as f64).into(),
            },
            1,
        );

        let v = vel.map(|v| (v.vx, v.vy, v.vz)).unwrap_or_default();
        state.circuit.velocity_in().push(
            Velocity {
                entity: id.0,
                vx: (v.0 as f64).into(),
                vy: (v.1 as f64).into(),
                vz: (v.2 as f64).into(),
            },
            1,
        );
    }
}

pub fn apply_dbsp_outputs_system(
    mut state: NonSendMut<DbspState>,
    mut query: Query<(Entity, &DdlogId, &mut Transform, Option<&mut VelocityComp>)>,
) {
    state.circuit.step().expect("DBSP step failed");

    let id_map: HashMap<i64, Entity> = query.iter().map(|(e, id, _, _)| (id.0, e)).collect();

    let positions = state.circuit.new_position_out().consolidate();
    for (pos, _, _) in positions.iter() {
        if let Some(&entity) = id_map.get(&pos.entity) {
            if let Ok((_, _, mut transform, _)) = query.get_mut(entity) {
                transform.translation.x = pos.x.into_inner() as f32;
                transform.translation.y = pos.y.into_inner() as f32;
                transform.translation.z = pos.z.into_inner() as f32;
            }
        }
    }

    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, _, _) in velocities.iter() {
        if let Some(&entity) = id_map.get(&vel.entity) {
            if let Ok((_, _, _, Some(mut v))) = query.get_mut(entity) {
                v.vx = vel.vx.into_inner() as f32;
                v.vy = vel.vy.into_inner() as f32;
                v.vz = vel.vz.into_inner() as f32;
            }
        }
    }

    state.circuit.clear_inputs();
}
