use emulator::bus::Bus;
use emulator::cartridge::Rom;
use emulator::cpu::CPU;
use emulator::trace::trace;
use ppu::PPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let program = std::fs::read(filename).unwrap();
    let rom = Rom::new(&program).unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window(
            "NES Emulator in Rust by acr92",
            (256 * 3) as u32,
            (240 * 3) as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let ppu = PPU::new(rom.chr_rom.clone(), rom.screen_mirroring);
    let mut frame = Frame::new();
    let mut bus = Bus::new_with_callback(ppu, move |ppu| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),

                _ => {}
            }
        }
    });
    bus.rom = Some(Box::from(rom));

    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}
