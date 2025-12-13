//! Targeted micro-benchmark for observer-driven damage routing.
//!
//! The goal of this test is not to enforce strict timing constraints (which
//! would be brittle in CI), but to:
//! - Exercise the observer-driven path at scale.
//! - Record rough CPU and allocation deltas relative to direct inbox pushes.
//!
//! Run with:
//! - `RUST_LOG=info cargo test --features "test-support observers-v1-spike" \
//!   --test perf_damage_routing_observers_v1_spike -- --nocapture`

#![cfg(feature = "observers-v1-spike")]

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::{DamageInbox, DbspDamageIngress};
use lille::DbspPlugin;
use log::info;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct CountingAlloc;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: We delegate to the system allocator. `layout` is provided by
        // the allocator contract and must be forwarded unchanged.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` originate from `alloc`, and we forward them
        // unchanged to the system allocator.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

// NOTE: This test installs a global allocator for the entire integration test
// binary so allocations can be counted. The counters are reset between the
// direct and observer paths to provide a relative comparison.

fn reset_alloc_counters() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

fn read_alloc_counters() -> (u64, u64) {
    (
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

const fn sample_event() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 1,
        source: DamageSource::External,
        at_tick: 1,
        seq: Some(1),
    }
}

#[test]
fn routing_costs_are_reasonable() {
    const N: usize = 10_000;

    // Enable log output for `-- --nocapture` runs without failing if another
    // test already initialised logging.
    let _logging_initialised = env_logger::builder().is_test(true).try_init().is_ok();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DbspPlugin);

    let events = vec![sample_event(); N];

    // Baseline: direct resource pushes.
    reset_alloc_counters();
    let direct_start = Instant::now();
    let direct_count = {
        let mut inbox = app.world_mut().resource_mut::<DamageInbox>();
        let inbox_ref = inbox.as_mut();
        DamageInbox::extend(inbox_ref, events.iter().copied());
        assert!(!inbox_ref.is_empty());
        DamageInbox::drain(inbox_ref).count()
    };
    let _elapsed_direct = direct_start.elapsed();
    let (direct_allocs, direct_bytes) = read_alloc_counters();
    assert_eq!(direct_count, N);

    // Spike: observer route via trigger dispatch.
    reset_alloc_counters();
    let observer_start = Instant::now();
    {
        let world = app.world_mut();
        for event in &events {
            world.trigger(DbspDamageIngress::from(*event));
        }
    }
    let _elapsed_observer = observer_start.elapsed();
    let (observer_allocs, observer_bytes) = read_alloc_counters();
    let observer_count = {
        let mut inbox = app.world_mut().resource_mut::<DamageInbox>();
        DamageInbox::drain(inbox.as_mut()).count()
    };
    assert_eq!(observer_count, N);

    // This assertion is intentionally coarse: we only want to catch
    // pathological regressions (for example, O(N) allocations versus O(1)).
    assert!(
        observer_allocs <= direct_allocs.saturating_add(100),
        concat!(
            "observer routing allocated too much: direct allocs={direct_allocs} ",
            "bytes={direct_bytes}; observer allocs={observer_allocs} bytes={observer_bytes}"
        ),
        direct_allocs = direct_allocs,
        direct_bytes = direct_bytes,
        observer_allocs = observer_allocs,
        observer_bytes = observer_bytes,
    );

    info!(
        concat!(
            "damage routing micro-bench (N={N}): direct allocs={direct_allocs} bytes={direct_bytes}; ",
            "observer allocs={observer_allocs} bytes={observer_bytes}"
        ),
        N = N,
        direct_allocs = direct_allocs,
        direct_bytes = direct_bytes,
        observer_allocs = observer_allocs,
        observer_bytes = observer_bytes,
    );
}
