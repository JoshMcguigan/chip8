extern crate libchip8;
extern crate sdl2;

use std::error::Error;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use libchip8::Chip8;

fn main() {
    println!("Chip8 emulator in Rust");

    // TODO Setup the render system
    // setupGraphics();
    // setupInput();
    let path = Path::new("PONG");
    let display = path.display();

    // Initialize the emulator and load the game
    let mut chip = Chip8::new();
    let mut file = match File::open(path) {
        Err(why) => panic!("Couldn't open {}: {}", display, Error::description(&why)),
        Ok(file) => file,
    };

    let mut game = Vec::new();
    match file.read_to_end(&mut game) {
        Err(why) => panic!("Couldn't read {}: {}", display, Error::description(&why)),
        Ok(_) => (),
    };

    //let test = vec![0x62, 0x00, 0x61, 0x0B, 0xF1, 0x29, 0xD2, 0x05];
    //chip.loadHex(&test);
    chip.loadHex(&game);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Chip8 Emulator", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();
    //renderer.set_draw_color(Color::RGB(255, 0, 0));
    //renderer.clear();
    //renderer.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut texture = renderer.create_texture_streaming(
        PixelFormatEnum::RGB24, 256, 256).unwrap();

    // Emulation loop
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        chip.emulateCycle();

        if chip.drawFlag {
            // TODO
            // drawGraphics();
            print!("{:?}", chip);
            chip.drawFlag = false;
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..32 {
                    for x in 0..64 {
                        let offset = y*pitch + x*3;
                        let value = if 0 != chip.graphics[y * 64 + x] { 255 } else { 0 };
                        buffer[offset + 0] = value as u8;
                        buffer[offset + 1] = value as u8;
                        buffer[offset + 2] = 0;
                    }
                }

            }).unwrap();
            renderer.clear();
            renderer.copy(&texture, None, Some(Rect::new(100, 100, 256, 256)));
            //renderer.copy_ex(&texture, None,
            //              Some(Rect::new(450, 100, 256, 256)), 30.0, None, false, false).unwrap();
            renderer.present();
        }

        // Store key press state
        //chip.setKeys();
    }
}
