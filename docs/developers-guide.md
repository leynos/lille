# DBSP synchronization developer's guide

This guide documents the contract between Bevy's Entity-Component-System (ECS)
and the DBSP (Differential Dataflow Stream Processing) circuit that governs
Lille's simulation state. It covers how the synchronization systems are
scheduled, how per-frame state is tracked and rolled back on failure, how
circuit outputs are applied back to components, and the local tooling used to
lint the `dbsp_sync` module.

For the circuit's data model and dataflow construction, see [Declarative
world inference with DBSP and Rust](
declarative-world-inference-with-dbsp-and-rust.md). For the health and damage
synchronization protocol in more depth, see §3.5 of [Lille physics engine
design](lille-physics-engine-design.md). For the test-writing patterns used
throughout `dbsp_sync`, see [Testing declarative game logic in DBSP](
testing-declarative-game-logic-in-dbsp.md). For the `observers-v1-spike`
feature's effect on scheduling, see [ADR-001: DBSP Observers V1 spike](
adr-001-dbsp-observers-v1-spike.md).

## 1. Frame lifecycle

`DbspPlugin::build` (`src/dbsp_sync/plugin.rs`) wires the synchronization
systems into the app during plugin construction:

1. It registers `log_dbsp_error` as an observer of `DbspSyncError` events, so
   failures are logged (via `error!`) even when raised before any schedule
   runs.
2. Under the `observers-v1-spike` feature, it also registers
   `observers_v1::buffer_damage_ingress`.
3. It calls `init_dbsp_system` synchronously to construct the `DbspCircuit`
   and insert the `DbspState` non-send resource. If circuit construction
   fails, it triggers a `DbspSyncError` with context `Init` and returns
   *without* registering the sync chain — the plugin is otherwise inert for
   the rest of the app's lifetime.
4. It initializes the `DamageInbox` resource, schedules
   `init_world_handle_system` at `Startup`, and calls `add_dbsp_sync_chain`.

`add_dbsp_sync_chain` chains two systems with Bevy's `.chain()` combinator,
which guarantees `cache_state_for_dbsp_system` runs to completion before
`apply_dbsp_outputs_system` starts within the same schedule pass:

```rust,ignore
let chain = (cache_state_for_dbsp_system, apply_dbsp_outputs_system).chain();
```

- By default, this chain runs in the `Update` schedule.
- Under the `observers-v1-spike` feature, it runs in `PostUpdate` instead, so
  that `Commands::trigger` calls issued by gameplay systems during `Update`
  (for example, `DbspDamageIngress` triggers) are flushed and delivered to
  `buffer_damage_ingress` — populating `DamageInbox` — before the DBSP chain
  drains it that same frame. See ADR-001 for the sequencing rationale.

Within a single pass of the chain:

- **`cache_state_for_dbsp_system`** (`src/dbsp_sync/input/mod.rs`) reads ECS
  component state and pushes it into the circuit's input handles, via the
  `cache_state_for_dbsp_impl` helper described in
  [§2](#2-frame-rollback-api-on-dbspstate).
- **`apply_dbsp_outputs_system`** (`src/dbsp_sync/output/mod.rs`) steps the
  circuit and, on success, applies its outputs back onto ECS components. See
  [§3](#3-step-failure-handling) and [§4](#4-output-weight-semantics).

## 2. Frame-rollback API on `DbspState`

`cache_state_for_dbsp_impl` mutates several `DbspState` bookkeeping
collections before it is known whether `apply_dbsp_outputs_system` will
successfully step the circuit this frame:

- `health_snapshot`: the last `HealthState` pushed per entity, drained and
  retracted (`-1` weight) so this frame can push a fresh snapshot.
- `pending_damage_retractions`: damage events pushed last frame, taken with
  `mem::take` and retracted (`-1` weight) so they are not double-counted.
- `expected_health_retractions`: cleared and repopulated as retractions are
  issued, matching later `HealthDelta` outputs against expected retractions.
- `applied_unsequenced`: mutated per entity as new unsequenced damage events
  are deduplicated during `ingest_damage_events`.

If `state.step_circuit()` later fails, these mutations must be undone: the
circuit's inputs are cleared without ever being accepted, so the Rust-side
bookkeeping must be restored to match what the circuit actually holds (that
is, nothing from this frame). `DbspState` exposes five methods to manage this
without a per-frame deep clone of the tracking state:

- **`begin_frame_rollback()`** — called at the very start of
  `cache_state_for_dbsp_impl`. Resets `health_snapshot_backup` and
  `pending_damage_backup` to `None` and clears the `applied_unsequenced_undo`
  log, starting a fresh rollback record for this frame.
- **`record_unsequenced_undo(entity)`** — called from `ingest_damage_events`
  for each *unsequenced* damage event, immediately before the entity's
  `applied_unsequenced` entry is mutated by the deduplication check. It
  records that entity's prior `applied_unsequenced` value once per frame
  (repeat calls for the same entity in the same frame are no-ops), so a
  rollback can restore exactly that value later.
- **`stash_frame_rollback(health_snapshot, pending_damage)`** — called once,
  at the end of `cache_state_for_dbsp_impl`, after the cache pass has
  finished mutating the live state. It stores the `Vec<HealthState>` and
  `Vec<DamageEvent>` that were already extracted from the live collections
  earlier in the pass (via `collect_previous_health_snapshots` and
  `mem::take`) as the frame's backups.
- **`commit_frame_tracking()`** — called by `apply_dbsp_outputs_system` after
  a successful `step_circuit()` call. Discards the backups and undo log, so a
  later, stray call to `rollback_frame_tracking()` cannot revert this frame's
  now-committed changes.
- **`rollback_frame_tracking()`** — called by `apply_dbsp_outputs_system`
  when `step_circuit()` returns `Err`. Rebuilds `health_snapshot` from the
  backed-up `Vec<HealthState>` (keyed by `entity`), restores
  `pending_damage_retractions` from the backed-up `Vec<DamageEvent>`, and
  replays the `applied_unsequenced_undo` log: entities with a recorded prior
  value have it reinserted; entities with a recorded `None` (meaning they had
  no entry before this frame) have their entry removed.

The design goal is to avoid deep-cloning the whole tracking state every
frame. The health/damage backups reuse the same vectors the cache pass
already extracts via `mem::take`/`drain`-style moves — no extra clone is
taken solely for rollback purposes. The `applied_unsequenced` undo log takes
a different approach because that collection is a map mutated key-by-key
rather than wholesale: instead of cloning the whole map,
`applied_unsequenced_undo` records only the prior value for each entity
actually touched this frame, the first time it is touched.

This is exercised directly in `src/dbsp_sync/state.rs`'s unit tests:
`rollback_restores_health_snapshot_and_pending_damage` covers the health and
pending-damage backups, and `applied_unsequenced_rollback_matrix` is a
parameterized test over whether the entity had a prior entry, whether the
undo was recorded once or twice, and whether the frame commits or rolls
back — asserting rollback restores the exact pre-frame value and commit
makes a later rollback a no-op.

## 3. Step-failure handling

`apply_dbsp_outputs_system` (`src/dbsp_sync/output/mod.rs`) begins by calling
`state.step_circuit()`, which invokes the stepper function pointer stored on
`DbspState` (`try_step` in production; tests may override it via
`set_stepper_for_testing`). When this returns `Err`:

1. The system queues a `DbspSyncError` (context `Step`) via
   `commands.trigger(...)`. This is a *deferred* `Commands` call: it is
   buffered onto the command queue rather than applied immediately, so the
   `log_dbsp_error` observer only runs once the queued command is flushed at
   the schedule's next command-application point — not synchronously at this
   line.
2. Independently of when that trigger flushes, the system synchronously
   clears every circuit input handle via `state.circuit.clear_inputs()`, so
   the buffered records this frame's cache pass pushed (positions,
   velocities, health state, damage events) are never replayed on a later,
   successful frame.
3. It then calls `state.rollback_frame_tracking()` to restore the pre-frame
   `health_snapshot`, `pending_damage_retractions`, and `applied_unsequenced`
   entries described in [§2](#2-frame-rollback-api-on-dbspstate). Clearing
   the inputs alone would leave that bookkeeping pointing at records the
   circuit never accepted, which would corrupt the retractions the *next*
   frame's cache pass issues.
4. The function returns early. `apply_positions`, `apply_velocities`, and
   `apply_health_deltas` are never called on a failed step, so no ECS
   component is mutated — the circuit remains the sole authority and no
   partial writes occur.

On success, the system applies outputs (see [§4](#4-output-weight-semantics)),
drains any remaining circuit output via `take_from_all()` on each output
handle so stale values cannot be reapplied next frame, clears
`expected_health_retractions` and the circuit's inputs, and finally calls
`state.commit_frame_tracking()` to discard the now-unneeded rollback backups.

This path is exercised by `src/dbsp_sync/output/tests/failure_paths.rs`:
`step_failure_triggers_error_event` asserts the error event is captured and
that a failed step leaves the ECS `Transform` untouched;
`failed_step_clears_inputs_so_they_do_not_replay` asserts a subsequent,
successful run does not replay the cleared inputs; and
`failed_step_rolls_back_health_tracking` runs a full pipeline (two real
`app.update()` calls) to assert that `DbspState::health_snapshot` after a
failed step exactly matches its value before that frame's cache pass ran.

## 4. Output weight semantics

`apply_positions`, `apply_velocities`, and `apply_health_deltas` (all in
`src/dbsp_sync/output/mod.rs`) each read from a DBSP output handle
(`new_position_out`, `new_velocity_out`, `health_delta_out` respectively),
call `.consolidate()` on it, and iterate the resulting `(record, (), weight)`
tuples. Each loop begins with the same guard:

```rust,ignore
if weight <= 0 {
    continue;
}
```

DBSP's `consolidate()` merges every contribution to a Z-set by key and
**removes entries whose net weight is zero** — a record present with equal
positive and negative weight contributions in the same batch simply does not
appear in the consolidated output. Consequently, the `weight <= 0` guard
never observes a genuine zero-weight record in practice; its operative
effect is skipping records with a strictly **negative** weight, which
represent retractions (for example, an entity's previous position or health
snapshot being withdrawn as part of the retract/reinsert pattern described
in [§2](#2-frame-rollback-api-on-dbspstate)). The `<= 0` comparison, rather
than `< 0`, is written defensively against any non-positive weight rather
than to handle an expected zero-weight case.

This is exercised by dedicated tests asserting a negative-weight record does
not mutate its target component:
`negative_weight_position_is_not_applied` and
`negative_weight_velocity_is_not_applied` (in
`src/dbsp_sync/output/tests/mod.rs` and
`src/dbsp_sync/output/tests/edge_cases.rs` respectively) push a position or
velocity record with weight `-1` and assert the `Transform`/`VelocityComp`
stays at its default value; `negative_weight_health_delta_is_not_applied`
retracts a `HealthState` snapshot (weight `-1`) alongside a positive-weight
damage event and asserts `Health` is unchanged.

## 5. Local linting/tooling workflow

Run `make lint` to check the whole workspace before committing changes under
`src/dbsp_sync`. The target runs three checks in sequence:

```makefile
lint:
	set -euo pipefail
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" cargo doc --workspace --no-deps
	cargo clippy --all-targets --all-features -- $(RUST_FLAGS)
	$(RUST_FLAGS_ENV) $(WHITAKER) --all -- --all-targets --all-features
```

1. `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS` set to `--cfg
   docsrs -D warnings`, so broken intra-doc links or other rustdoc warnings
   fail the build.
2. `cargo clippy --all-targets --all-features -- -D warnings`.
3. The Whitaker Dylint suite, invoked as `whitaker --all -- --all-targets
   --all-features` (the `whitaker` binary is looked up on `PATH` by default;
   override it by setting the `WHITAKER` make variable).

CI (`.github/workflows/ci.yml`) runs the same clippy and Whitaker checks in
its "Lint" step, but first has to ensure the `whitaker-installer` and
`whitaker` binaries are present. The "Install the Whitaker Dylint suite"
step:

- Reuses a cached `whitaker-installer` binary (restored from
  `~/.cargo/bin/whitaker-installer` and `~/.cache/cargo-binstall` by
  `actions/cache`) only when its reported `whitaker-installer --version`
  output exactly matches the pinned `WHITAKER_INSTALLER_VERSION` environment
  variable (`0.2.6` at the time of writing). A bare presence check or a
  substring match on the version string is deliberately avoided, since
  either could accept a stale or near-miss version.
- Otherwise, it installs `whitaker-installer`, preferring `cargo binstall`:
  it checks `cargo binstall --version` succeeds, then attempts to binstall
  `whitaker-installer` pinned to `WHITAKER_INSTALLER_VERSION` with
  `--no-confirm --locked`. If `cargo binstall` is unavailable, or that
  install attempt fails, it falls back to a locked source build: `cargo
  install --locked whitaker-installer --version
  "${WHITAKER_INSTALLER_VERSION}"`.
- Finally runs `whitaker-installer` (with no arguments) to complete
  installation of the `whitaker` tool itself before the "Lint" step invokes
  it.

> The exact behaviour of the bare `whitaker-installer` invocation (for
> example, which `whitaker` binary version it installs and where) is not
> defined in this repository's workflow; consult the `whitaker-installer`
> tool's own documentation for details.
