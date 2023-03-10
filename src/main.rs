use crate::bus::Bus;
use crate::cartridge::Rom;
use crate::cpu::CPU;
use crate::trace::trace;
use core::mem::Mem;
use ppu::PPU;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::EventPump;
use std::env;

mod bus;
mod cartridge;
mod cpu;
mod opcodes;
mod register;
mod trace;

fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => {
                cpu.mem_write(0xff, 0x77);
            }
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.mem_write(0xff, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.mem_write(0xff, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.mem_write(0xff, 0x64);
            }
            _ => { /* Do nothing */ }
        }
    }
}

fn color(byte: u8) -> Color {
    match byte {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color::GREY,
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color::MAGENTA,
        7 | 14 => Color::YELLOW,
        _ => Color::CYAN,
    }
}

fn read_screen_state(cpu: &mut CPU, frame: &mut [u8; 32 * 32 * 3]) -> bool {
    let mut frame_index = 0;
    let mut update = false;
    for i in 0x0200..0x0600 {
        let color_index = cpu.mem_read(i as u16);
        let (b1, b2, b3) = color(color_index).rgb();
        if frame[frame_index] != b1 || frame[frame_index + 1] != b2 || frame[frame_index + 2] != b3
        {
            frame[frame_index] = b1;
            frame[frame_index + 1] = b2;
            frame[frame_index + 2] = b3;
            update = true;
        }
        frame_index += 3;
    }

    update
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let program = std::fs::read(filename).unwrap();
    let rom = Rom::new(&program).unwrap();

    let ppu = PPU::new_empty_rom();
    let mut bus = Bus::new(ppu);
    bus.rom = Some(Box::from(rom));
    let mut cpu = CPU::new(bus);
    cpu.reset();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Snake game", (32.0 * 10.0) as u32, (32.0 * 10.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 32, 32)
        .unwrap();

    let mut screen_state = [0 as u8; 32 * 32 * 3];
    let mut rng = rand::thread_rng();

    // TODO: Only use this when running nestest
    cpu.bus.cycles = 7;
    cpu.bus.ppu.cycles = 21;
    cpu.bus.ppu.scanline = 0;
    cpu.register.pc = 0xC000;

    cpu.run_with_callback(move |cpu| {
        println!("{}", trace(cpu));

        handle_user_input(cpu, &mut event_pump);
        cpu.mem_write(0xFE, rng.gen_range(1..16));

        if read_screen_state(cpu, &mut screen_state) {
            texture.update(None, &screen_state, 32 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }

        std::thread::sleep(std::time::Duration::new(0, 10_000));
    });
}
