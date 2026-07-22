# Architectural decision record (ADR) 002: ordered-float v5 via a vendored feldera-size-of fork

## Status

Accepted, 2026-07-21. Lille moved to ordered-float 5.x by vendoring a minimal,
temporary fork of `feldera-size-of` under `third_party/feldera-size-of`,
patched in via `[patch.crates-io]`.

## Date

2026-07-21.

## Context and Problem Statement

Lille embeds `OrderedFloat<f64>` in DBSP records (for example `Position` and
`BlockSlope`) and derives `feldera_size_of::SizeOf` on those types. Every
published `feldera-size-of` release up to and including `0.1.7` — and the
`main` branch upstream — pins its optional `ordered-float` dependency at
`^3.0.0`. Its `SizeOf` impl for `OrderedFloat`/`NotNan` therefore applies only
to ordered-float 3.x. Against ordered-float 5.x the derive fails to compile
with:

```plaintext
error[E0277]: the trait bound `ordered_float::OrderedFloat<f64>:
feldera_size_of::SizeOf` is not satisfied
```

Lille cannot implement `SizeOf` for `OrderedFloat` itself: both the trait and
the type are foreign to the crate, so the orphan rule forbids the impl.

Complicating matters, `dbsp` 0.98 independently requires
`ordered-float ^4.2.0`, so ordered-float 4.x and 5.x necessarily coexist in the
dependency graph regardless of how this problem is resolved.

## Decision Drivers

- Lille needs ordered-float 5.x for reasons outside the scope of this ADR
  (tracked separately in the branch history); this ADR only concerns the
  compatibility break in `feldera-size-of`.
- The orphan rule rules out a local `impl SizeOf for OrderedFloat<f64>`.
- No upstream `feldera-size-of` release, and no commit on its `main` branch,
  accepts ordered-float 5.x.
- Whatever fix is chosen must not silently dilute the workspace's
  `cargo-llvm-cov` coverage figures with unexercised upstream code.

## Options Considered

### Option A: stay on ordered-float v3

Keep the workspace pinned to ordered-float 3.x so the upstream `feldera-size-of`
`SizeOf` impls continue to apply unmodified.

Rejected: this blocks the ordered-float upgrade outright and leaves the
dependency graph carrying an outdated `ordered-float` major version
indefinitely.

### Option B: wait for, or adopt, an upstream feldera-size-of release

Track the upstream `feldera/size-of` repository and pick up a released version
once it accepts ordered-float 5.x.

Rejected for now: no released version, and no commit on upstream `main`,
supports ordered-float 5.x at the time of this decision. Adopting this approach
later is the intended exit path (see Removal Criteria).

### Option C: patch to a git fork

Publish the widened-constraint fork as a Git repository and patch it in via
`[patch.crates-io]` with a `git` source instead of a `path` source.

Noted but not adopted: no suitable fork host under the project's control was
available at decision time. A git-sourced patch would also have sidestepped the
coverage dilution problem, since `cargo-llvm-cov`'s default ignore list already
excludes git checkouts (it only skips the registry, git checkouts, the target
directory, and the toolchain) — unlike path dependencies, which live under the
workspace root and are not excluded.

### Option D (chosen): vendor a minimal path-dependency fork

Vendor a minimal fork of `feldera-size-of` `0.1.7` under
`third_party/feldera-size-of`, wire it in via `[patch.crates-io]` as a path
dependency, and exclude it from the workspace so Cargo does not fold it into
workspace-wide lints, tests, or member resolution.

| Topic                       | Option A (stay on v3) | Option B (wait upstream) | Option C (git fork)           | Option D (vendored path fork)     |
| --------------------------- | --------------------- | ------------------------ | ----------------------------- | --------------------------------- |
| Unblocks ordered-float 5.x  | No                    | No (not yet available)   | Yes                           | Yes                               |
| Requires infra not in place | N/A                   | N/A                      | Yes (fork host)               | No                                |
| Coverage denominator impact | None                  | None                     | None (auto-excluded)          | Requires explicit `coverage(off)` |
| Maintenance burden          | None                  | None until released      | Track upstream security fixes | Track upstream security fixes     |

_Table 1: Comparison of options for restoring `feldera-size-of` compatibility
with ordered-float 5.x._

## Decision Outcome

Vendor a minimal fork of `feldera-size-of` `0.1.7` under
`third_party/feldera-size-of`, based on Cargo's normalized manifest for that
release, and redirect the crate to it via `[patch.crates-io]`.

The fork's changes from upstream `0.1.7` are deliberately minimal:

- `Cargo.toml`: the optional `ordered-float` dependency constraint is widened
  from `3.0.0` to `5`.
- `src/support/ordered_float.rs`: the `SizeOf` impl bound is switched from
  `Float` to `FloatCore`. ordered-float 5.x bounds its `Deref` impls for
  `OrderedFloat<T>`/`NotNan<T>` on `FloatCore` rather than `Float`, and the
  impl relies on the `&OrderedFloat<T>` → `&T` deref coercion; `Float` does not
  imply `FloatCore`, so without this change the impl no longer type-checks.
  This is the only source change the ordered-float 5.x upgrade requires. `rkyv`
  support stays on 0.7 via the `rkyv_64` feature, so no `rkyv` version change
  was needed.
- `src/lib.rs`: adds a crate-level `#![allow(warnings)]`, because Cargo does
  not apply `--cap-lints allow` to path dependencies the way it does to
  registry dependencies, and adds
  `#![cfg_attr(coverage_nightly, coverage(off))]`. As a path dependency the
  fork's source lives under the workspace root, which `cargo-llvm-cov`'s
  default ignore list does not exclude (it only skips the registry, git
  checkouts, the target directory, and the toolchain); without this attribute
  the largely-unexercised upstream code diluted the coverage denominator,
  dropping measured coverage from approximately 89.5% to approximately 72.9%.
- `.codescene/code-health-rules.json` gains a `third_party/**` rule set that
  disables the "Code Duplication" check, since the fork's per-type `SizeOf`
  impls are upstream code with an intentionally uniform structure that this
  repository does not maintain and does not hold to its own duplication
  standard.

The workspace wiring:

```toml
[workspace.dependencies]
ordered-float = { version = "5", features = ["serde", "rkyv_64"] }

# `feldera-size-of` 0.1.x pins its optional `ordered-float` dependency at
# `^3.0.0`, so its `SizeOf` impl for `OrderedFloat`/`NotNan` does not apply to
# the ordered-float 5.x used across this workspace. Until upstream publishes
# ordered-float 5.x support (https://github.com/feldera/size-of), redirect the
# crate to a minimal vendored fork that widens the constraint. Remove this
# patch and the `third_party/feldera-size-of` directory once an upstream
# release ships.
[patch.crates-io]
feldera-size-of = { path = "third_party/feldera-size-of" }

[workspace]
# The vendored `feldera-size-of` fork is wired in via `[patch.crates-io]`, not
# as a workspace member; exclude it so Cargo does not fold it into this
# workspace.
exclude = ["third_party/feldera-size-of"]
```

Full details of the fork's provenance and changes are recorded in
[`third_party/README.md`](../third_party/README.md).

## Known Risks and Limitations

- The fork will not receive upstream security fixes automatically; any future
  `feldera-size-of` patch releases must be evaluated and ported manually.
- The fork is temporary by design and is expected to be removed once an
  upstream release resolves the underlying constraint (see Removal Criteria).
- The `#![allow(warnings)]` crate-level attribute suppresses all lint output
  for the vendored source, which is appropriate for unmaintained upstream code
  but would mask genuine issues if the fork's own source were later modified
  beyond the changes documented above.

## Removal Criteria

Once `feldera-size-of` publishes a release whose `ordered-float` constraint
accepts 5.x:

1. Delete `third_party/feldera-size-of/`.
2. Drop the `[patch.crates-io]` entry from the workspace `Cargo.toml`.
3. Drop the `third_party/feldera-size-of` line from `[workspace] exclude`.
4. Depend on the upstream release directly.
