//! Tests verifying that record types using `dbsp_copy_record!` implement `Copy`
//! and can be duplicated without changing their state.
use lille::dbsp_circuit::{
    FloorHeightAt, HighestBlockAt, MovementDecision, Position, Target, Velocity,
};
use ordered_float::OrderedFloat;
use rstest::rstest;

#[rstest]
#[case::position(Position { entity: 0, x: OrderedFloat(0.0), y: OrderedFloat(0.0), z: OrderedFloat(0.0) })]
#[case::velocity(Velocity { entity: 0, vx: OrderedFloat(0.0), vy: OrderedFloat(0.0), vz: OrderedFloat(0.0) })]
#[case::highest_block(HighestBlockAt { x: 0, y: 0, z: 0 })]
#[case::floor_height(FloorHeightAt { x: 0, y: 0, z: OrderedFloat(0.0) })]
#[case::target(Target { entity: 0, x: OrderedFloat(0.0), y: OrderedFloat(0.0) })]
#[case::movement_decision(MovementDecision { entity: 0, dx: OrderedFloat(0.0), dy: OrderedFloat(0.0) })]
fn copy_records_are_copy<T>(#[case] sample: T)
where
    T: Copy + PartialEq + core::fmt::Debug,
{
    let duplicate = sample;
    let original = sample;
    assert_eq!(duplicate, original);
}
