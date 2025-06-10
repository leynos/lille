# lille

A simple real-time strategy prototype demonstrating a DDlog-driven
game loop with Bevy rendering. The project currently implements
"Phase 1" of the migration roadmap, synchronising the legacy
`GameWorld` state into Bevy and rendering static entities.

## Installing DDlog

To install the DDlog toolchain required for development run:

```bash
./scripts/install_ddlog.sh
source ./.env
```

The `source` command loads the DDlog environment variables into the
current shell session.

The script downloads DDlog v1.2.3 into `~/.local/ddlog` and writes
environment variable assignments to `.env`. If that file
already exists it will be backed up with a `.bak` suffix before
being replaced. Any existing directory at `~/.local/ddlog` will be
removed prior to extraction.
