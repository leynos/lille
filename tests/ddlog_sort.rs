//! Tests for DDlog command sorting functionality.
//!
//! This module contains tests that verify the correct sorting behaviour of DDlog commands,
//! ensuring that commands are properly ordered by relation and entity identifiers.

#[cfg(feature = "ddlog")]
use differential_datalog::record::{IntoRecord, RelIdentifier, UpdCmd};
#[cfg(feature = "ddlog")]
use lille::ddlog_handle::{self, extract_entity, RelIdentifierExt};
#[cfg(feature = "ddlog")]
use lille_ddlog::{typedefs::entity_state as es, Relations};
#[cfg(feature = "ddlog")]
use ordered_float::OrderedFloat;

#[cfg(feature = "ddlog")]
#[test]
fn commands_sorted_by_rel_and_entity() {
    let mut cmds = vec![
        UpdCmd::Delete(
            RelIdentifier::RelId(Relations::entity_state_Position as usize),
            es::Position {
                entity: 7,
                x: OrderedFloat(0.0),
                y: OrderedFloat(0.0),
                z: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Insert(
            RelIdentifier::RelId(Relations::entity_state_Target as usize),
            es::Target {
                entity: 5,
                tx: OrderedFloat(0.0),
                ty: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Insert(
            RelIdentifier::RelId(Relations::entity_state_Position as usize),
            es::Position {
                entity: 10,
                x: OrderedFloat(0.0),
                y: OrderedFloat(0.0),
                z: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Insert(
            RelIdentifier::RelId(Relations::entity_state_Position as usize),
            es::Position {
                entity: 3,
                x: OrderedFloat(0.0),
                y: OrderedFloat(0.0),
                z: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Delete(
            RelIdentifier::RelId(Relations::entity_state_Target as usize),
            es::Target {
                entity: 8,
                tx: OrderedFloat(0.0),
                ty: OrderedFloat(0.0),
            }
            .into_record(),
        ),
    ];

    ddlog_handle::sort_cmds(cmds.as_mut_slice());
    let ids: Vec<(usize, i64)> = cmds
        .iter()
        .map(|c| {
            #[expect(
                unreachable_patterns,
                reason = "Support potential future UpdCmd variants"
            )]
            #[allow(unfulfilled_lint_expectations)]
            match c {
                UpdCmd::Insert(r, rec) | UpdCmd::Delete(r, rec) => {
                    (r.as_id(), extract_entity(r, rec))
                }
                _ => (usize::MAX, i64::MAX),
            }
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            (Relations::entity_state_Position as usize, 3),
            (Relations::entity_state_Position as usize, 7),
            (Relations::entity_state_Position as usize, 10),
            (Relations::entity_state_Target as usize, 5),
            (Relations::entity_state_Target as usize, 8),
        ]
    );
}

#[cfg(feature = "ddlog")]
#[test]
fn commands_sorted_with_same_rel_and_entity() {
    let mut cmds = vec![
        UpdCmd::Insert(
            RelIdentifier::RelId(Relations::entity_state_Target as usize),
            es::Target {
                entity: 5,
                tx: OrderedFloat(0.0),
                ty: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Delete(
            RelIdentifier::RelId(Relations::entity_state_Target as usize),
            es::Target {
                entity: 5,
                tx: OrderedFloat(0.0),
                ty: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Delete(
            RelIdentifier::RelId(Relations::entity_state_Position as usize),
            es::Position {
                entity: 3,
                x: OrderedFloat(0.0),
                y: OrderedFloat(0.0),
                z: OrderedFloat(0.0),
            }
            .into_record(),
        ),
        UpdCmd::Insert(
            RelIdentifier::RelId(Relations::entity_state_Position as usize),
            es::Position {
                entity: 3,
                x: OrderedFloat(0.0),
                y: OrderedFloat(0.0),
                z: OrderedFloat(0.0),
            }
            .into_record(),
        ),
    ];

    ddlog_handle::sort_cmds(cmds.as_mut_slice());
    let ids: Vec<(usize, i64)> = cmds
        .iter()
        .map(|c| {
            #[expect(
                unreachable_patterns,
                reason = "Support potential future UpdCmd variants"
            )]
            #[allow(unfulfilled_lint_expectations)]
            match c {
                UpdCmd::Insert(r, rec) | UpdCmd::Delete(r, rec) => {
                    (r.as_id(), extract_entity(r, rec))
                }
                _ => (usize::MAX, i64::MAX),
            }
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            (Relations::entity_state_Position as usize, 3),
            (Relations::entity_state_Position as usize, 3),
            (Relations::entity_state_Target as usize, 5),
            (Relations::entity_state_Target as usize, 5),
        ]
    );
}
