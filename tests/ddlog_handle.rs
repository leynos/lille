//! Tests for the `DdlogHandle` drop implementation.

use differential_datalog::api::STOP_CALLS;
use lille::DdlogHandle;
use rstest::{fixture, rstest};
use std::sync::atomic::Ordering;

#[fixture]
fn handle_and_initial_count() -> (DdlogHandle, usize) {
    (DdlogHandle::default(), STOP_CALLS.load(Ordering::SeqCst))
}

#[cfg(not(feature = "ddlog"))]
#[rstest]
fn dropping_handle_does_not_call_stop_without_feature(
    handle_and_initial_count: (DdlogHandle, usize),
) {
    let (handle, initial) = handle_and_initial_count;
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial);
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial);
}

#[cfg(feature = "ddlog")]
#[rstest]
fn dropping_handle_stops_program(handle_and_initial_count: (DdlogHandle, usize)) {
    let (handle, initial) = handle_and_initial_count;
    drop(handle);
    assert_eq!(STOP_CALLS.load(Ordering::SeqCst), initial + 1);
}
