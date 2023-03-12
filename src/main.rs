use cpu6502::cpu::CPU;
use emulator::bus::NESBus;
use emulator::cartridge::Rom;
use emulator::joypad::JoypadButton;
use ppu::PPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::{env, thread};

const WINDOW_SCALE: f32 = 3.0;

struct JoypadEvent {
    button: JoypadButton,
    key_down: bool,
}

impl JoypadEvent {
    fn pressed(button: JoypadButton) -> Self {
        JoypadEvent {
            button,
            key_down: true,
        }
    }

    fn released(button: JoypadButton) -> Self {
        JoypadEvent {
            button,
            key_down: false,
        }
    }
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

    let (tx_frame, rx_frame): (Sender<Frame>, Receiver<Frame>) = mpsc::channel();
    let (tx_joycon, rx_joycon): (Sender<Vec<JoypadEvent>>, Receiver<Vec<JoypadEvent>>) =
        mpsc::channel();

    let render_thread = thread::spawn(move || {
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
            let frame = rx_frame.recv().unwrap();

            texture
                .update(None, &frame.data, Frame::WIDTH * Frame::RGB_SIZE)
                .unwrap();

            canvas.copy(&texture, None, None).unwrap();

            canvas.present();
            let mut key_events: Vec<JoypadEvent> = vec![];
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => std::process::exit(0),

                    Event::KeyDown { keycode, .. } => {
                        if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                            key_events.push(JoypadEvent::pressed(*key))
                        }
                    }
                    Event::KeyUp { keycode, .. } => {
                        if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::AcBack)) {
                            key_events.push(JoypadEvent::released(*key))
                        }
                    }
                    _ => {}
                }
            }

            tx_joycon.send(key_events).unwrap();
        }
    });

    let ppu = PPU::new(rom.chr_rom.clone(), rom.screen_mirroring);
    let mut bus = NESBus::new_with_callback(
        ppu,
        Box::new(move |ppu, joypad| {
            let mut frame = Frame::new();
            render::render(ppu, &mut frame);
            tx_frame.send(frame).expect("Should send frame");

            for event in rx_joycon.recv().expect("Should receive joycon state") {
                if event.key_down {
                    joypad.set_pressed(event.button);
                } else {
                    joypad.set_released(event.button);
                }
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
