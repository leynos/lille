# Documentation index

This directory gathers design notes and testing references. The following pages
provide further detail:

- [Architecture](architecture.md)
- [ADR-003: bounded `rstest` matrices over a property-testing framework](
  adr-003-bounded-rstest-over-property-testing.md)
- [Developer's guide](developers-guide.md)
- [Bevy headless testing](bevy-headless-testing.md)
- [Bevy 0.16+ migration plan (archived)](bevy-0-16-plus-migration-plan.md)
- [Behavioural testing in Rust with RSpec](behavioural-testing-in-rust-with-rspec.md)
- [Complexity antipatterns and refactoring strategies](
  complexity-antipatterns-and-refactoring-strategies.md)
- [DBSP synchronization developer's guide](dbsp-synchronization-guide.md)
- [Declarative world inference with DBSP and Rust](
  declarative-world-inference-with-dbsp-and-rust.md)
- [Documentation style guide](documentation-style-guide.md)
- [Lille physics and world engine roadmap](
  lille-physics-and-world-engine-roadmap.md)
- [Lille physics engine design](lille-physics-engine-design.md)
- [Map data format](map-data-format.md)
- [Rust testing with rstest fixtures](rust-testing-with-rstest-fixtures.md)
- [Testing declarative game logic in DBSP](testing-declarative-game-logic-in-dbsp.md)
- [Test utilities](test_utils.md)
- [User's guide](users-guide.md)

## Spelling policy

Run `make spelling` to enforce en-GB-oxendict prose spelling. The gate
regenerates `typos.toml` from the live shared dictionary and the
`typos.local.toml` overlay on every run, so `typos.toml` is never drift checked
in CI. Put narrow repository-specific exceptions in `typos.local.toml`; never
edit generated entries by hand.
