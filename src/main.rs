mod input;

use crate::input::{create_keymap, InputAction, InputButton, InputEvent};
use cpu6502::cpu::CPU;
use emulator::bus::NESBus;
use emulator::cartridge::Rom;
use emulator::joypad::Joypad;
use ppu::PPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::EventPump;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
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

    let (tx_frame, rx_frame): (Sender<Frame>, Receiver<Frame>) = mpsc::channel();
    let (tx_joycon, rx_joycon): (Sender<Vec<InputEvent>>, Receiver<Vec<InputEvent>>) =
        mpsc::channel();

    let render_thread = thread::spawn(move || create_render_thread(rx_frame, tx_joycon));

    let ppu = PPU::new(rom.chr_rom.clone(), rom.screen_mirroring);
    let mut bus = NESBus::new_with_callback(
        ppu,
        Box::new(move |ppu, joypad| {
            let mut frame = Frame::new();
            render::render(ppu, &mut frame);
            tx_frame.send(frame).expect("Should send frame");

            for key_event in rx_joycon.recv().expect("Should receive joycon state") {
                update_joypad_state(joypad, key_event)
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

fn create_render_thread(rx_frame: Receiver<Frame>, tx_joycon: Sender<Vec<InputEvent>>) -> ! {
    println!("Started render thread");

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

    loop {
        let mut frame = rx_frame.recv().unwrap();

        texture
            .update(None, &frame.data, Frame::WIDTH * Frame::RGB_SIZE)
            .unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        let key_events = process_input(&key_map, &mut event_pump);

        for event in &key_events {
            if let InputButton::Key(key) = event.button {
                if event.key_down {
                    // We only care about on key released
                    continue;
                }

                if matches!(key, InputAction::CaptureScreenshot) {
                    save_screenshot(&mut frame).unwrap();
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
