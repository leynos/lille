//! Tests for the `DdlogHandle` drop implementation.

use differential_datalog::api::STOP_CALLS;
use lille::DdlogHandle;
use rstest::{fixture, rstest};
use serial_test::serial;
use std::sync::atomic::Ordering;

#[fixture]
fn ddlog_handle() -> DdlogHandle {
    DdlogHandle::default()
}

#[fixture]
fn initial_stop_calls() -> usize {
    STOP_CALLS.store(0, Ordering::SeqCst);
    0
}

#[cfg(not(feature = "ddlog"))]
#[rstest]
#[serial]
fn dropping_handle_does_not_call_stop_without_feature(
    ddlog_handle: DdlogHandle,
    initial_stop_calls: usize,
) {
    assert_eq!(initial_stop_calls, 0);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
    drop(ddlog_handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "ddlog")]
#[rstest]
#[serial]
fn dropping_handle_stops_program(ddlog_handle: DdlogHandle, initial_stop_calls: usize) {
    assert_eq!(initial_stop_calls, 0);
    drop(ddlog_handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), 1);
}
