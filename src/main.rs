use emulator::bus::Bus;
use emulator::cartridge::Rom;
use emulator::cpu::CPU;
use emulator::joypad::JoypadButton;
use ppu::PPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::env;

const WINDOW_SCALE: f32 = 3.0;

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
            (Frame::WIDTH as f32 * WINDOW_SCALE) as u32,
            (Frame::HEIGHT as f32 * WINDOW_SCALE) as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let key_map = create_keymap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(WINDOW_SCALE, WINDOW_SCALE).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .unwrap();

    let ppu = PPU::new(rom.chr_rom.clone(), rom.screen_mirroring);
    let mut frame = Frame::new();
    let mut bus = Bus::new_with_callback(ppu, Box::new(move |ppu, joypad| {
        render::render(ppu, &mut frame);
        texture
            .update(None, &frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),

                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                        joypad.set_pressed(*key);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                        joypad.set_released(*key);
                    }
                }
                _ => {}
            }
        }
    }));
    bus.rom = Some(Box::from(rom));

    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}

fn create_keymap() -> HashMap<Keycode, JoypadButton> {
    let mut key_map: HashMap<Keycode, JoypadButton> = HashMap::new();
    key_map.insert(Keycode::Down, JoypadButton::DOWN);
    key_map.insert(Keycode::Up, JoypadButton::UP);
    key_map.insert(Keycode::Right, JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, JoypadButton::LEFT);
    key_map.insert(Keycode::Space, JoypadButton::SELECT);
    key_map.insert(Keycode::Return, JoypadButton::START);
    key_map.insert(Keycode::A, JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, JoypadButton::BUTTON_B);
    key_map
}
