# Entity → WorldEntity Rename

Status: active

Lille `0.1.0` renamed the `lille::Entity` data type to `lille::WorldEntity` to
avoid clashes with Bevy’s `Entity`. A deprecated type alias is provided for the
`0.1.x` series so existing code continues to compile with warnings.

## Migration Guidance

- Update imports to pull `WorldEntity` instead of `Entity`.
- Replace struct fields, parameters, and type annotations that referenced
  `Entity` with `WorldEntity`.
- When interacting with Bevy APIs continue to use `bevy::prelude::Entity`; the
  rename only affects Lille’s simplified world representation.

```rust
use lille::WorldEntity;

fn track(entity: WorldEntity) {
    // ...
}
```

The `Entity` alias will be removed in the next release. Migrating now prevents
future breakage and makes Bevy and Lille entities easier to distinguish during
refactors and debugging.
