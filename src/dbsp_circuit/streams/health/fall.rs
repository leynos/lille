//! Fall damage derivation streams.
//!
//! Detects landing transitions and emits [`DamageEvent`] records that apply
//! fall damage entirely within the DBSP circuit.

use crate::dbsp_circuit::{DamageEvent, DamageSource, PositionFloor, Tick, Velocity};
use crate::{FALL_DAMAGE_SCALE, LANDING_COOLDOWN_TICKS, SAFE_LANDING_SPEED, TERMINAL_VELOCITY};
use dbsp::utils::Tup2;
use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

fn detect_landings(
    standing: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
) -> Stream<RootCircuit, OrdZSet<i64>> {
    let standing_entities = standing.map(|pf| pf.position.entity);
    let prev_unsupported = unsupported.map(|pf| pf.position.entity).delay();

    prev_unsupported.map_index(|entity| (*entity, ())).join(
        &standing_entities.map_index(|entity| (*entity, ())),
        |entity, _, _| *entity,
    )
}

fn apply_landing_cooldown(
    landings: &Stream<RootCircuit, OrdZSet<i64>>,
) -> Stream<RootCircuit, OrdZSet<i64>> {
    let mut cooldown_end = landings.clone();
    for _ in 0..LANDING_COOLDOWN_TICKS {
        cooldown_end = cooldown_end.delay();
    }

    let cooldown_updates = landings.clone().plus(&cooldown_end.neg());
    let active_cooldown = cooldown_updates.integrate();
    let cooling_entities = active_cooldown.delay().map_index(|entity| (*entity, ()));

    landings
        .map_index(|entity| (*entity, ()))
        .antijoin(&cooling_entities)
        .map(|(entity, _)| *entity)
}

fn calculate_fall_damage(
    allowed_landings: &Stream<RootCircuit, OrdZSet<i64>>,
    unsupported_velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
    ticks: &Stream<RootCircuit, Tick>,
) -> Stream<RootCircuit, OrdZSet<DamageEvent>> {
    let prev_velocities = unsupported_velocities.delay();
    let landing_impacts = allowed_landings
        .map_index(|entity| (*entity, *entity))
        .join(
            &prev_velocities.map_index(|vel| (vel.entity, vel.vz)),
            |_entity, &landing_entity, &vz| (landing_entity, vz),
        );

    let downward_impacts = landing_impacts.flat_map(|&(entity, vz)| {
        let speed = -vz.into_inner();
        (speed > 0.0)
            .then_some((entity, OrderedFloat(speed)))
            .into_iter()
    });

    downward_impacts.apply2(ticks, |impacts, tick| {
        let mut tuples = Vec::new();
        for ((entity, speed), (), weight) in impacts.iter() {
            if weight == 0 {
                continue;
            }
            let entity_id = match u64::try_from(entity) {
                Ok(id) => id,
                Err(_) => {
                    debug_assert!(false, "negative entity id {entity}");
                    continue;
                }
            };
            let clamped_speed = speed.into_inner().min(TERMINAL_VELOCITY);
            let excess = clamped_speed - SAFE_LANDING_SPEED;
            if excess <= 0.0 {
                continue;
            }
            let scaled = excess * FALL_DAMAGE_SCALE;
            if scaled <= 0.0 {
                continue;
            }
            let damage = scaled.min(f64::from(u16::MAX)).floor() as u16;
            if damage == 0 {
                continue;
            }
            let event = DamageEvent {
                entity: entity_id,
                amount: damage,
                source: DamageSource::Fall,
                at_tick: *tick,
                seq: None,
            };
            tuples.push(Tup2(Tup2(event, ()), weight));
        }
        OrdZSet::from_tuples((), tuples)
    })
}

/// Derives fall damage events from landing transitions.
///
/// # Examples
/// ```rust,no_run
/// use dbsp::{operator::Generator, Circuit, RootCircuit};
/// use lille::dbsp_circuit::{fall_damage_stream, PositionFloor, Tick, Velocity};
///
/// let (_circuit, (_standing_in, _unsupported_in, _velocity_in, _fall_output)) =
///     RootCircuit::build(|circuit| {
///         let (standing_stream, standing_in) = circuit.add_input_zset::<PositionFloor>();
///         let (unsupported_stream, unsupported_in) =
///             circuit.add_input_zset::<PositionFloor>();
///         let (velocity_stream, velocity_in) = circuit.add_input_zset::<Velocity>();
///         let ticks = circuit.add_source(Generator::new({
///             let mut tick: Tick = 0;
///             move || {
///                 let current = tick;
///                 tick = tick.checked_add(1).expect("tick counter overflowed u64");
///                 current
///             }
///         }));
///         let fall = fall_damage_stream(
///             &standing_stream,
///             &unsupported_stream,
///             &velocity_stream,
///             &ticks,
///         );
///         Ok((standing_in, unsupported_in, velocity_in, fall.output()))
///     })
///     .expect("build fall damage stream");
/// ```
pub fn fall_damage_stream(
    standing: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported_velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
    ticks: &Stream<RootCircuit, Tick>,
) -> Stream<RootCircuit, OrdZSet<DamageEvent>> {
    let landings = detect_landings(standing, unsupported);
    let allowed_landings = apply_landing_cooldown(&landings);
    calculate_fall_damage(&allowed_landings, unsupported_velocities, ticks)
}
