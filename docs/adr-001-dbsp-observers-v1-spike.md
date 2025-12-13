# Differential Datalog-Based Stream Processing (DBSP) Observers V1 spike (damage ingress)

## Context

During the Bevy 0.16 upgrade, the existing DBSP event push/retract flow (the
`DamageInbox` buffer drained during `cache_state_for_dbsp_system`) was
retained, and adoption of Observers V1 was deferred.

Observers V1 (and the Events V2 trigger API) promise simpler event routing and
clearer ownership semantics, potentially reducing boilerplate for DBSP-facing
event ingress/egress (damage events, health deltas, and transform
synchronization).

This spike is tracked under issue 231.

## Goals

- Prototype Observers V1 for DBSP-facing event routing on at least one event
  type (damage ingress).
- Compare complexity and performance (CPU + allocations) against the existing
  direct `DamageInbox` push model.
- Record a decision: adopt now or defer to a later migration (0.17+/Events V2
  follow-up).

## Scope and constraints

- DBSP remains the source of truth; no behavioural changes in the default build
  are allowed.
- Keep the spike feature-gated and minimal.
- Cover the observer-driven route with tests and a targeted micro-benchmark.

## Implementation summary

Feature flag:

- `observers-v1-spike` (Cargo feature)

Changes:

- Adds `DbspDamageIngress` as a feature-gated `#[derive(Event)]` wrapper around
  `dbsp_circuit::DamageEvent`.
- Installs an observer that translates `DbspDamageIngress` triggers into
  `DamageInbox` pushes.
- Under the feature flag, moves the DBSP sync chain to `PostUpdate` so
  `Commands::trigger(DbspDamageIngress { .. })` issued in `Update` is flushed
  before DBSP ingestion.

No Bevy feature flags are required beyond the existing Bevy 0.17 configuration;
the project already uses observer APIs for `DbspSyncError`.

## How to run

Run the observer tests:

- `make test-observers-v1`

Run the micro-benchmark with captured output:

```shell
RUST_LOG=info cargo test --features "test-support observers-v1-spike" \
  --test perf_damage_routing_observers_v1_spike -- --nocapture
```

## Comparison

### Existing routing (baseline)

- **Ingress API:** systems/tests mutate `ResMut<DamageInbox>` directly.
- **Behaviour:** `cache_state_for_dbsp_system` drains the inbox once per frame,
  deduplicates within the tick, pushes into the DBSP input Z-set, and schedules
  a retraction on the next frame.
- **Complexity:** minimal, but requires systems to plumb the inbox resource
  through signatures.

### Observer routing (spike)

- **Ingress API:** systems/tests `trigger(DbspDamageIngress::from(event))`.
- **Behaviour:** unchanged DBSP ingestion; observers only buffer into
  `DamageInbox`.
- **Scheduling note:** in-schedule triggers use `Commands`, so the DBSP sync
  chain must run after a deferred flush to avoid a one-frame latency. The spike
  achieves this by using `PostUpdate` under the feature flag.
- **Complexity:** removes `ResMut<DamageInbox>` wiring at callsites but adds a
  new event wrapper, observer wiring, and a scheduling conditional.

## Performance notes

This spike adds a micro-benchmark test that compares direct inbox pushes versus
`World::trigger(DbspDamageIngress { .. })`, tracking both time and allocations.

The test is intentionally coarse and only asserts that observer routing does
not introduce pathological allocation behaviour.

Sample output (run on 13 December 2025, Linux, `N = 10_000`):

- Direct inbox push: ~236 µs, 14 allocations, ~680 KiB allocated
- Observer routing: ~848 µs, 20 allocations, ~556 KiB allocated

These numbers are only intended for relative comparison and will vary across
machines and Bevy versions. Reproduce by running:

```shell
RUST_LOG=info cargo test --features "test-support observers-v1-spike" \
  --test perf_damage_routing_observers_v1_spike -- --nocapture
```

## Decision

Defer a full migration of DBSP ingress/egress routing to Observers V1 for now.

Rationale:

- The observer-driven route is ergonomically nicer at callsites, but it
  introduces scheduling coupling: triggers must be flushed before DBSP sync to
  preserve same-frame ingestion.
- The micro-benchmark indicates higher CPU overhead for the observer-driven
  route compared to direct inbox mutation, even though allocation behaviour is
  broadly comparable at this scale.
- The intended broader migration (health delta outputs and transform sync)
  would require additional design work to avoid silently changing when the
  Entity Component System (ECS) reads observe DBSP outputs.
- Bevy’s event APIs are still evolving (Events V2), so keeping the spike
  feature-gated minimizes churn risk.

Concrete blockers to adopting broadly in 0.17:

- Define and enforce a stable schedule contract for when DBSP sync runs
  relative to gameplay systems emitting triggers.
- Decide whether transform sync should remain a snapshot scan (current model)
  or move to change-based ingestion (observer model), and document the
  behavioural implications.
