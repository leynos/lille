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

The build script also loads this `.env` automatically using the `dotenvy` crate
so that `cargo build` can locate the `ddlog` compiler and standard library
without additional setup.

The script downloads DDlog v1.2.3 into `~/.local/ddlog` and writes environment
variable assignments to `.env`. If that file already exists, it will be backed up
with a `.bak` suffix before being replaced. Any existing directory at
`~/.local/ddlog` will be removed before extraction.
