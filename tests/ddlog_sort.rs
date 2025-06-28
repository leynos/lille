#[cfg(feature = "ddlog")]
use differential_datalog::record::{Record, RelIdentifier, UpdCmd};
#[cfg(feature = "ddlog")]
use lille::ddlog_handle::{self, RelIdentifierExt};

#[cfg(feature = "ddlog")]
#[test]
fn commands_sorted_by_rel_and_entity() {
    let mut cmds = vec![
        UpdCmd::Insert(RelIdentifier::RelId(2), Record { entity: 5 }),
        UpdCmd::Insert(RelIdentifier::RelId(1), Record { entity: 10 }),
        UpdCmd::Insert(RelIdentifier::RelId(1), Record { entity: 3 }),
    ];

    ddlog_handle::sort_cmds(cmds.as_mut_slice());
    let ids: Vec<(usize, i64)> = cmds
        .iter()
        .map(|c| match c {
            UpdCmd::Insert(r, rec) | UpdCmd::Delete(r, rec) => (r.as_id(), rec.entity),
        })
        .collect();
    assert_eq!(ids, vec![(1, 3), (1, 10), (2, 5)]);
}
