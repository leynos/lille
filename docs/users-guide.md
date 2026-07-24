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

If the configured `primary_map` path fails validation, no primary map
spawns. Instead, the plugin triggers a
`LilleMapError::InvalidPrimaryMapAssetPath` event carrying the offending
path. An observer logs this event via `error!`; it does not panic, so
runtime and test runs fail loudly but safely.

For the design rationale behind these rules, see §5.5 of [Integrating
isometric Tiled maps into Lille](
lille-isometric-tiled-maps-design.md#55-primary-map-asset-path-validation).
