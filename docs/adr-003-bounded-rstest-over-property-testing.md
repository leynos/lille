# Architectural decision record (ADR) 003: bounded `rstest` matrices over a property-testing framework

## Status

Accepted, 2026-07-26.

## Date

2026-07-26.

## Context and Problem Statement

Several invariants in the DBSP-sync and map-lifecycle code must hold
*exactly*, not merely "usually", because they govern correctness properties
such as at-most-one decision per entity, exact frame rollback, and rejection
of unsafe asset paths. Property-testing frameworks (`proptest`, Kani, Verus)
are the conventional tool for this class of invariant, generating many inputs
per run (or, for Kani/Verus, proving over the whole input domain) rather than
relying on a handful of hand-picked cases.

None of `proptest`, Kani, or Verus is a workspace dependency (checked against
the workspace `Cargo.toml`). Adopting one to cover these invariants would be
a new supply-chain decision, not a reuse of existing tooling.

## Decision Drivers

- The invariants concerned each have a small, **finite** input space: a
  handful of weight combinations, a handful of path-component forms, a
  handful of rollback/commit orderings.
- The `rstest` stack is already used throughout the codebase's test suite
  (see [Mastering test fixtures in Rust with `rstest`](
  rust-testing-with-rstest-fixtures.md)), so a case-matrix approach reuses
  existing infrastructure.
- Adding `proptest`, Kani, or Verus is a supply-chain cost (new dependency,
  new tool to install and maintain) that this testing strategy does not need
  to pay if exhaustive enumeration is achievable by hand.

## Options Considered

### Option A: adopt `proptest`

Generate randomly-sampled inputs (with shrinking on failure) for the
invariants under test.

Rejected for these invariants: `proptest` samples a subset of a space rather
than enumerating it exhaustively, and for spaces this small a hand-written
matrix can cover every case rather than a sample. `proptest` is also not
currently a workspace dependency.

### Option B: adopt Kani or Verus

Use bounded model checking (Kani) or a proof assistant (Verus) to prove the
invariant holds over its entire input domain.

Rejected for now: neither is a workspace dependency, both carry a
significantly heavier toolchain and authoring cost than a test matrix, and
for input spaces small enough to enumerate by hand, a proof tool is more
machinery than the problem requires.

### Option C (chosen): bounded, exhaustive `rstest` case matrices

Enumerate every case in the finite input space as an explicit `rstest` case,
using the `rstest` stack already in use throughout the codebase.

## Decision Outcome

For the DBSP-sync and map-lifecycle invariants listed below, cover the
invariant with a bounded, exhaustive `rstest` case matrix rather than
adopting a property-testing or proof framework:

- **Movement-decision dedupe** — one decision per entity, none for net-zero
  total weight, and a consolidated Z-set multiplicity of exactly `1` for the
  emitted decision. Canonical example:
  `dedupe_emits_one_decision_for_positive_weight_and_none_for_zero` in
  `src/dbsp_circuit/streams/behaviour/decide/tests.rs`.
- **Frame rollback** — exact pre-frame restoration of `health_snapshot`,
  `pending_damage_retractions`, and `applied_unsequenced` on a failed
  `step_circuit()` call, versus retention of the frame's changes on commit.
  Canonical example: `applied_unsequenced_rollback_matrix` in
  `src/dbsp_sync/state.rs`.
- **Asset-path validation** — rejection of rooted paths and standalone `..`
  traversal components, versus acceptance of relative paths where `..`
  appears only as a substring. Canonical example:
  `validate_asset_path_component_matrix` in `src/map/lifecycle/tests.rs`.

Because each of these input spaces is small and finite, an `rstest` matrix
enumerating every case covers the *entire* space — every relevant
combination, not a sampled subset of it — which is the property a
property-testing framework would otherwise be adopted to obtain. This ADR is
the documented, approved exception to defaulting to a property-testing or
proof framework for exactly-correctness invariants: it applies only to the
invariants named above and any future invariant with an equivalently small,
finite input space.

## Known Risks and Limitations

- The exhaustiveness argument is only sound while the input space stays
  small enough to enumerate by hand. If an invariant's input space grows —
  for example, if the number of independent weight/ordering dimensions
  increases — a hand-written matrix risks silently becoming a sample rather
  than remaining exhaustive, without an obvious signal that this has
  happened.
- No random generators or shrinkers are introduced, so this approach does
  not surface unanticipated edge cases the way property-based sampling can;
  it only verifies the cases the matrix's author enumerated.

## Removal Criteria

This decision should be revisited, and a property-testing or proof framework
adopted for the affected invariant, once either:

1. An invariant's input space grows large enough that exhaustive enumeration
   by hand becomes impractical or error-prone to maintain, or
2. `proptest`, Kani, or Verus becomes a workspace dependency for an
   unrelated reason, at which point re-using it here may cost less than
   maintaining a bespoke matrix.
