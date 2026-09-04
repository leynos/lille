# Developer's guide

This guide records the practical facts a contributor needs about Lille's active
dependency stack. It is the source of truth for the current Bevy and
`bevy_ecs_tiled` versions; the
[Bevy migration plan](bevy-0-16-plus-migration-plan.md) is an archived
historical record and must not be used to infer current versions.

## Toolchain

- `rust-toolchain.toml` pins `nightly-2025-09-14` (rustc 1.91.0-nightly) with
  the `rustfmt` and `clippy` components.
- The nightly channel is required: `src/lib.rs` uses
  `#![cfg_attr(docsrs, feature(doc_cfg))]`, and `make lint` builds the docs with
  `--cfg docsrs`, which needs the unstable `doc_cfg` feature.
- **Do not bump the toolchain or Bevy to 0.19 without also satisfying the
  constraint below.** Bevy 0.19 requires Rust 1.95.0, which this nightly cannot
  provide. `bevy_ecs_tiled` 0.13 already tracks Bevy 0.19, so the plugin is not
  the blocker; the toolchain is.

## Bevy

The workspace targets the **Bevy 0.18.1** release line. Keep the entire Bevy
surface on one minor line: never mix major/minor families across type
signatures, imports, plugins, events, or system parameters.

*Active Bevy dependency versions (workspace and direct subcrates):*

| Dependency       | Version | Notes                                                                                               |
| ---------------- | ------- | --------------------------------------------------------------------------------------------------- |
| `bevy`           | 0.18.1  | Workspace dependency, `default-features = false`, `reflect_auto_register`; Linux target adds `x11`. |
| `bevy_app`       | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_ecs`       | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_math`      | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_reflect`   | 0.18.1  | Feature `auto_register_inventory`.                                                                  |
| `bevy_transform` | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_log`       | 0.18.1  | Optional; enabled through the `render` feature.                                                     |

The optional renderer is gated behind the `render` feature (which pulls in the
Bevy asset, core-pipeline, render, sprite, winit, log, and PNG features); the
`text` feature layers `bevy/bevy_text` on top.

### Buffered events use the Message API

Bevy 0.18 split buffered events from observer events:

- **Buffered events** derive `Message` and are read/written with
  `MessageReader<T>` / `MessageWriter<T>`; from a `World`, use
  `World::write_message`. This is what `TiledEvent<MapCreated>` uses.
- **Observer events** derive `Event` and are consumed via `On<T>` observers,
  emitted with `Commands::trigger` / `World::trigger` and registered with
  `App::add_observer`. Lille's `LilleMapError`, `UnloadPrimaryMap`,
  `PrimaryMapUnloaded`, `DbspSyncError`, and `DbspDamageIngress` are observer
  events.

`App` is `#[must_use]` in Bevy 0.18, so do not add a bare `#[must_use]` to
functions that return `App`; `clippy::double_must_use` will reject it.

The migrated buffered-message surface is guarded two ways. The runtime map
integration tests exercise it dynamically, and a `trybuild` compile-pass
harness pins it statically: `tests/compile_pass.rs` compiles the fixture
`tests/compile_pass/message_reader_migration.rs`, which uses
`MessageReader<TiledEvent<MapCreated>>` and `World::write_message`.
Reintroducing the legacy `EventReader` / `World::send_event` names breaks the
fixture. Run it with:

```sh
cargo test --features test-support --test compile_pass
```

The harness is gated on `test-support` (like the other map tests), so it also
runs as part of `make test` (which passes `--features test-support`) and the CI
coverage step. The fixture is a standalone crate, so `bevy_ecs_tiled` is
carried as a non-optional dev-dependency purely to make it nameable there.

## Map support: `bevy_ecs_tiled`

- Version **0.12.0** (optional, behind the `map` feature),
  `default-features = false`.
- Features: `png` and `user_properties` are always enabled with the crate;
  `render` is added by the `render` feature and `atlas` by `test-support`.
- 0.12 is the `bevy_ecs_tiled` line that tracks Bevy 0.18 (upstream
  compatibility table: 0.11–0.12 target Bevy 0.18). The Bevy-0.19 line is 0.13,
  which already supports Bevy 0.19; adopting it is blocked solely by the Rust
  1.95.0 toolchain constraint above, not by plugin availability.

## `ordered-float` v5 and the vendored `feldera-size-of` fork

Lille's DBSP records store floating-point values through
`ordered_float::OrderedFloat<f64>` so that they have a total order, which DBSP
requires for keys, joins, and aggregations. Those records also derive
`feldera_size_of::SizeOf` for memory accounting. Reconciling the two across a
major `ordered-float` upgrade is the reason for the arrangement described here.

For the full decision record, see
[ADR 002](adr-002-ordered-float-v5-vendored-feldera-size-of-fork.md). The fork
itself is documented in [`third_party/README.md`](../third_party/README.md).
Fork lifecycle and removal are tracked in
[issue #294](https://github.com/leynos/lille/issues/294).

### Why `ordered-float` is pinned at v5

The workspace standardizes on `ordered-float` 5.x
(`ordered-float = { version = "5", features = ["serde", "rkyv_64"] }` in the
root `Cargo.toml`). This is the current major version, and Lille's own records
are built against it. The `rkyv_64` feature still targets rkyv 0.7, matching
Lille's `rkyv = "0.7"`, so the upgrade needs no rkyv changes.

Note that `dbsp` 0.98 independently requires `ordered-float ^4.2.0`, so
ordered-float 4.x and 5.x coexist in the dependency graph: dbsp resolves to v4
for its own internals, while Lille's records use v5. This is expected and
supported.

### Why `feldera-size-of` is patched through `[patch.crates-io]`

Every published `feldera-size-of` release, up to and including 0.1.7 (and
upstream `main`), pins its optional `ordered-float` dependency at `^3.0.0`. Its
`SizeOf` impl for `OrderedFloat`/`NotNan` therefore applies only to
ordered-float 3.x. Against 5.x the derive fails to compile:

```plaintext
error[E0277]: the trait bound `ordered_float::OrderedFloat<f64>:
feldera_size_of::SizeOf` is not satisfied
```

Lille cannot implement `SizeOf` for `OrderedFloat` itself, because both the
trait and the type are foreign (the orphan rule forbids it). No upstream
release accepts ordered-float 5.x, so there is nothing to upgrade to.

The workaround redirects the crate to a minimal vendored fork:

```toml
[patch.crates-io]
feldera-size-of = { path = "third_party/feldera-size-of" }
```

The fork widens its `ordered-float` constraint to 5 and switches the `SizeOf`
impl bound from `Float` to `FloatCore` (ordered-float 5.x bounds its `Deref`
impls on `FloatCore`, and the impl relies on the `&OrderedFloat<T>` → `&T`
deref coercion). That bound switch is the only source change the upgrade
requires.

### Why `third_party/feldera-size-of` is excluded from the workspace

```toml
[workspace]
members = ["build_support", "test_utils"]
exclude = ["third_party/feldera-size-of"]
```

The fork is a `[patch.crates-io]` target, not a first-class workspace member.
Excluding it keeps Cargo from folding it into this workspace and keeps its
upstream source out of the workspace-wide gates (`cargo fmt`, `cargo clippy`,
`cargo test`, `cargo doc`). It is still built and linked, but only as a patched
dependency of `lille`.

### Ownership and scope: carried, not maintained

`third_party/feldera-size-of` is **carried upstream code, not adopted or
maintained Lille code**. Except for the deliberately minimal changes listed in
`third_party/README.md`, its source is byte-identical to `feldera-size-of`
0.1.7, and Lille does not hold it to this repository's code-health, testing,
documentation, or lint standards. This is why the crate carries
`#![allow(warnings)]` and `#![cfg_attr(coverage_nightly, coverage(off))]`, and
why `.codescene/code-health-rules.json` disables the Code Duplication rule under
`third_party/**`.

Feedback about the upstream code that is outside the scope of the vendored fix
— for example requests to test, refactor, or re-architect functionality Lille
does not use, or to change the fork's fallible `SizeOf` traversal semantics or
lint allowances — is out of scope for this repository. Raise such concerns
upstream against [`feldera/size-of`](https://github.com/feldera/size-of).

The Lille-owned guard for this arrangement is `tests/ordered_float_size_of.rs`,
a compile-time integration regression test asserting that `OrderedFloat<f64>`,
`NotNan<f64>`, `Position`, and `BlockSlope` implement `SizeOf`. It is Lille
code and is maintained normally.

### Removing the fork

Once `feldera-size-of` publishes a release whose optional `ordered-float`
dependency accepts 5.x (ideally including the `Float` → `FloatCore` bound fix):

1. Bump Lille's `feldera-size-of` dependency to that upstream release.
2. Delete `third_party/feldera-size-of/`.
3. Drop the `[patch.crates-io]` entry and the `[workspace] exclude` line from
   the root `Cargo.toml`.
4. Remove the CodeScene `third_party/**` rule set from
   `.codescene/code-health-rules.json` if nothing else needs it.
5. Keep `tests/ordered_float_size_of.rs` as the regression guard.
6. Regenerate `Cargo.lock` and run the standard gates.

Progress against these steps is tracked in
[issue #294](https://github.com/leynos/lille/issues/294).

## DBSP synchronization

Each frame, `DbspPlugin` chains two systems so the first runs to completion
before the second starts: `cache_state_for_dbsp_system` reads ECS component
state into the DBSP circuit's input handles, then
`apply_dbsp_outputs_system` steps the circuit and writes its outputs back
onto ECS components.

`DbspState` exposes frame-rollback methods that keep Rust-side bookkeeping
in step with the circuit, called in this order:

- `begin_frame_rollback` — start of the cache pass; clears the previous
  frame's rollback log.
- `record_unsequenced_undo` — during damage ingestion, captures an
  `applied_unsequenced` entry's pre-frame value before it is mutated.
- `stash_frame_rollback` — saves the pre-frame health-snapshot and
  pending-damage values the cache pass extracted.
- `commit_frame_tracking` — on a successful step, discards the rollback
  log.
- `rollback_frame_tracking` — on a failed step, restores the pre-frame
  tracking.

When `state.step_circuit()` returns `Err`, the output system clears the
circuit inputs, restores the Rust-side tracking
(`rollback_frame_tracking`), emits a `DbspSyncError` event, and applies no
ECS output writes that frame.

`apply_positions`, `apply_velocities`, and `apply_health_deltas` apply only
consolidated records with a positive Z-set weight; non-positive
(retraction) weights are skipped.

For the detailed walkthrough, see
[DBSP synchronization developer's guide](dbsp-synchronization-guide.md).

### Movement-aggregation diagnostics

`movement_decision_streams` returns the same deduplicated `MovementDecision`
stream as `movement_decision_stream`, plus a diagnostic
`Stream<RootCircuit, OrdZSet<MovementAggregation>>`. The deduplication
boundary still guarantees at most one emitted movement decision per entity,
and a net-zero total weight still emits no decision; the diagnostic stream
adds visibility without changing that behaviour.

```rust
let (decisions, aggregations) =
    movement_decision_streams(fear, targets, positions);
```

`MovementAggregation { entity, total_weight }` reports that the circuit
collapsed movement decisions for one entity into one normalized vector. The
circuit emits an aggregation record only when the accumulated
`total_weight` falls outside `-1..=1`: a single decision emits no
diagnostic, and a net-zero total emits neither a movement decision nor an
aggregation record.

`DbspCircuit::movement_aggregation_out()` exposes the diagnostic stream as
`OutputHandle<OrdZSet<MovementAggregation>>`. As with every other circuit
output, consumers must consolidate the handle, process only records with a
positive Z-set weight, and drain the handle every frame — otherwise
diagnostics can accumulate and be reported again.

`apply_dbsp_outputs_system` performs that lifecycle:
`report_movement_aggregations` emits the warning in the command layer, then
the system calls `take_from_all()` on `movement_aggregation_out()`. Keep
the distinction explicit: the DBSP fold stays pure and does not log; the
output system owns logging. See [Movement-aggregation
diagnostics](users-guide.md#movement-aggregation-diagnostics) in the user's
guide for the consumer-facing contract.

### Asserting Z-set weights with `collect_weighted`

Because those weight gates are part of the contract, tests need to see the
weights, not just the records. `test_utils::collect_weighted` consolidates a
`dbsp::OutputHandle<OrdZSet<T>>` and returns `Vec<(T, ZWeight)>`, retaining
each consolidated Z-set weight rather than discarding it.

That retained weight is what lets a test assert multiplicity and retractions.
A record pushed twice consolidates into one record with weight `2`, so a
deduplicated output can be asserted to have multiplicity `1` — which
distinguishes "emitted once" from "emitted twice and collapsed only when
read".

```rust
use dbsp::RootCircuit;
use test_utils::collect_weighted;

let (circuit, (input, output)) = RootCircuit::build(|circuit| {
    let (stream, handle) = circuit.add_input_zset::<i64>();
    Ok((handle, stream.output()))
})?;

// Pushing the same record twice consolidates to one record of weight 2.
input.push(7, 1);
input.push(7, 1);
circuit.step()?;

assert_eq!(collect_weighted(&output), vec![(7, 2)]);
```

## Dependency resolution constraints

No `Cargo.lock` is committed, so Cargo resolves the graph afresh on every
machine and every CI run. A broken release of a transitive dependency
therefore reaches the build the day it is published, and the only lever is a
direct requirement in `Cargo.toml`.

`tinyvec = "~1.12"` is such a lever. It is a direct Cargo dependency, but no
source file in this repository names the crate: it arrives through Bevy's text
and font stack, and the requirement exists only to bound what Cargo resolves.

Version 1.13.0, published on 2026-09-03, imports `alloc::vec` as a module and
then invokes the `vec!` macro, which the pinned nightly rejects with
``cannot find macro `vec` in this scope``. Every `--all-features` build and
`make lint` fail on it. The requirement is a tilde rather than a caret because
it must hold at 1.12 until the breakage is fixed; `AGENTS.md` permits that for
a documented lock to patch-level updates.

`tests/dependency_resolution.rs` reads what Cargo resolved and fails if 1.13 or
later was selected, naming the version. It is a post-resolution check, not a
pre-build gate: Cargo compiles the graph before an integration test runs, so a
selected 1.13.0 fails the build first with the macro error above. What the test
catches directly is the case that would otherwise pass silently, a widened
requirement whose resolved version still compiles but sits outside the range
this workspace has verified. Diagnose a build that fails this way with
`cargo tree --invert tinyvec`.

Remove the requirement and the test together once upstream ships a fixed
release or yanks 1.13.0, tracked in
[issue 340](https://github.com/leynos/lille/issues/340). Upstream discussion is
in Lokathor/tinyvec#225 and Lokathor/tinyvec#226.

A future constraint of this kind belongs here too: a bare version requirement
with no explanation is indistinguishable from a real dependency, and the next
contributor will not know whether removing it is safe.

## Commit gates

Run the deterministic gates before committing (see `AGENTS.md` and the
`Makefile`): `make check-fmt`, `make test`, `make typecheck`, and `make lint`.
`make test` passes `--features test-support`, so it also runs the
buffered-message compile-pass harness
(`cargo test --features test-support --test compile_pass`; see
[Buffered events use the Message API](#buffered-events-use-the-message-api)).
`make lint` runs rustdoc (`--cfg docsrs`),
`cargo clippy --all-targets --all-features -- -D warnings`, and the Whitaker
Dylint suite.

## Continuous integration

Two workflows do the developer-blocking work. `ci.yml`'s `build-test` job runs
on every pull request, and `coverage-main.yml`'s `coverage-upload` job runs on
every push to `main`. Both use the `ubicloud-standard-8` runner label, which is
registered in `.github/actionlint.yaml`, and both declare `timeout-minutes` so
a hung step cannot bill to the platform's six-hour default.

Every other job stays on GitHub-hosted `ubuntu-latest`. That placement is a
rule, not an accident: delayed comments, metadata lookups, label handling, and
release orchestration are API-bound, so paid runner capacity buys them nothing
and their queue time is already short. `dependabot-automerge.yml` calls a
reusable workflow, which chooses its own runner.

### Tool installation

No tool is compiled from source. `whitaker-installer` is installed by
`leynos/shared-actions/.github/actions/install-whitaker`, which downloads the
pinned prebuilt release archive and verifies it against a digest pinned inside
the action, then runs the installer to place the Whitaker Dylint suite. Every
`leynos/shared-actions` reference pins commit
`3a2f2d5f17932657ddf50490a09ea5e7400ae35c`.

sccache is installed the same way, by `taiki-e/install-action` with
`tool: sccache@0.16.0` and `fallback: none`. The fallback matters: without it
the action would compile sccache from source when no prebuilt binary matched,
which is the outcome the rule exists to prevent.

### Cache ownership

Each mutable path has exactly one owner, so no two steps race to write it and
every miss is explainable from the rendered key.

| Path                                                                             | Owner                                         | Key inputs                                                                           |
| -------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------ |
| `~/.cargo/registry`, `~/.cargo/git`                                              | `setup-rust` (`cache-provider: github`)       | `runner.os`, `rust-toolchain.toml` and `Cargo.lock` hash                             |
| `~/.cargo/bin/whitaker-installer`, its version marker, `~/.local/share/whitaker` | `install-whitaker` (`cache-provider: github`) | `runner.os`, `runner.arch`, installer version, `dylint.toml` hash                    |
| `.uv-cache`, `.uv-tools`                                                         | the `Cache uv tool layers` step in `ci.yml`   | `runner.os`, `runner.arch`, `runner.environment`, `Makefile` and `scripts/*.py` hash |
| coverage ratchet baseline files                                                  | `generate-coverage`'s split restore and save  | `runner.os`, run id                                                                  |
| compiler output                                                                  | `sccache`, through its GitHub Actions backend | compiler flags and toolchain, hashed by sccache itself                               |

*Table 1: Cache ownership and cache-key inputs.*

`generate-coverage` is called with `cache-provider: external` in both jobs
because `setup-rust` already owns the Cargo registry and Git index; without
that input the action would become a second owner of the same two paths. For
the same reason no step archives a `target` tree. Every `actions/cache`
reference written in these workflow files pins
`55cc8345863c7cc4c66a329aec7e433d2d1c52a9` (v6.1.0). That claim covers the
workflow files only. A shared action may reach an `actions/cache` reference of
its own, and `upload-codescene-coverage` does; the next paragraph records it.

### The compiler cache

sccache owns compiler output, and nothing archives a `target` tree. Both build
jobs wire it up as four steps in a fixed order, and the order is the whole
point: get it wrong and the cache is silently a no-op.

The job sets `RUSTC_WRAPPER: sccache` and `SCCACHE_GHA_ENABLED: 'true'` at job
level. The first engages the wrapper; the second is what selects the GitHub
Actions backend. Without the second, sccache falls back to
`Local disk: ~/.cache/sccache`, which nothing persists between runs, so the
wrapper becomes pure overhead. `CARGO_INCREMENTAL: '0'` accompanies them
because sccache cannot cache an incremental compilation.

**Export.** A pinned `actions/github-script` step, after checkout, re-exports
`ACTIONS_CACHE_URL` and `ACTIONS_RUNTIME_TOKEN` into `GITHUB_ENV` and clears
`ACTIONS_CACHE_SERVICE_V2`. The runner gives those two variables to action code
but not to later shell steps, and sccache's backend reads them from the
environment. On Ubicloud `ACTIONS_CACHE_URL` names the runner's local cache
proxy, so re-exporting it is what puts the compiler cache in Ubicloud's store
rather than GitHub's. The v2 cache service bypasses that proxy, so it is
cleared; exporting `ACTIONS_RESULTS_URL` does not route through the proxy
either. The step also logs whether an endpoint and a token were present, and
warns when either is missing, so a misconfigured backend is diagnosable from
the log rather than from an unexplained slow build. It never prints the token.

**Install.** `taiki-e/install-action` places the pinned sccache binary. An
action step is safe here because installing a binary does not start the sccache
server.

**Start.** A `run:` step runs `sccache --zero-stats`, which starts the server.
This must be a `run:` step and it must follow the export. The Ubicloud runner
re-injects `ACTIONS_CACHE_SERVICE_V2=on` and `ACTIONS_RESULTS_URL` into every
action step, overriding what the export wrote to `GITHUB_ENV`. A server started
inside an action step, which is what `setup-rust` with `use-sccache: 'true'`
would do, therefore binds GitHub's v2 service; its writes then fail and nothing
reaches Ubicloud's store. The server binds its backend once, at start, so a
later change to the environment is invisible to it. That is why `setup-rust` is
called with `use-sccache: 'false'` in both jobs.

**Report.** `sccache --show-stats` runs after the build, printing the counters
to the log as well as to the job summary. The log copy is the one that matters:
the summary cannot be read through the REST API, so it cannot be checked after
the fact. Read `Cache location` on every run. It must name the GitHub Actions
backend; `Local disk: ~/.cache/sccache` means the backend was never selected
and nothing is being cached. A warm build reporting zero hits is a broken
contract, not a slow one.

Each job also deletes `target/llvm-cov-target` once coverage has been
generated, printing `df -h` either side. The instrumented tree has no later
consumer, and on smaller runner shapes a full disk has killed a job silently,
with no error text.

### Resource sampling

Both build jobs start a background sampler after checkout that records used
memory and used and free disk every 15 seconds, and report the peaks at the
end of the job. The `ubicloud-standard-8` label was inherited rather than
measured, so the samples are what a future decision to keep or shrink the shape
will rest on. Disk is sampled alongside memory because disk, not memory, is
what has exhausted runners in this estate, and it did so with no error text at
all.

`ci.yml` accepts `workflow_dispatch` so a warm run can be measured on demand.
A dispatch restores what a pull request restores and writes nothing:
`coverage-main.yml` is the only job that saves on this repository.

One download remains deliberately uncached. The `cs-coverage` CLI is fetched on
every run because `upload-codescene-coverage` only caches it when `cli-version`
is pinned, and its cache step uses an unpinned `actions/cache@v4` that this
repository cannot pin from here.

The uv tool layers are an exception to the trunk-writer rule rather than to the
ownership rule. They are cached by the pull-request job that installs them,
which is also the only job that installs them, so there is no trunk job to
designate as the sole writer instead.

### One test execution per pull request

The instrumented coverage run is the only test execution on Linux. It uses
`all-features`, `all-targets`, and `doctests`, so it covers everything the
former separate `cargo test` step covered and more, for one compile rather than
two. A workflow contract in `tests/workflow_contracts.rs` fails if a second
`cargo test` or `cargo nextest` step reappears in either job.

### Workflow contracts

`tests/workflow_contracts.rs` asserts the rules
above: pinned cache and shared-action references, no source-built tools, one
owner per cached path, GitHub-hosted placement for non-build jobs, registered
runner labels, an installer before the first use of what it installs, and a
single test execution per build job. It also pins the inputs that make those
rules true, so a workflow cannot keep the shape of the policy while dropping
its substance: `cache-provider`, `use-sccache`, the Whitaker installer
version, the coverage flags, the uv cache paths and key, a bounded
`timeout-minutes` for each build job, the resource sampler and its report, and
the compiler-cache wiring: the two job-level variables, and the export,
install, start, build, report order that the sccache server's one-shot backend
binding depends on.

`tests/support/workflow_model.rs` holds the job, step, and runner-selection
types the properties and the contracts share;
`tests/support/workflow_estate.rs` holds the pinned commits, the whole-file
`Workflow` type, and the errors parsing reports, which only the contracts need.
`tests/support/workflow_loader.rs` turns files into those values.

Parsing is strict about shape and permissive about spelling. A field that is
present but of the wrong type is an error rather than a silent default, because
a contract that read an empty string for a mistyped `runs-on` would pass a
workflow it should reject. A step that sets both `uses` and `run` is rejected
too: GitHub Actions runs a step one way or the other, never both. Against that,
every form the platform genuinely accepts must parse. `runs-on` may be a label,
a list of labels, or a mapping naming a runner group, and `on` may be an event,
a list, or a mapping, read under the bare key that YAML 1.1 turns into the
boolean true. The files are read through a `cap_std` directory capability
rooted at `.github/workflows`.

Action references are matched on the whole coordinate before the `@`, publisher
included. A suffix match would let `untrusted/setup-rust` satisfy a rule
written about the shared `setup-rust`, which is the opposite of what a pinning
rule is for.

Two assurance methods are used together, following
[ADR 003](adr-003-bounded-rstest-over-property-testing.md).
`tests/workflow_contracts.rs` holds bounded `rstest` cases over the workflow
files as they stand, and `tests/workflow_model_properties.rs` samples the
wider domain with `proptest`: arbitrary step orderings, repeated display
names, interleaved unrelated steps, and split caches whose halves agree or
disagree on a key, or where a third step claims a paired key. The properties
check cache-owner uniqueness and installer-ordering against small oracles
written independently of the implementation. Run both with `make test`, and run
`actionlint` after editing any workflow.

Only one restore and one save sharing a key count as a single owner. Two
restores on the same key are two owners, and so are a matching pair plus a
third step, because otherwise a genuine duplicate could hide behind the
split-cache exception.
