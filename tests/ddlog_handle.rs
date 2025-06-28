//! Tests for the `DdlogHandle` drop implementation.

#[cfg(not(feature = "ddlog"))]
#[test]
fn dropping_handle_does_not_call_stop_without_feature() {
    use differential_datalog::api::STOP_CALLS;
    use lille::DdlogHandle;
    use std::sync::atomic::Ordering;

    let handle = DdlogHandle::default();
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "ddlog")]
#[test]
fn dropping_handle_stops_program() {
    use differential_datalog::api::STOP_CALLS;
    use std::sync::atomic::Ordering;

    let initial_count = STOP_CALLS.load(Ordering::SeqCst);
    let handle = lille::DdlogHandle::default();
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial_count + 1);
}
