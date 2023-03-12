use emulator::joypad::JoypadButton;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum InputAction {
    CaptureScreenshot,
}

#[derive(Copy, Clone)]
pub enum InputButton {
    Joypad(JoypadButton),
    Key(InputAction),
}

pub struct InputEvent {
    pub button: InputButton,
    pub key_down: bool,
}

impl InputEvent {
    pub fn pressed(button: InputButton) -> Self {
        InputEvent {
            button,
            key_down: true,
        }
    }

    pub fn released(button: InputButton) -> Self {
        InputEvent {
            button,
            key_down: false,
        }
    }
}

pub fn create_keymap() -> HashMap<Keycode, InputButton> {
    let mut key_map: HashMap<Keycode, InputButton> = HashMap::new();
    key_map.insert(Keycode::Down, InputButton::Joypad(JoypadButton::DOWN));
    key_map.insert(Keycode::Up, InputButton::Joypad(JoypadButton::UP));
    key_map.insert(Keycode::Right, InputButton::Joypad(JoypadButton::RIGHT));
    key_map.insert(Keycode::Left, InputButton::Joypad(JoypadButton::LEFT));
    key_map.insert(Keycode::Space, InputButton::Joypad(JoypadButton::SELECT));
    key_map.insert(Keycode::Return, InputButton::Joypad(JoypadButton::START));
    key_map.insert(Keycode::A, InputButton::Joypad(JoypadButton::BUTTON_A));
    key_map.insert(Keycode::S, InputButton::Joypad(JoypadButton::BUTTON_B));

    key_map.insert(Keycode::G, InputButton::Key(InputAction::CaptureScreenshot));

    key_map
}
