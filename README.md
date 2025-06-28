# Lille

A simple real-time strategy prototype demonstrating a DDlog-driven game loop
with Bevy rendering. The project currently implements "Phase 1" of the migration
roadmap, synchronizing the legacy `GameWorld` state into Bevy and rendering
static entities.

## Game Setting

*A fractured city of steel, brick, and neon — and your next battlefield.*

A city with too many masters and too little future. Once the jewel of a vanished
federation, its streets are now ruled by whoever can seize them — corporate
militias, rogue syndicates, data cults, and the last vestiges of an irrelevant
state.

The skyline tells the story: rusted iron arcades tower over flooded canals and
cracked boulevards. Century-old stone buildings are patched with ferroglass and
recycled scaffolds. Above them loom crumbling superstructures from a failed age
of expansion, their data relays and skyways flickering with black market
signals.

Beneath this decaying grandeur, commerce thrives in the margins. Street-level
markets sprawl through abandoned plazas; encrypted auction houses operate out of
gutted tram stations. Every corner offers a new opportunity — or an ambush.

Here, real power belongs to those who can move fastest, strike hardest, and
control the flow of information. Squads operate in the open, armed, and
augmented. Every mission risks crossing the unseen lines between factions — and
starting a war you cannot finish.

## Prepare to Deploy

Control is an illusion. Ownership is temporary. The streets belong to those who
can take them — and keep them.

## Installing DDlog

To install the DDlog toolchain required for development run:

```bash
./scripts/install_ddlog.sh
source ./.env
```

The `source` command loads the DDlog environment variables into the current
shell session.

The build script automatically loads this `.env` using
`dotenvy::dotenv_override` so that `cargo build` can locate the `ddlog` compiler
and standard library without additional setup.

The script downloads DDlog v1.2.3 into `~/.local/ddlog` and writes environment
variable assignments to `.env`. If that file already exists, it will be backed
up with a `.bak` suffix before being replaced. Any existing directory at
`~/.local/ddlog` will be removed before extraction.

## Build script

The `build.rs` entry point delegates to the `build_support` crate. This helper
crate generates constants from `constants.toml`, validates them against
`constants.schema.json`, downloads the project font, and compiles the DDlog
ruleset when the compiler is available.

The font download uses the operating system's certificate store for TLS
verification. Ensure your environment has a valid set of root certificates so
the HTTPS request succeeds.

## Isolated build support

Run the build support logic without compiling the whole game using the helper
script:

```bash
./scripts/build_support_runner.sh
```

The script compiles the helper binary and executes it directly so
`CARGO_MANIFEST_DIR` points to the repository root while running. It sets
`OUT_DIR` so the helper binary behaves like a build script.

This performs the same steps as `build.rs`, generating constants, downloading
the font, and compiling the DDlog ruleset when available. The helper does not
output "No such file or directory" errors when locating `constants.toml`, though
compilation may still fail if the DDlog compiler is missing.

The `build-support-run` Makefile target sets
`BUILD_SUPPORT_FAIL_ON_DDLOG_ERROR=true` by default so that any DDlog build
errors abort the run. Override the variable with an empty value to continue with
the fallback stub crate instead, e.g.:

```bash
BUILD_SUPPORT_FAIL_ON_DDLOG_ERROR= make build-support-run
```

## DDlog stubs

The `generated/lille_ddlog` directory is populated with placeholder code so the
project can be formatted and linted without the ddlog components. The
`ddlog-stubs` Makefile target copies these stubs from the `stubs/` directory
into `generated/`. When the `generated/lille_ddlog/lib.rs` target is built, the
stubs are replaced by the actual inferencer generated from the DDlog ruleset.

## Running tests with DDlog

Before executing any tests that use the `ddlog` feature, make sure the DDlog
tool-chain is installed (see *Installing DDlog* above).

The `-ddlog` targets, will generate the lille-ddlog crate prior to performing
the relevant action:

```bash
make test-ddlog
```

The above invokes `cargo test --features ddlog` with the compiled crate.
