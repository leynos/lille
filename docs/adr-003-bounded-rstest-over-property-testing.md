# Architectural decision record (ADR) 003: bounded case matrices over a property-testing framework

## Status

Accepted, 2026-07-26.

## Date

2026-07-26.

## Context and Problem Statement

Several invariants in the DBSP-sync and map-lifecycle code must hold
*exactly*, not merely "usually", because they govern correctness properties
such as at-most-one decision per entity, exact frame rollback, and rejection
of unsafe asset paths. Two distinct classes of tool are the conventional
choice for this class of invariant: property-based *sampling* (`proptest`),
which generates many randomly drawn inputs per run, and formal *proof* tools
(Kani, Verus), which prove a property over a bounded or whole input domain,
rather than relying on a handful of hand-picked cases.

None of `proptest`, Kani, or Verus is a workspace dependency (checked against
the workspace `Cargo.toml`). Adopting one to cover these invariants would be
a new supply-chain decision, not a reuse of existing tooling.

## Decision Drivers

- The rollback/commit orderings and prior-entry/repeat-undo/commit-vs-
  rollback state combinations have a small, **finite** input space that a
  handwritten matrix can enumerate exhaustively.
- The movement-decision weights (`i64`-valued) have a **large but finite**
  domain, and asset-path strings have a genuinely **unbounded** domain (no
  fixed maximum length); a handwritten matrix can only cover a handful of
  representative equivalence classes (positive, negative, and net-zero
  weights; rooted, `..`-component, and substring-`..` path forms), not the
  entire domain.
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

Rejected for these invariants: `proptest` samples a subset of a domain
rather than enumerating it exhaustively. For the finite state/ordering
dimensions, a handwritten matrix can cover every case rather than a
sample; for the weight and path domains, a handwritten matrix is no more
exhaustive than sampling, but avoids the dependency and generator/shrinker
authoring cost. `proptest` is also not currently a workspace dependency.

### Option B: adopt Kani or Verus

Use bounded model checking (Kani), which proves the invariant holds within
the harness's configured bounds (loop unwind limits, size bounds) rather
than over the whole input domain, or a proof assistant (Verus), which
deductively proves the invariant holds over its entire input domain.

Rejected for now: neither is a workspace dependency, and both carry a
significantly heavier toolchain and authoring cost than a test matrix. For
the finite state/ordering dimensions, a proof tool is more machinery than
the problem requires. For the weight and path domains, Kani's bounded
guarantee and Verus's whole-domain proof would each give stronger assurance
than the enumerated matrix, but that investment is not judged warranted at
present.

### Option C (chosen): bounded case matrices

Enumerate the finite state/ordering dimensions exhaustively, and enumerate
representative equivalence classes for the weight and path domains, as
explicit `rstest` cases, using the `rstest` stack already in use throughout
the codebase.

## Decision Outcome

For the DBSP-sync and map-lifecycle invariants listed below, cover the
invariant with a bounded case matrix rather than adopting a
property-testing or proof framework:

- **Movement-decision dedupe** — one decision per entity, none for net-zero
  total weight, and a consolidated Z-set multiplicity of exactly `1` for the
  emitted decision. Canonical example:
  `dedupe_emits_one_decision_for_positive_weight_and_none_for_zero` in
  `src/dbsp_circuit/streams/behaviour/decide/tests.rs`.
- **Frame rollback** — exact pre-frame restoration of `health_snapshot`,
  `pending_damage_retractions`, and `applied_unsequenced` on a failed
  `step_circuit()` call, versus retention of the frame's changes on commit.
  Canonical example: `applied_unsequenced_rollback_matrix` in
  `src/dbsp_sync/state/tests.rs`.
- **Asset-path validation** — rejection of rooted paths and standalone `..`
  traversal components, versus acceptance of relative paths where `..`
  appears only as a substring. Canonical example:
  `validate_asset_path_component_matrix` in `src/map/lifecycle/tests.rs`.

For the frame-rollback ordering dimensions, the matrix enumerates every
case in a genuinely finite, small domain, so it is exhaustive there. For
the movement-decision weights (`i64`-valued), the domain is large but
finite; for asset-path strings, the domain is genuinely unbounded. In both
cases the matrix instead enumerates chosen representative equivalence
classes (positive, negative, and net-zero weights; rooted, `..`-component,
and substring-`..` path forms), not the entire domain. This is a
deliberately narrower guarantee than a property-testing framework
(sampling) or a proof tool (Kani, Verus) would give over those two domains.
This ADR is the documented, approved exception to defaulting to a
property-testing or proof framework for exactly-correctness invariants: it
applies only to the invariants named above and any future invariant of an
equivalent shape, on the understanding that coverage is exhaustive only
where the underlying domain is finite and small enough to enumerate.

## Known Risks and Limitations

- For the movement-decision weight and asset-path invariants, the matrix is
  already not exhaustive: it covers hand-chosen equivalence classes over a
  large-but-finite (weights) or unbounded (paths) domain, and a bug
  specific to a value outside those classes would not be caught. This is a
  genuine, present coverage gap, not a future risk.
- For the frame-rollback ordering invariant, the exhaustiveness argument is
  sound only while the state/ordering space stays small enough to enumerate
  by hand. If the number of independent dimensions grows, a handwritten
  matrix risks silently becoming a sample rather than remaining exhaustive,
  without an obvious signal that this has happened.
- No random generators or shrinkers are introduced, so this approach does
  not surface unanticipated edge cases the way property-based sampling can;
  it only verifies the cases the matrix's author enumerated.

## Removal Criteria

This decision should be revisited, and a property-testing or proof framework
adopted for the affected invariant, once any of:

1. The movement-decision weight or asset-path invariants show evidence — a
   bug report, an incident, or a review finding — that the enumerated
   equivalence classes miss a value class that matters, indicating stronger
   assurance over those domains is warranted.
2. The frame-rollback state/ordering invariant's input space grows large
   enough that exhaustive enumeration by hand becomes impractical or
   error-prone to maintain, or
3. `proptest`, Kani, or Verus becomes a workspace dependency for an
   unrelated reason, at which point re-using it here may cost less than
   maintaining a bespoke matrix.
