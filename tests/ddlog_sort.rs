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
use rstest::rstest;

#[cfg(feature = "ddlog")]
fn extract_ids(cmds: &[UpdCmd]) -> Vec<(usize, i64)> {
    cmds.iter()
        .map(|c| match c {
            UpdCmd::Insert(r, rec)
            | UpdCmd::InsertOrUpdate(r, rec)
            | UpdCmd::Delete(r, rec)
            | UpdCmd::DeleteKey(r, rec) => (r.as_id(), extract_entity(r, rec)),
            UpdCmd::Modify(r, _, new) => (r.as_id(), extract_entity(r, new)),
        })
        .collect()
}

#[cfg(feature = "ddlog")]
fn extract_ops(cmds: &[UpdCmd]) -> Vec<&'static str> {
    cmds.iter()
        .map(|c| match c {
            UpdCmd::Insert(_, _) => "insert",
            UpdCmd::InsertOrUpdate(_, _) => "insert_or_update",
            UpdCmd::Delete(_, _) => "delete",
            UpdCmd::DeleteKey(_, _) => "delete_key",
            UpdCmd::Modify(_, _, _) => "modify",
        })
        .collect()
}

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
    let ids = extract_ids(&cmds);
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
#[rstest]
#[case(
    "same_rel_and_entity",
    vec![
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
    ],
    vec![
        (Relations::entity_state_Position as usize, 3),
        (Relations::entity_state_Position as usize, 3),
        (Relations::entity_state_Target as usize, 5),
        (Relations::entity_state_Target as usize, 5),
    ],
    vec!["insert", "delete", "insert", "delete"],
)]
#[case(
    "mixed_operations",
    vec![
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
    ],
    vec![
        (Relations::entity_state_Position as usize, 3),
        (Relations::entity_state_Position as usize, 7),
        (Relations::entity_state_Position as usize, 10),
        (Relations::entity_state_Target as usize, 5),
        (Relations::entity_state_Target as usize, 8),
    ],
    vec!["insert", "delete", "insert", "insert", "delete"],
)]
fn test_sorting_scenarios(
    #[case] _name: &str,
    #[case] mut cmds: Vec<UpdCmd>,
    #[case] expected_ids: Vec<(usize, i64)>,
    #[case] expected_ops: Vec<&str>,
) {
    ddlog_handle::sort_cmds(cmds.as_mut_slice());
    let ids = extract_ids(&cmds);
    assert_eq!(ids, expected_ids);
    let ops = extract_ops(&cmds);
    assert_eq!(ops, expected_ops);
}
