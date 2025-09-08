# Test utilities

The `test_utils` crate provides constructors and assertions for tests. To avoid
repetitive import lists, it exposes a `prelude` module that bundles the most
common helpers.

```rust
use test_utils::prelude::*;

#[test]
fn example() {
    assert_all_present("block", &["block"]);
}
```

The prelude includes:

- Assertions: `assert_all_present`, `assert_all_absent`,
  `assert_valid_rust_syntax`
- Constructors: `block`, `force`, `force_with_mass`, `new_circuit`, `pos`,
  `slope`, `target`, `vel`
- Physics types: `BlockCoords`, `BlockId`, `Coords2D`, `Coords3D`, `EntityId`,
  `FearValue`, `ForceVector`, `Gradient`, `Mass`

Default to `use test_utils::prelude::*;` in tests unless a module only needs a
small subset of helpers.
