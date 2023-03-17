mod input;

use crate::input::{create_keymap, InputAction, InputButton, InputEvent};
use cpu6502::cpu::CPU;
use emulator::bus::NESBus;
use emulator::cartridge::Rom;
use emulator::joypad::Joypad;
use ppu::oam::Oam;
use ppu::PPU;
use render::frame::Frame;
use render::rectangle::Rectangle;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::TextureQuery;
use sdl2::surface::Surface;
use sdl2::EventPump;
use std::collections::HashMap;
use std::fmt::format;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, RwLock};
use std::{env, thread};

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

    let (tx_frame, rx_frame): (Sender<Vec<Frame>>, Receiver<Vec<Frame>>) = mpsc::channel();
    let (tx_joycon, rx_joycon): (Sender<Vec<InputEvent>>, Receiver<Vec<InputEvent>>) =
        mpsc::channel();
    let (tx_debug, rx_debug): (Sender<Vec<String>>, Receiver<Vec<String>>) = mpsc::channel();

    let bank = Arc::new(RwLock::new(0 as usize));
    let bank_for_render = bank.clone();

    let render_thread =
        thread::spawn(move || create_render_thread(rx_frame, tx_joycon, rx_debug, bank_for_render));

    let ppu = PPU::new(rom.chr_rom.clone(), rom.screen_mirroring);
    let mut bus = NESBus::new_with_callback(
        ppu,
        Box::new(move |ppu, joypad| {
            let mut game_frame = Frame::new();
            game_frame.data = ppu.frame.to_vec();

            let mut nt1_frame = Frame::new();
            let viewport = Rectangle::new(0, 0, Frame::WIDTH, Frame::HEIGHT);
            render::render_name_table(ppu, &mut nt1_frame, &ppu.vram[0..0x400], viewport, 0, 0);

            let mut nt2_frame = Frame::new();
            let viewport = Rectangle::new(0, 0, Frame::WIDTH, Frame::HEIGHT);
            render::render_name_table(ppu, &mut nt2_frame, &ppu.vram[0x400..0x800], viewport, 0, 0);

            let chr_frame = {
                let guard = bank.read().unwrap();
                render::debug::show_tiles(ppu.chr_rom.as_slice(), *guard)
            };

            tx_frame
                .send(vec![game_frame, nt1_frame, nt2_frame, chr_frame])
                .expect("Should send frames");

            let mut debug_oam: Vec<String> = vec![];
            for i in 0..26 {
                debug_oam.push(format!(
                    "{}: ({}, {}) ID: {:02X} AT: {:02X}",
                    i,
                    ppu.oam_data[i * 4 + 3],
                    ppu.oam_data[i * 4 + 0],
                    ppu.oam_data[i * 4 + 1],
                    ppu.oam_data[i * 4 + 2]
                ));
            }

            tx_debug.send(debug_oam).unwrap();

            for key_event in rx_joycon.recv().expect("Should receive joycon state") {
                update_joypad_state(joypad, key_event);
            }
        }),
    );
    bus.rom = Some(Box::from(rom));

    let mut cpu = CPU::new(Box::from(bus));
    cpu.reset();
    cpu.run();

    render_thread
        .join()
        .expect("Should be able to attach to the render thread");
}

fn create_render_thread(
    rx_frame: Receiver<Vec<Frame>>,
    tx_joycon: Sender<Vec<InputEvent>>,
    rx_debug: Receiver<Vec<String>>,
    bank: Arc<RwLock<usize>>,
) -> ! {
    println!("Started render thread");

    let sdl_context = sdl2::init().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window(
            "NES Emulator in Rust by acr92",
            (Frame::WIDTH as f32 * 2.0 * WINDOW_SCALE) as u32,
            (Frame::HEIGHT as f32 * 2.0 * WINDOW_SCALE) as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let key_map = create_keymap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(WINDOW_SCALE, WINDOW_SCALE).unwrap();

    let creator = canvas.texture_creator();
    let mut game_texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .unwrap();

    let mut nt1_texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .unwrap();

    let mut nt2_texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .unwrap();

    let mut chr_rom_texture = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            Frame::WIDTH as u32,
            Frame::HEIGHT as u32,
        )
        .unwrap();

    let mut font = ttf_context.load_font("09809_COURIER.ttf", 42).unwrap();

    let mut debug_text = creator
        .create_texture_target(
            PixelFormatEnum::RGB24,
            (Frame::WIDTH * WINDOW_SCALE as usize) as u32,
            (Frame::HEIGHT * WINDOW_SCALE as usize) as u32,
        )
        .unwrap();

    loop {
        let mut frames = rx_frame.recv().unwrap();

        let game_frame = &frames[0];
        // TODO: clean this up
        let nt1_frame = &frames[1];
        let nt2_frame = &frames[2];
        let chr_rom_frame = &frames[3];

        game_texture
            .update(None, &game_frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();
        nt1_texture
            .update(None, &nt1_frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();
        nt2_texture
            .update(None, &nt2_frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();
        chr_rom_texture
            .update(None, &chr_rom_frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();

        let debug_strings = rx_debug.recv().unwrap();

        canvas
            .with_texture_canvas(&mut debug_text, |c| {
                c.clear();

                let lines = vec![String::from("DEBUG:")];
                for (index, line) in lines.iter().chain(debug_strings.iter()).enumerate() {
                    let color = if index == 0 {
                        Color::RGBA(255, 0, 0, 255)
                    } else {
                        Color::RGBA(255, 255, 255, 255)
                    };

                    let surface = font.render(line).blended(color).unwrap();

                    let texture_creator = c.texture_creator();
                    let texture = texture_creator
                        .create_texture_from_surface(surface)
                        .unwrap();

                    let TextureQuery { width, height, .. } = texture.query();

                    c.copy(
                        &texture,
                        None,
                        Some(Rect::new(
                            16,
                            16 + ((height as f32) * (index as f32) * 1.5) as i32,
                            width as u32,
                            height as u32,
                        )),
                    )
                    .unwrap();
                }

                ()
            })
            .unwrap();

        canvas
            .copy(
                &game_texture,
                None,
                Some(Rect::new(0, 0, Frame::WIDTH as u32, Frame::HEIGHT as u32)),
            )
            .unwrap();
        canvas
            .copy(
                &debug_text,
                None,
                Some(Rect::new(
                    Frame::WIDTH as i32,
                    0,
                    Frame::WIDTH as u32,
                    Frame::HEIGHT as u32,
                )),
            )
            .unwrap();
        canvas
            .copy(
                &nt1_texture,
                None,
                Some(Rect::new(
                    0,
                    Frame::HEIGHT as i32,
                    Frame::WIDTH as u32,
                    Frame::HEIGHT as u32,
                )),
            )
            .unwrap();
        canvas
            .copy(
                &chr_rom_texture,
                None,
                Some(Rect::new(
                    Frame::WIDTH as i32,
                    Frame::HEIGHT as i32,
                    Frame::WIDTH as u32,
                    Frame::HEIGHT as u32,
                )),
            )
            .unwrap();

        canvas.present();
        let key_events = process_input(&key_map, &mut event_pump);

        for event in &key_events {
            if let InputButton::Key(key) = event.button {
                if event.key_down {
                    // We only care about on key released
                    continue;
                }

                if matches!(key, InputAction::CaptureScreenshot) {
                    save_screenshot(&mut frames[0]).unwrap();
                } else if matches!(key, InputAction::FlipChrBank) {
                    let mut bank_ref = bank.write().unwrap();
                    *bank_ref = if *bank_ref == 0 { 1 } else { 0 };
                }
            }
        }

        tx_joycon.send(key_events).unwrap();
    }
}

fn process_input(
    key_map: &HashMap<Keycode, InputButton>,
    event_pump: &mut EventPump,
) -> Vec<InputEvent> {
    let mut key_events: Vec<InputEvent> = vec![];
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),

            Event::KeyDown { keycode, .. } => {
                if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                    key_events.push(InputEvent::pressed(*key))
                }
            }
            Event::KeyUp { keycode, .. } => {
                if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                    key_events.push(InputEvent::released(*key))
                }
            }
            _ => {}
        }
    }
    key_events
}

fn update_joypad_state(joypad: &mut Joypad, key_event: InputEvent) {
    if let InputButton::Joypad(joypad_button) = key_event.button {
        if key_event.key_down {
            joypad.set_pressed(joypad_button);
        } else {
            joypad.set_released(joypad_button);
        }
    }
}

fn save_screenshot(frame: &mut Frame) -> Result<(), String> {
    Surface::from_data(
        frame.data.as_mut_slice(),
        Frame::WIDTH as u32,
        Frame::HEIGHT as u32,
        (Frame::WIDTH * Frame::RGB_SIZE) as u32,
        PixelFormatEnum::RGB24,
    )
    .unwrap()
    .save_bmp(Path::new("hello.bmp"))?;

    println!("Saved screenshot");
    Ok(())
}
