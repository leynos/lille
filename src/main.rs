//! Example game application using the Lille library.
//! Launches a Bevy app and wires up logging, world state, and basic systems.
use anyhow::Result;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use lille::{init_logging, spawn_world_system, DbspPlugin};

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
/// Parses command-line arguments, configures logging, and launches the Bevy app with custom system scheduling for world state integration and world setup.
fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(args.verbose);

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(DbspPlugin)
        .add_systems(Startup, spawn_world_system)
        .run();
    Ok(())
}
