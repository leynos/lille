# Developer guide

Practical notes for working on Lille that are not obvious from the source
alone. This guide currently focuses on the temporary `ordered-float` v5
compatibility arrangement; extend it as further cross-cutting concerns arise.

## `ordered-float` v5 and the vendored `feldera-size-of` fork

Lille's DBSP records store floating-point values through
`ordered_float::OrderedFloat<f64>` so that they have a total order, which DBSP
requires for keys, joins, and aggregations. Those records also derive
`feldera_size_of::SizeOf` for memory accounting. Reconciling the two across a
major `ordered-float` upgrade is the reason for the arrangement described here.

For the full decision record, see
[ADR 002](adr-002-ordered-float-v5-vendored-feldera-size-of-fork.md). The fork
itself is documented in
[`third_party/README.md`](../third_party/README.md). Fork lifecycle and removal
are tracked in
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

The Lille-owned guard for this arrangement is
`tests/ordered_float_size_of.rs`, a compile-time integration regression test
asserting that `OrderedFloat<f64>`, `NotNan<f64>`, `Position`, and `BlockSlope`
implement `SizeOf`. It is Lille code and is maintained normally.

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
