# Lille user's guide

This guide documents user-facing configuration and behaviour for people
running or configuring Lille. It complements the design and developer guides
in this directory, which describe implementation details for contributors.

## 1. Primary map asset path validation

`LilleMapPlugin` spawns a single "primary" Tiled map at startup. The map to
load is configured via the `primary_map` field of the `LilleMapSettings`
resource, which defaults to `maps/primary-isometric.tmx`. Before spawning,
the plugin validates this path and rejects any value that could resolve
outside the asset root.

### Path rules

- The path must be **relative** to the Bevy asset root; it is passed
  directly to the asset server's loader.
- An **empty path** is rejected.
- A **rooted path**, in any platform form, is rejected:
  - Unix-absolute paths, for example `/etc/maps/x.tmx`.
  - Windows backslash-root and UNC paths, for example `\maps\x.tmx` or
    `\\server\share\x.tmx`.
  - Windows drive-absolute paths, for example `C:\maps\x.tmx` or
    `C:/maps/x.tmx`.
- A path containing `..` as a **whole path component** is rejected, whether
  the component is delimited by `/` or `\`. For example, `maps/../secrets.tmx`
  and `maps\..\secrets.tmx` are both rejected.
- A path where `..` appears only as a **substring** of a component, rather
  than as a standalone component, is accepted. For example,
  `maps/primary..backup.tmx` is a valid filename, not a traversal attempt.

### Examples

| Path | Outcome | Reason |
| --- | --- | --- |
| `maps/primary-isometric.tmx` | Accepted | Ordinary relative path. |
| `maps/primary..backup.tmx` | Accepted | `..` is a substring, not a component. |
| `/etc/maps/x.tmx` | Rejected | Unix-absolute (rooted) path. |
| `C:\maps\x.tmx` | Rejected | Windows drive-absolute path. |
| `\\server\share\x.tmx` | Rejected | Windows UNC path. |
| `maps/../secrets.tmx` | Rejected | `..` is a whole path component. |

_Table 1: Accepted and rejected forms of `LilleMapSettings::primary_map`._

### What happens on rejection

If the configured `primary_map` path fails validation, the plugin
triggers and logs a `LilleMapError::InvalidPrimaryMapAssetPath` event
(carrying the offending path) via `error!`, then skips spawning the
primary map. The plugin does not panic and keeps running; only a test
assertion that expects the rejection would fail if the event were not
triggered.

For the design rationale behind these rules, see §5.5 of [Integrating
isometric Tiled maps into Lille](lille-isometric-tiled-maps-design.md#55-primary-map-asset-path-validation).

## 2. DBSP circuit contracts

These contracts matter to anyone composing Lille's DBSP stream helpers
directly, or reading the state the sync systems maintain.

### `apply_movement` expects deduplicated decisions

`apply_movement` applies at most one movement decision per entity per tick.
It does **not** deduplicate its input: `movement_decision_stream` already
folds duplicate decisions for an entity into a single normalized vector, and
`apply_movement` consumes that result. Feeding it a stream that still
contains two decisions for one entity is a contract violation — release
builds log a warning and debug builds panic on the `debug_assert!`.

When wiring the helpers by hand, put `movement_decision_stream` (or an
equivalent per-entity fold) ahead of `apply_movement`, as
`DbspCircuit::new` does.

### Only positive-weight output records are applied

The output systems iterate the _consolidated_ Z-set for positions,
velocities, and health deltas, and apply only records whose weight is
positive. Retractions (negative weight) are skipped and never mutate ECS
components or the world handle. Consolidation removes net-zero records
before the loops see them, so in practice the guard's effect is to ignore
retractions.

Downstream code should therefore not rely on observing a retraction as an
ECS mutation; a retracted record simply leaves the previous component value
in place.

### Reliability counters

`DbspState` exposes three bounded counters, with no per-entity labels, for
sampling circuit health:

- `applied_health_duplicates()` — duplicate health or damage events filtered.
- `step_failures()` — circuit steps that failed and were rolled back.
- `skipped_outputs()` — output records skipped for a non-positive weight.

A failed step also logs a warning naming the operation and error, and rolls
the frame's Rust-side tracking back, so the next frame starts in a consistent
state.

### Movement-aggregation diagnostics

`movement_decision_stream` folds an entity's decisions into a single
normalized vector, but that fold is a pure map and cannot log from inside
the circuit. `movement_decision_streams` exposes the same fold alongside a
diagnostic stream, so the caller can report what happened:

```rust
let (decisions, aggregations) =
    movement_decision_streams(fear, targets, positions);
```

The returned `decisions` stream is identical to the one
`movement_decision_stream` alone would produce; the second stream adds
information without changing movement behaviour. Each `MovementAggregation`
record is `{ entity: i64, total_weight: i64 }`. A record is emitted only
when deduplication actually collapsed something — an entity with a single
decision emits no record, and an entity whose weights net to zero emits
neither a decision nor an aggregation record.

`DbspCircuit::movement_aggregation_out()` exposes the corresponding
`OutputHandle<OrdZSet<MovementAggregation>>`. Aggregation records carry
Z-set weights like every other circuit output, so consumers must apply the
same rule as [Only positive-weight output records are
applied](#only-positive-weight-output-records-are-applied): consolidate the
handle, skip any record whose weight is `<= 0`, then act on the rest.
Lille's own `apply_dbsp_outputs_system` does this, emitting one `warn!` per
positive-weight record naming the entity and total weight. A consumer
driving its own circuit must likewise drain the handle every frame — with
`take_from_all()` or equivalent — or records accumulate.

For the full frame lifecycle and rollback API, see the [DBSP
synchronization developer's guide](dbsp-synchronization-guide.md).
