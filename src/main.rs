extern crate chip8;
extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate sdl2;

mod audio;
mod graphics;
mod input;
mod loader;

use chip8::Chip8;
use clap::{Arg, App};
use std::time::Duration;

/// Adjust the scaling factor of the chip8's display.  The larger the number,
/// the bigger the display.
const SCALE : u32 = 8;

/// The scaled width of the display.
const WIDTH : u32 = chip8::WIDTH * SCALE;

/// The scaled height of the display.
const HEIGHT : u32 = chip8::HEIGHT * SCALE;

fn main() {
    // Init the logger
    env_logger::init().unwrap();

    // Setup the commandline flags and usage/help message.
    let matches = App::new("Chip8 Emulator")
        .version(chip8::version())
        .author("Chris Konstad <chriskon149@gmail.com>")
        .about("Runs Chip8 games.")
        .arg(Arg::with_name("ROM")
             .help("Sets the path to the ROM to play")
             .required(true))
        .get_matches();

    println!("Chip8 emulator starting...");

    // Initialize the emulator and load the game
    let mut chip = Chip8::new();
    chip.load_hex(&loader::load_file(matches.value_of("ROM").unwrap()));

    // Prepare SDL for video, audio, and input
    let sdl_context = sdl2::init().unwrap();
    let mut beeper = audio::Beeper::new(&sdl_context,
                                        Duration::from_millis(250));
    let mut keyboard = input::Keyboard::new(&sdl_context);

    // TODO TEST AT 60frames a second!
    // This isn't set to 60Hz because that was too slow
    let mut window = graphics::Display::new(&sdl_context,
                                            "Chip8 Emulator",
                                            WIDTH,
                                            HEIGHT,
                                            Duration::from_millis(2));

    // Emulation loop
    'running: loop {
        // Check the input and store it on the chip
        match keyboard.check(&mut chip.key) {
            input::Command::Quit => break 'running,
            input::Command::Continue => {}
        }

        // Run a cycle on the chip
        chip.emulate_cycle();

        // Render the frame if needed
        if chip.draw_flag {
            debug!("{:?}", chip);
            chip.draw_flag = false;
            window.draw_frame(&chip.graphics);
        }

        // Make sound if needed
        beeper.set_beep(chip.make_sound);
    }
}
