# lille

A simple real-time strategy prototype demonstrating a DDlog-driven
game loop with Bevy rendering. The project currently implements
"Phase 1" of the migration roadmap, synchronising the legacy
`GameWorld` state into Bevy and rendering static entities.

## Installing DDlog

To install the DDlog toolchain required for development run:

```bash
./scripts/install_ddlog.sh
source ~/.ddlog_env
```

This downloads DDlog v1.2.3 into `~/.local/ddlog` and updates
`PATH` and `DDLOG_HOME` via `~/.ddlog_env`.
