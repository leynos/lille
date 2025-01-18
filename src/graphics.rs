use piston_window::*;
use std::error::Error;
use crate::world::GameWorld;

pub const WINDOW_SIZE: u32 = 1000;
pub const PIXEL_SIZE: f64 = 3.0;

pub struct GraphicsContext {
    window: PistonWindow,
    glyphs: Glyphs,
}

impl GraphicsContext {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut window: PistonWindow = WindowSettings::new("Lille", [WINDOW_SIZE, WINDOW_SIZE])
            .exit_on_esc(true)
            .build()?;

        // Get font path from build script
        let font_path = env!("FONT_PATH");
        println!("Loading font from: {}", font_path);
        
        // Load font using window's built-in method
        let glyphs = window.load_font(font_path)?;

        Ok(Self { window, glyphs })
    }

    pub fn next(&mut self) -> Option<Event> {
        self.window.next()
    }

    pub fn render(&mut self, event: &Event, world: &GameWorld) {
        self.window.draw_2d(event, |c, g, d| {
            // Clear the screen with a dark gray background
            clear([0.2, 0.2, 0.2, 1.0], g);

            // Draw all entities and actors
            for (&pos, &count) in world.get_all_positions().iter() {
                // Make bad guys appear red
                let color = if count >= 5 {
                    [1.0, 0.0, 0.0, 1.0]  // Red for bad guys
                } else {
                    let value = (count as f32 * 0.3).min(1.0);
                    [value, value, value, 1.0]  // White for others
                };

                rectangle(
                    color,
                    [
                        pos.0 as f64 * PIXEL_SIZE,
                        pos.1 as f64 * PIXEL_SIZE,
                        PIXEL_SIZE,
                        PIXEL_SIZE,
                    ],
                    c.transform,
                    g,
                );
            }

            // Draw tick counter with error handling
            if let Err(e) = text::Text::new_color([1.0, 1.0, 1.0, 1.0], 32)
                .draw(
                    &format!("Tick: {}", world.tick_count),
                    &mut self.glyphs,
                    &c.draw_state,
                    c.transform.trans(10.0, 30.0),
                    g,
                )
            {
                eprintln!("Failed to render text: {}", e);
            }

            // Ensure glyphs are updated
            self.glyphs.factory.encoder.flush(d);
        });
    }
}