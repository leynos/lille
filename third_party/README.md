# Vendored third-party crates

This directory holds minimal, temporary forks of upstream crates that Lille
depends on. Each fork is wired into the build through `[patch.crates-io]` in
the workspace `Cargo.toml`, and is excluded from the workspace via
`[workspace] exclude`.

## Scope: carried, not adopted

These crates are carried verbatim from upstream to deliver a specific fix, not
adopted as maintained Lille code. Except for the deliberately minimal changes
listed for each fork below, the source is byte-identical to the named upstream
release, and Lille does not hold it to this repository's code-health, testing,
documentation, or lint standards (hence the crate-level `#![allow(warnings)]`
and the CodeScene `third_party/**` overrides).

Review feedback about the upstream code that is outside the scope of the
vendored fix — for example requests to test, refactor, or re-architect upstream
functionality Lille does not use — is therefore out of scope here. Raise such
concerns upstream against the source project instead. In-scope feedback is
anything touching the fix itself or the wiring that carries it.

## `feldera-size-of`

A fork of [`feldera-size-of`](https://github.com/feldera/size-of) `0.1.7`
(crates.io), based on Cargo's normalized manifest for that release.

### Why it exists

Lille uses `ordered-float` 5.x and derives `feldera_size_of::SizeOf` on DBSP
records that embed `OrderedFloat<f64>` (for example `Position` and
`BlockSlope`). Every published `feldera-size-of` release — up to and including
`0.1.7` — pins its optional `ordered-float` dependency at `^3.0.0`, so its
`SizeOf` impl for `OrderedFloat`/`NotNan` applies only to ordered-float 3.x.
Against ordered-float 5.x the derive fails to compile with:

```plaintext
error[E0277]: the trait bound `ordered_float::OrderedFloat<f64>:
feldera_size_of::SizeOf` is not satisfied
```

Upstream `main` still pins `ordered-float = "3.0.0"` at the time of writing, so
there is no release to upgrade to.

### What was changed from upstream `0.1.7`

The changes are deliberately minimal:

- **`Cargo.toml`** — the optional `ordered-float` dependency is widened from
  `3.0.0` to `5`. The `size-of-derive` `path` dependency is already resolved to
  the published `0.1.2` in the normalized manifest, so no derive-crate
  vendoring is required.
- **`src/support/ordered_float.rs`** — the `SizeOf` impl bound is switched from
  `Float` to `FloatCore`. ordered-float 5.x bounds its `Deref` impls for
  `OrderedFloat<T>`/`NotNan<T>` on `FloatCore` rather than `Float`, and the
  impl relies on the `&OrderedFloat<T>` → `&T` deref coercion. `Float` does not
  imply `FloatCore`, so without this the impl no longer type-checks. This is
  the only source change required by the ordered-float 5.x upgrade.
- **`src/tests/mod.rs`** — adds a unit test demonstrating
  `OrderedFloat<f64>: SizeOf`, and removes the upstream `ui` trybuild test.
- **`src/lib.rs`** — besides the `#![allow(warnings)]` lint cap and the inline
  crate doc (replacing `include_str!("../README.md")`), adds crate-level
  `#![cfg_attr(coverage_nightly, feature(coverage_attribute))]` and
  `#![cfg_attr(coverage_nightly, coverage(off))]`. As a path dependency the
  fork's source lives under the workspace root, which cargo-llvm-cov's default
  ignore list does not exclude (it only skips the registry, git checkouts, the
  target dir, and the toolchain); the largely-unexercised upstream code would
  otherwise dilute the coverage denominator. cargo-llvm-cov sets
  `cfg(coverage_nightly)` on nightly runs, so the attributes are inert during
  ordinary builds.
- The `src/tests/pass/*.rs` trybuild fixtures and packaging cruft
  (`Cargo.toml.orig`, `Cargo.lock`, `.cargo_vcs_info.json`, `release.toml`,
  `.github/`) are removed. The trybuild fixtures and the crate's doctests use
  the `size-of-derive` macro, which expands to the `::size_of` crate path; that
  only resolves when the crate is imported aliased as `size-of` (as Lille
  does). Built standalone the crate is `feldera_size_of`, so they cannot
  compile here. Lille consumes the crate as a library under its `size-of`
  alias, so this does not affect Lille; the library and its unit tests build
  cleanly.

### Removing this fork

Delete `third_party/feldera-size-of/`, drop the `[patch.crates-io]` entry and
the `[workspace] exclude` line from the workspace `Cargo.toml`, and let Lille
depend on the upstream release directly, once `feldera-size-of` publishes a
release whose `ordered-float` constraint accepts 5.x.
