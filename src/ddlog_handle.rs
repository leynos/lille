//! Helper types and logic for interfacing with `DDlog`.
//! Provides the `DdlogHandle` resource and entity representations used by syncing systems.
use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};
#[cfg(not(feature = "ddlog"))]
use crate::{GRACE_DISTANCE, GRAVITY_PULL};

#[cfg(feature = "ddlog")]
use differential_datalog::api::HDDlog;
#[cfg(feature = "ddlog")]
#[allow(unused_imports)]
use differential_datalog::{DDlog, DDlogDynamic};
#[cfg(feature = "ddlog")]
use ordered_float::OrderedFloat;

#[derive(Clone, Serialize)]
pub struct DdlogEntity {
    pub position: Vec3,
    pub unit: UnitType,
    pub health: i32,
    pub target: Option<Vec2>,
}

impl Default for DdlogEntity {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            unit: UnitType::Civvy { fraidiness: 0.0 },
            health: 0,
            target: None,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct NewPosition {
    pub entity: i64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Resource)]
pub struct DdlogHandle {
    #[cfg(feature = "ddlog")]
    pub prog: Option<HDDlog>,
    pub blocks: Vec<Block>,
    pub slopes: HashMap<i64, BlockSlope>,
    pub entities: HashMap<i64, DdlogEntity>,
    pub deltas: Vec<NewPosition>,
}

impl Default for DdlogHandle {
    fn default() -> Self {
        #[cfg(feature = "ddlog")]
        let prog = match lille_ddlog::run(1, false) {
            Ok((p, _)) => Some(p),
            Err(e) => {
                log::error!("failed to start DDlog: {e}");
                None
            }
        };
        Self {
            #[cfg(feature = "ddlog")]
            prog,
            blocks: Vec::new(),
            slopes: HashMap::new(),
            entities: HashMap::new(),
            deltas: Vec::new(),
        }
    }
}

pub fn init_ddlog_system(mut commands: Commands) {
    commands.insert_resource(DdlogHandle::default());
    info!("DDlog handle created");
}

impl DdlogHandle {
    #[cfg(not(feature = "ddlog"))]
    fn highest_block_at(&self, x: i32, y: i32) -> Option<&Block> {
        self.blocks
            .iter()
            .filter(|b| b.x == x && b.y == y)
            .max_by_key(|b| b.z)
    }

    /// Calculates the floor height at `(x, y)` relative to a block.
    #[cfg(not(feature = "ddlog"))]
    pub fn floor_height_at(block: &Block, slope: Option<&BlockSlope>, x: f32, y: f32) -> f32 {
        let base = block.z as f32 + 1.0;
        if let Some(s) = slope {
            base + (x - block.x as f32) * s.grad_x + (y - block.y as f32) * s.grad_y
        } else {
            base
        }
    }

    #[cfg(not(feature = "ddlog"))]
    fn floor_height_at_point(&self, x: f32, y: f32) -> f32 {
        let x_grid = x.floor() as i32;
        let y_grid = y.floor() as i32;
        if let Some(block) = self.highest_block_at(x_grid, y_grid) {
            let slope = self.slopes.get(&block.id);
            DdlogHandle::floor_height_at(block, slope, x, y)
        } else {
            0.0
        }
    }

    #[cfg(not(feature = "ddlog"))]
    fn apply_gravity(&self, pos: &mut Vec3, floor: f32) {
        if pos.z > floor + GRACE_DISTANCE as f32 {
            pos.z += GRAVITY_PULL as f32;
        } else {
            pos.z = floor;
        }
    }
    #[cfg(not(feature = "ddlog"))]
    fn civvy_move(&self, id: i64, ent: &DdlogEntity, pos: Vec3) -> Vec2 {
        let fraidiness = match ent.unit {
            UnitType::Civvy { fraidiness } => fraidiness,
            _ => return Vec2::ZERO,
        };

        let mut min_d2 = f32::INFINITY;
        let mut closest = None;
        let mut total_fear = 0.0;

        for (&bid, b_ent) in self.entities.iter() {
            if let UnitType::Baddie { meanness } = b_ent.unit {
                if bid == id {
                    continue;
                }
                let to_actor = pos.truncate() - b_ent.position.truncate();
                let d2 = to_actor.length_squared();
                let fear_radius = fraidiness * meanness * 2.0;
                if d2 < fear_radius * fear_radius {
                    total_fear += 1.0 / (d2 + 0.001);
                }
                if d2 < min_d2 {
                    min_d2 = d2;
                    closest = Some(b_ent.position);
                }
            }
        }

        if total_fear > 0.2 {
            if let Some(b_pos) = closest {
                return Vec2::new((pos.x - b_pos.x).signum(), (pos.y - b_pos.y).signum());
            }
        } else if let Some(target) = ent.target {
            return Vec2::new((target.x - pos.x).signum(), (target.y - pos.y).signum());
        }

        Vec2::ZERO
    }

    #[cfg(not(feature = "ddlog"))]
    fn compute_entity_update(&self, id: i64, ent: &DdlogEntity) -> Vec3 {
        let floor = self.floor_height_at_point(ent.position.x, ent.position.y);
        let mut pos = ent.position;
        self.apply_gravity(&mut pos, floor);
        let delta = self.civvy_move(id, ent, pos);
        pos.x += delta.x;
        pos.y += delta.y;
        pos
    }

    #[cfg(feature = "ddlog")]
    fn ddlog_position_cmds(&self) -> Vec<differential_datalog::record::UpdCmd> {
        use differential_datalog::record::{IntoRecord, RelIdentifier, UpdCmd};
        use lille_ddlog::{typedefs::entity_state::Position, Relations};

        self.entities
            .iter()
            .map(|(&id, ent)| {
                let record = Position {
                    entity: id,
                    x: OrderedFloat(ent.position.x),
                    y: OrderedFloat(ent.position.y),
                    z: OrderedFloat(ent.position.z),
                };
                UpdCmd::Insert(
                    RelIdentifier::RelId(Relations::entity_state_Position as usize),
                    record.into_record(),
                )
            })
            .collect()
    }

    #[cfg(feature = "ddlog")]
    fn parse_new_position(
        val: &differential_datalog::record::Record,
    ) -> Option<lille_ddlog::typedefs::physics::NewPosition> {
        use differential_datalog::ddval::DDValConvert;
        use lille_ddlog::{relval_from_record, Relations};

        match relval_from_record(Relations::physics_NewPosition, val) {
            Ok(ddval) => {
                <lille_ddlog::typedefs::physics::NewPosition as DDValConvert>::try_from_ddvalue(
                    ddval,
                )
            }
            Err(e) => {
                log::warn!("failed to convert NewPosition record: {e}");
                None
            }
        }
    }

    #[cfg(feature = "ddlog")]
    fn handle_new_position(&mut self, out: lille_ddlog::typedefs::physics::NewPosition) {
        let pos = Vec3::new(out.x.into_inner(), out.y.into_inner(), out.z.into_inner());
        if let Some(ent) = self.entities.get_mut(&out.entity) {
            ent.position = pos;
        }
        self.deltas.push(NewPosition {
            entity: out.entity,
            x: pos.x,
            y: pos.y,
            z: pos.z,
        });
    }

    #[cfg(feature = "ddlog")]
    fn apply_ddlog_deltas(
        &mut self,
        changes: &std::collections::BTreeMap<
            usize,
            Vec<(differential_datalog::record::Record, isize)>,
        >,
    ) {
        use lille_ddlog::Relations;

        self.deltas.clear();
        if let Some(delta) = changes.get(&(Relations::physics_NewPosition as usize)) {
            for (val, weight) in delta {
                if *weight > 0 {
                    if let Some(out) = Self::parse_new_position(val) {
                        self.handle_new_position(out);
                    }
                } else if *weight < 0 {
                    log::warn!("ignoring negative weight {weight} for physics_NewPosition delta");
                }
            }
        }
    }

    #[cfg(feature = "ddlog")]
    fn execute_ddlog_transaction(
        prog: &mut differential_datalog::api::HDDlog,
        cmds: Vec<differential_datalog::record::UpdCmd>,
    ) -> Option<std::collections::BTreeMap<usize, Vec<(differential_datalog::record::Record, isize)>>>
    {
        if let Err(e) = prog.transaction_start() {
            log::error!("DDlog transaction_start failed: {e}");
            return None;
        }

        let mut iter = cmds.into_iter();
        match prog.apply_updates_dynamic(&mut iter) {
            Err(e) => {
                log::error!("DDlog apply_updates failed: {e}");
                None
            }
            Ok(()) => match prog.transaction_commit_dump_changes_dynamic() {
                Ok(changes) => Some(changes),
                Err(e) => {
                    log::error!("DDlog commit failed: {e}");
                    None
                }
            },
        }
    }

    #[cfg(not(feature = "ddlog"))]
    fn collect_fallback_updates(&self) -> Vec<(i64, Vec3)> {
        self.entities
            .iter()
            .map(|(&id, ent)| (id, self.compute_entity_update(id, ent)))
            .collect()
    }

    #[cfg(not(feature = "ddlog"))]
    fn apply_fallback_updates(&mut self, updates: Vec<(i64, Vec3)>) {
        self.deltas.clear();
        for (id, pos) in updates {
            if let Some(ent) = self.entities.get_mut(&id) {
                if pos != ent.position {
                    ent.position = pos;
                    self.deltas.push(NewPosition {
                        entity: id,
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    });
                }
            }
        }
    }

    /// Advances the simulation by one tick.
    ///
    /// When the `ddlog` feature is enabled, this method manages the full
    /// lifecycle of a DDlog transaction. It streams the cached state from
    /// [`DdlogHandle`] into the DDlog program, commits the transaction, and
    /// applies any returned deltas. In builds without DDlog, it falls back to a
    /// simplified Rust implementation that directly updates positions.
    pub fn step(&mut self) {
        #[cfg(feature = "ddlog")]
        {
            let cmds = self.ddlog_position_cmds();
            if let Some(prog) = &mut self.prog {
                if let Some(changes) = Self::execute_ddlog_transaction(prog, cmds) {
                    self.apply_ddlog_deltas(&changes);
                }
            }
        }

        #[cfg(not(feature = "ddlog"))]
        {
            let updates = self.collect_fallback_updates();
            self.apply_fallback_updates(updates);
        }
    }
}
