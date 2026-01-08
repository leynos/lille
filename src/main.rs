#![cfg(feature = "render")]
//! Example game application using the Lille library.
//! Launches a Bevy app and wires up logging, world state, and basic systems.
use anyhow::Result;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "map")]
use lille::LilleMapPlugin;
use lille::{init_logging, DbspPlugin, PresentationPlugin};

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
/// Parses command-line arguments, configures logging, and launches the Bevy app
/// with custom system scheduling for world state integration and world setup.
///
/// # Errors
/// Propagates failures from logger initialisation or Bevy app execution.
fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(args.verbose)?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
    app.add_plugins(DbspPlugin);
    app.add_plugins(PresentationPlugin);

    #[cfg(feature = "map")]
    {
        app.add_plugins(LilleMapPlugin);
    }

    app.run();
    Ok(())
}
