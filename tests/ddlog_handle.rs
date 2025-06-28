//! Tests for the `DdlogHandle` drop implementation.

#[cfg(not(feature = "ddlog"))]
#[test]
fn dropping_handle_does_not_call_stop_without_feature() {
    use lille::ddlog_handle::{DdlogHandle, STOP_CALLS};
    use std::sync::atomic::Ordering;

    let handle = DdlogHandle::default();
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "ddlog")]
use lille::ddlog_handle::DdlogApi;
#[cfg(feature = "ddlog")]
use mockall::mock;

#[cfg(feature = "ddlog")]
mock! {
    pub Api {}
    impl DdlogApi for Api {
        fn transaction_start(&mut self) -> Result<(), String>;
        fn apply_updates_dynamic(
            &mut self,
            updates: &mut dyn Iterator<Item = differential_datalog::record::UpdCmd>,
        ) -> Result<(), String>;
        fn transaction_commit_dump_changes_dynamic(
            &mut self,
        ) -> Result<
            std::collections::BTreeMap<usize, Vec<(differential_datalog::record::Record, isize)>>,
            String,
        >;
        fn stop(self: Box<Self>) -> Result<(), String>;
    }
}

#[cfg(feature = "ddlog")]
#[test]
fn dropping_handle_stops_program() {
    use lille::ddlog_handle::{DdlogHandle, STOP_CALLS};
    use std::sync::atomic::Ordering;

    let initial_count = STOP_CALLS.load(Ordering::SeqCst);
    let mut mock_prog = MockApi::new();
    mock_prog.expect_stop().times(1).returning(|| Ok(()));

    let handle = DdlogHandle::with_program(Box::new(mock_prog));
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial_count + 1);
}
