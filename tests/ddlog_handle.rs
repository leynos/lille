//! Tests for the `DdlogHandle` drop implementation.

use differential_datalog::api::STOP_CALLS;
use lille::DdlogHandle;
use rstest::{fixture, rstest};
use serial_test::serial;
use std::sync::atomic::Ordering;

#[fixture]
fn handle_and_initial_count() -> (DdlogHandle, usize) {
    STOP_CALLS.store(0, Ordering::SeqCst);
    let handle = DdlogHandle::default();
    (handle, STOP_CALLS.load(Ordering::SeqCst))
}

#[cfg(not(feature = "ddlog"))]
#[rstest]
#[serial]
fn dropping_handle_does_not_call_stop_without_feature(
    handle_and_initial_count: (DdlogHandle, usize),
) {
    let (ddlog_handle, _initial_count) = handle_and_initial_count;
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
    drop(ddlog_handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "ddlog")]
#[rstest]
#[serial]
fn dropping_handle_stops_program(handle_and_initial_count: (DdlogHandle, usize)) {
    let (ddlog_handle, initial_count) = handle_and_initial_count;
    drop(ddlog_handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial_count + 1);
}
