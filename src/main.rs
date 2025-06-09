use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use lille::{
    init_ddlog_system, init_logging, init_world_system, push_state_to_ddlog_system,
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

fn main() {
    let args = Args::parse();
    init_logging(args.verbose);

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_systems(Startup, init_ddlog_system)
        .add_systems(Startup, init_world_system.after(init_ddlog_system))
        .add_systems(Startup, spawn_world_system.after(init_world_system))
        .add_systems(
            Startup,
            push_state_to_ddlog_system.after(spawn_world_system),
        )
        .run();
}
