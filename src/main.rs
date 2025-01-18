use clap::Parser;
use lille::graphics::GraphicsContext;
use lille::world::GameWorld;
use lille::init_logging;
use piston_window::*;
use std::error::Error;

/// A realtime strategy game
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    init_logging(args.verbose);

    let mut world = GameWorld::new();
    let mut graphics = GraphicsContext::new()?;

    while let Some(e) = graphics.next() {
        if let Some(_) = e.render_args() {
            graphics.render(&e, &world);
        }

        // Update game state
        world.update();
    }

    Ok(())
}
