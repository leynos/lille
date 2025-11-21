# Bevy 0.16+ Migration Plan

## Goal

Upgrade Lille from Bevy 0.12 to Bevy 0.17.3 so we can adopt `bevy_ecs_tiled`
0.9+ for the planned map pipeline, while keeping the DBSP synchronisation flow
stable and headless builds reproducible.

## Current State Snapshot

- `bevy`, `bevy_ecs`, `bevy_app`, and related crates are pinned to 0.12 with
  most default features disabled.
- Rendering is optional behind the `render` feature; headless CI depends on
  MinimalPlugins.
- `DbspPlugin` relies on `NonSend` resources, chained `Update` systems, and
  manual access to `app.world`.
- Rendering entrypoints are limited to `DefaultPlugins` plus a demo
  `spawn_world_system` that spawns sprites and a `Camera2dBundle`.

## Drivers and External Constraints

- `bevy_ecs_tiled` 0.9 requires Bevy ≥ 0.16 and provides the APIs referenced by
  our map design docs.[^1]
- Bevy 0.13–0.17 introduce new observer-based events, the Required Components
  model, GPU-driven renderer revisions, and automatic reflection registration,
  so incremental upgrades keep risk manageable.[^2][^3][^4][^5][^6]
- The Bevy team expects consumers to track the latest stable Rust toolchain;
  our pinned nightly is new enough but needs validation after each bump.

## Compatibility Inventory

### Core App (`src/main.rs`)

- Uses `App::new()` plus `DefaultPlugins.build().disable::<LogPlugin>()` and
  adds custom plugins. The `build().disable()` idiom is still supported but now
  has `DefaultPlugins.set()` alternatives; keep tests for either path.
- CLI parsing (clap) and logging bootstrapping remain unaffected.

### Gameplay and DBSP layers (`src/dbsp_*`)

- Heavy use of `NonSend` resources, `ResMut`, and manual world access is still
  supported in 0.17, but `World` APIs gained additional borrow checking. Add
  targeted compile tests during each bump.
- Systems rely on tuples of components in queries and `.chain()` ordering. The
  chaining API survives through 0.17, yet Observers / Events V2 unlock leaner
  alternatives. Capture follow-up tasks but avoid refactors mid-upgrade.

### Rendering and Assets (`src/spawn_world.rs`)

- Uses `SpriteBundle` and `Camera2dBundle`. Bevy 0.15 deprecated most bundles
  in favour of Required Components so we must migrate to spawning the
  components directly and rely on the new default requirements.[^4]
- We only use solid-colour sprites and have no textures or meshes, so the new
  asset usage flags introduced in 0.13 do not block the upgrade. Keep an eye on
  `RenderAssetUsages` once textured sprites arrive.

### Tooling and CI

- `rust-toolchain.toml` pins `nightly-2025-09-14`. Validate nightly vs the
  Bevy MSRV (latest stable) and be ready to pin e.g. `1.82` if nightly
  regressions show up.
- CI scripts call `make fmt|lint|test`; no Bevy-specific runners exist yet, so
  new checks (wasm, feature combos) must be added explicitly.

## Subsystem ownership

| Subsystem                | Owner(s)                 |
| ------------------------ | ------------------------ |
| Render                   | Leynos / Payton McIntosh |
| Testing and CI           | Leynos / Payton McIntosh |
| DBSP circuit integration | Leynos / Payton McIntosh |

Lille currently has a single maintainer, so the same person covers both owner
and reviewer duties. Whenever a change is high risk (render regressions, DBSP
semantics, CI infra), queue an ad-hoc reviewer from the wider contributors list
before merging to keep the “two sets of eyes” policy meaningful.

## Execution Phases

### Phase 0 – Pre-flight

- Confirm no local forks of Bevy crates exist by running `cargo tree -i bevy`.
- Capture baseline artefacts: `cargo check --all-features`, `cargo test`, and a
  short render smoke test to compare behaviour later.
- Document owners for each subsystem (render/test/DBSP) and line up reviewers.

#### Phase 0 baseline (18 November 2025)

- Logs for the required commands live under
  `artifacts/bevy-0-17-upgrade/phase-0/`. The workflow and observations are
  documented in `docs/migrations/bevy-0-17-phase-0.md` so future phases can
  reuse the same scripts and diff the results.
- Render smoke testing uses
  `RUST_LOG=info timeout 5s cargo run -p lille --features render -- --verbose`
  to avoid hanging CI while still exercising window creation and DBSP
  synchronization. The timeout-induced exit status is expected.

### Phase 1 – 0.12 → 0.13

- Bump `bevy*` crates to 0.13.*, keeping feature flags unchanged.
- Regenerate `Cargo.lock`; expect new `wgpu`/`winit` transitive updates.
- Run `cargo clippy` with existing deny rules. Address any new lint warnings,
  especially around `Query` lifetimes because `apply_deferred` is now handled
  automatically.[^6]
- Smoke-test MinimalPlugins scenarios (unit tests and `DbspPlugin` tests).

#### Phase 1 status (18 November 2025)

- Dependency graph now targets Bevy 0.13.2; see
  `docs/migrations/bevy-0-17-phase-1.md` plus the recorded outputs in
  `artifacts/bevy-0-17-upgrade/phase-1/`.
- Added `tests/physics_bdd/dbsp_authority.rs`, a combined `rstest` and
  `rust-rspec` scenario that exercises the MinimalPlugins + `DbspPlugin` path
  for both happy and unhappy cases, proving the DBSP circuit remains the
  authority after the scheduler changes in 0.13.

### Phase 2 – 0.13 → 0.14

- Update dependencies to 0.14.*.
- Review any `FixedTimestep` or scheduling usages (none today) but ensure test
  helpers relying on stage order still work because schedule timing changed in
  0.14.[^5]
- Validate that `World::get_non_send_resource` invocations remain correct under
  the new schedule. Add regression tests if we rely on deterministic order.
- Rebuild docs to verify that rustdoc output still compiles with new lint
  defaults.

### Phase 3 – 0.14 → 0.15

- Upgrade to 0.15.* and switch from deprecated bundles to Required Components
  in `spawn_world_system`. Spawn sprites via `Sprite` + `Transform` +
  `Visibility` components and cameras via `Camera2d` + `Projection`.
- Audit `Camera2dBundle::default()` replacements such as `Camera2d` +
  `Msaa::Sample4` if needed to retain anti-aliasing.
- Run the render feature locally to catch any material or visibility
  regressions introduced by the new bundle model.[^4]

### Phase 4 – 0.15 → 0.16

- Move to 0.16.* and adopt the improved spawn API where possible (e.g. chaining
  `.with_children()` or builder closures) because ECS relationships now support
  faster hierarchy lookups.[^3]
- Evaluate whether DBSP systems that push or retract events could benefit from
  Observers V1 (0.16) to simplify event routing. Log follow-up tickets instead
  of mixing refactors into the bump.
- Validate linux-only dependencies (`x11`) against the new Bevy window stack.

### Phase 5 – 0.16 → 0.17.3

- Final bump to 0.17.3. Update `bevy_log` usages to the renamed Observer +
  Event APIs if we opt in, and adopt Reflect auto-registration to simplify any
  future `App::register_type` calls.[^2]
- Ensure `DbspPlugin` error handling aligns with the Events V2 changes so that
  diagnostics continue to surface in logs.
- Re-run all feature combos (`default`, `render`, `text`) plus `cargo test
  --all-features` and `cargo test -p
  build_support` to confirm the workspace is stable on the new stack.

### Phase 6 – Map enablement after 0.17.3 lands

- Add `bevy_ecs_tiled = "0.9"` and wire the plugin behind a new `map` feature,
  keeping compatibility with headless runs.[^1]
- Follow the existing design docs to register Lille components with
  `bevy_ecs_tiled` and stream tiles into the DBSP bridge.
- Update docs/roadmaps to mark the Bevy dependency as unblocked.

## Testing and Validation Strategy

- For each phase, run `make fmt`, `make lint`, `make test`, and
  `cargo test --all-features`. Capture logs to compare against baseline.
- Add a quick headless Bevy app smoke test (e.g. run `cargo run --bin lille` in
  CI with `render` enabled on a xvfb display) once at Phase 3 and once at Phase
  5.
- Use `cargo tree` diffs to ensure no unwanted feature flags leak in.
- Track bench metrics (frame time inside DBSP integration test) to confirm no
  regressions slip in when the renderer or scheduler changes.

## Risks and Mitigations

- **Bundle removal surprises:** tackle bundle-to-component rewrites on a branch
  before the official version bump so failures are isolated.
- **Nightly compiler drift:** if nightly breaks Bevy, fall back to the Bevy
  MSRV and update `rust-toolchain.toml` accordingly.
- **Hidden transitive updates:** pin any newly introduced GPU or window
  features via `Cargo.toml` to avoid unexpected toggles in downstream crates.
- **Docs drift:** every phase should update relevant docs plus the changelog to
  avoid confusing contributors about the active Bevy version.

## Acceptance Checklist

- [ ] All Bevy crates, plus `bevy_ecs_tiled`, pinned to the new versions.
- [ ] Sprite/camera spawning migrated to Required Components with render smoke
      tests recorded.
- [ ] `DbspPlugin` tests and map integration docs updated.
- [ ] CI matrix extended with at least one render-enabled run.
- [ ] Roadmap and README mention the new minimum Bevy version.

[^1]: <https://docs.rs/bevy_ecs_tiled/0.9.1/bevy_ecs_tiled/>
[^2]: <https://bevyengine.org/news/bevy-0-17/>
[^3]: <https://bevyengine.org/news/bevy-0-16/>
[^4]: <https://bevyengine.org/news/bevy-0-15/>
[^5]: <https://bevyengine.org/news/bevy-0-14/>
[^6]: <https://bevyengine.org/news/bevy-0-13/>
