use bitflags::bitflags;

bitflags! {
       // https://wiki.nesdev.com/w/index.php/Controller_reading_code
       pub struct JoypadButton: u8 {
           const RIGHT             = 0b10000000;
           const LEFT              = 0b01000000;
           const DOWN              = 0b00100000;
           const UP                = 0b00010000;
           const START             = 0b00001000;
           const SELECT            = 0b00000100;
           const BUTTON_B          = 0b00000010;
           const BUTTON_A          = 0b00000001;
       }
}

pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: JoypadButton,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: JoypadButton::from_bits_truncate(0),
        }
    }

    pub fn write(&mut self, data: u8) {
        self.strobe = data & 1 == 1;
        if self.strobe {
            self.button_index = 0
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }

        let response = (self.button_status.bits & (1 << self.button_index)) >> self.button_index;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }
        response
    }

    pub fn set_pressed(&mut self, button: JoypadButton) {
        self.button_status.set(button, true)
    }

    pub fn set_released(&mut self, button: JoypadButton) {
        self.button_status.set(button, false);
    }
}

#[cfg(test)]
mod test {
    use crate::joypad::{Joypad, JoypadButton};
    use k9::assert_equal;

    #[test]
    fn test_joypad() {
        let mut joypad = Joypad::new();

        start_polling(&mut joypad);
        stop_polling(&mut joypad);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);

        start_polling(&mut joypad);
        joypad.set_pressed(JoypadButton::BUTTON_B);
        stop_polling(&mut joypad);

        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 1); // BUTTON_B index
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);

        start_polling(&mut joypad);
        joypad.set_released(JoypadButton::BUTTON_B);
        stop_polling(&mut joypad);

        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
        assert_equal!(joypad.read(), 0);
    }

    fn stop_polling(joypad: &mut Joypad) {
        joypad.write(0);
    }

    fn start_polling(joypad: &mut Joypad) {
        joypad.write(1);
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Joypad::new()
    }
}
