# Documentation index

This directory gathers design notes and testing references. The following pages
provide further detail:

- [Architecture](architecture.md)
- [Developer's guide](developers-guide.md)
- [Bevy headless testing](bevy-headless-testing.md)
- [Bevy 0.16+ migration plan (archived)](bevy-0-16-plus-migration-plan.md)
- [Behavioural testing in Rust with RSpec](behavioural-testing-in-rust-with-rspec.md)
- [Complexity antipatterns and refactoring strategies](
  complexity-antipatterns-and-refactoring-strategies.md)
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

## Spelling policy

Run `make spelling` to enforce en-GB-oxendict prose spelling. The generated
`typos.toml` starts from the shared estate dictionary, refreshes its untracked
local cache only when the authority is newer, and then applies the narrow
repository policy in `typos.local.toml`. Edit the local policy and regenerate
the configuration rather than changing generated entries by hand.
