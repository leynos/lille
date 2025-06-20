//! Example game application using the Lille library.
//! Launches a Bevy app and wires up logging, DDlog, and basic systems.
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use color_eyre::eyre::Result;
use lille::{
    apply_ddlog_deltas_system, init_ddlog_system, init_logging, push_state_to_ddlog_system,
    spawn_world_system,
};

/// A realtime strategy game
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Entry point for the realtime strategy game application.
///
/// Parses command-line arguments, configures logging, and launches the Bevy app with custom system scheduling for DDlog integration and world setup.
fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    init_logging(args.verbose);

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_systems(Startup, init_ddlog_system)
        .add_systems(Startup, spawn_world_system.after(init_ddlog_system))
        .add_systems(
            Startup,
            push_state_to_ddlog_system.after(spawn_world_system),
        )
        .add_systems(
            Update,
            (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
        )
        .run();
    Ok(())
}
