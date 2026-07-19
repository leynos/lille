//! Fear-level derivation streams.
//!
//! Merges explicit fear inputs with entity positions so every positioned
//! entity carries a fear level, defaulting to zero when none is supplied.

use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

use crate::dbsp_circuit::{FearLevel, Position};

/// Merges explicit fear inputs with entity positions, defaulting to zero.
///
/// Each position yields a [`FearLevel`] record. Explicit fear levels flow
/// through unchanged, while an antijoin identifies missing entities and assigns
/// them a `0.0` level before the results are unioned back together.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Result;
/// # use dbsp::RootCircuit;
/// # use ordered_float::OrderedFloat;
/// # use lille::dbsp_circuit::{FearLevel, Position};
/// # use lille::dbsp_circuit::fear_level_stream;
/// # fn main() -> Result<()> {
/// let (mut circuit, (positions_in, fears_in, mut level_out)) =
///     RootCircuit::build(|circuit| {
///         let (positions_stream, positions_handle) =
///             circuit.add_input_zset::<Position>();
///         let (fears_stream, fears_handle) =
///             circuit.add_input_zset::<FearLevel>();
///         let handle =
///             fear_level_stream(&positions_stream, &fears_stream).output();
///         Ok((positions_handle, fears_handle, handle))
///     })?;
///
/// positions_in.push(
///     Position {
///         entity: 7,
///         x: 0.0.into(),
///         y: 0.0.into(),
///         z: 0.0.into(),
///     },
///     1,
/// );
/// circuit.step()?;
///
/// let levels: Vec<FearLevel> = level_out
///     .consolidate()
///     .iter()
///     .map(|(level, (), _)| level.clone())
///     .collect();
/// assert_eq!(
///     levels,
///     vec![FearLevel {
///         entity: 7,
///         level: OrderedFloat(0.0),
///     }]
/// );
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn fear_level_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    fears: &Stream<RootCircuit, OrdZSet<FearLevel>>,
) -> Stream<RootCircuit, OrdZSet<FearLevel>> {
    let missing = positions
        .map_index(|p| (p.entity, ()))
        .antijoin(&fears.map_index(|f| (f.entity, ())))
        .map(|(entity, ())| FearLevel {
            entity: *entity,
            level: OrderedFloat(0.0),
        });

    fears.plus(&missing)
}
