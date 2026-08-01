# Architectural decision record (ADR) 003: bounded case matrices over a property-testing framework

## Status

Accepted, 2026-07-26. Amended 2026-08-01: `proptest` is now a
development-only workspace dependency and supplements the bounded matrices
for the broader domains. The matrices remain, and the rationale below for
choosing them still stands where the domain is finite and small.

## Date

2026-07-26.

## Context and problem statement

Several invariants in the DBSP-sync and map-lifecycle code must hold
*exactly*, not merely "usually", because they govern correctness properties
such as at-most-one decision per entity, exact frame rollback, and rejection
of unsafe asset paths. Two distinct classes of tool are the conventional
choice for this class of invariant: property-based *sampling* (`proptest`),
which generates many randomly drawn inputs per run, and formal *proof* tools
(Kani, Verus), which prove a property over a bounded or whole input domain,
rather than relying on a handful of hand-picked cases.

When this ADR was first accepted, none of `proptest`, Kani, or Verus was a
workspace dependency, so adopting one was a new supply-chain decision rather
than reuse of existing tooling. `proptest` has since been added as a
development-only workspace dependency, for the reasons recorded in the
amendment below; Kani and Verus remain unadopted.

## Decision drivers

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
- Adding a dependency is a supply-chain cost (new dependency, new tool to
  install and maintain) that this testing strategy does not need to pay
  where exhaustive enumeration is achievable by hand. `proptest` is a
  library rather than a separate toolchain, so that cost is far lower for it
  than for Kani or Verus.

## Options considered

### Option A: adopt `proptest`

Generate randomly-sampled inputs (with shrinking on failure) for the
invariants under test.

Rejected *as a replacement* for the matrices, and later adopted *alongside*
them. `proptest` samples a subset of a domain rather than enumerating it
exhaustively, so for the finite state/ordering dimensions a handwritten
matrix still covers every case where sampling would only cover some. For the
weight and path domains, where a matrix is no more exhaustive than sampling,
the original judgement — that the dependency and generator/shrinker authoring
cost outweighed the benefit — was reversed; see the amendment below.

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

### Option C (chosen): bounded case matrices, since supplemented by `proptest`

Enumerate the finite state/ordering dimensions exhaustively, and enumerate
representative equivalence classes for the weight and path domains, as
explicit `rstest` cases, using the `rstest` stack already in use throughout
the codebase.

## Decision outcome

For the DBSP-sync and map-lifecycle invariants listed below, cover the
invariant with a bounded case matrix. As originally accepted this was
*instead of* a property-testing or proof framework; as amended it is the
first of two layers, with sampled `proptest` properties supplementing the
matrices over the broader domains (see the amendment below):

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
and substring-`..` path forms), not the entire domain. Taken alone that is a
deliberately narrower guarantee than sampling or a proof tool would give
over those two domains, which is precisely the gap the 2026-08-01 amendment
closes with `proptest`. The matrices remain the exhaustive layer wherever
the underlying domain is finite and small enough to enumerate; a proof tool
(Kani, Verus) is still not used for any of these invariants.

## Amendment, 2026-08-01: `proptest` supplements the matrices

The coverage gap this ADR recorded as "a genuine, present coverage gap, not
a future risk" was acted on: `proptest` is now a development-only workspace
dependency, and sampled properties cover the broader domains alongside the
matrices. The two layers are distinct and both are kept:

- **Exhaustive finite matrices** — the `rstest` cases enumerate small,
  genuinely finite domains completely. Sampling cannot improve on them
  there, and they remain the canonical statement of those invariants.
- **Sampled broader-domain properties** — `proptest` covers the domains a
  matrix can only sample from: arbitrary asset-path strings, weighted
  decision sets over the full `i64` weight range, and generated sequences of
  frame-rollback lifecycle actions.

The properties live beside the matrices they supplement, in a `properties`
submodule of each test module:

- `properties::to_decision_matches_the_overflow_safe_weight_oracle` and
  `properties::dedupe_emits_at_most_one_decision_of_multiplicity_one` in
  `src/dbsp_circuit/streams/behaviour/decide/tests.rs`.
- `properties::validate_asset_path_agrees_with_the_documented_contract` in
  `src/map/lifecycle/tests.rs`.
- `properties::rollback_restores_the_frame_start_tracking` in
  `src/dbsp_sync/state/tests.rs`.

Two things surfaced from generating extreme values, both now fixed in
production code rather than hidden by narrowing a generator:
`MovementAccumulator` accumulates its weight total with `saturating_add`,
since a plain `+=` panicked on an overflowing total; and the aggregation
check tests `total_weight` against `-1..=1` rather than calling `abs()`,
which overflows on `i64::MIN`. The weight oracle folds in `i128` and clamps
per step, so it never overflows and matches the saturating accumulator
exactly.

One bound is *not* this crate's to fix: DBSP's own `ZWeight` arithmetic
panics on extreme weights before any code here runs, so the circuit-level
property draws from a bounded weight range. The accumulator's handling of
the `i64` extremes is covered by the pure property, which does not go
through DBSP.

Kani and Verus remain unadopted; the reasoning in Option B is unchanged.

## Known risks and limitations

- The sampled properties reduce, but do not close, the coverage gap over the
  weight and path domains: sampling explores far more of those domains than
  the matrices do, yet still cannot enumerate them. A bug confined to a value
  class the generators never draw would remain uncaught.
- For the frame-rollback ordering invariant, the exhaustiveness argument is
  sound only while the state/ordering space stays small enough to enumerate
  by hand. If the number of independent dimensions grows, a handwritten
  matrix risks silently becoming a sample rather than remaining exhaustive,
  without an obvious signal that this has happened.
- The matrices themselves still only verify the cases their author
  enumerated; it is the accompanying properties, not the matrices, that
  surface unanticipated edge cases.
- The properties are sampled, so a run is not a proof and a passing suite is
  not evidence that no counter-example exists. Their generators encode an
  author's judgement about the interesting shapes of the domain, which is a
  weaker but similar assumption to the matrices'.

## Removal criteria

Criteria 1 and 3 have been met for `proptest`, which is why the amendment
above adopted it. The decision should be revisited again, and a *proof* tool
(Kani or Verus) adopted for the affected invariant, once any of:

1. A sampled property proves insufficient — a bug report, an incident, or a
   review finding showing that a counter-example exists which sampling is
   unlikely to draw, indicating that a bounded or whole-domain proof is
   warranted.
2. The frame-rollback state/ordering invariant's input space grows large
   enough that exhaustive enumeration by hand becomes impractical or
   error-prone to maintain, or
3. Kani or Verus becomes a workspace dependency for an unrelated reason, at
   which point re-using it here may cost less than maintaining the current
   matrix-plus-property pairing.
