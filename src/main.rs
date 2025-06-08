use bevy::prelude::*;
use clap::Parser;
use lille::{init_ddlog_system, init_logging};

/// A realtime strategy game
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn hello_world() {
    info!("Hello Bevy!");
}

fn main() {
    let args = Args::parse();
    init_logging(args.verbose);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(init_ddlog_system)
        .add_systems(Startup, hello_world)
        .run();
}
