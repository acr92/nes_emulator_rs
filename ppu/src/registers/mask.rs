use bitflags::bitflags;

bitflags! {
    /// # Mask Register (PPUMASK) https://www.nesdev.org/wiki/PPU_registers
    ///
    /// 7  bit  0
    /// ---- ----
    /// BGRs bMmG
    /// |||| ||||
    /// |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
    /// |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    /// |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    /// |||| +---- 1: Show background
    /// |||+------ 1: Show sprites
    /// ||+------- Emphasize red (green on PAL/Dendy)
    /// |+-------- Emphasize green (red on PAL/Dendy)
    /// +--------- Emphasize blue
    ///
    pub struct MaskRegister: u8 {
        const GRAYSCALE                 = 0b00000001;
        const SHOW_BACKGROUND_LEFTMOST  = 0b00000010;
        const SHOW_SPRITES_LEFTMOST     = 0b00000100;
        const SHOW_BACKGROUND           = 0b00001000;
        const SHOW_SPRITES              = 0b00010000;
        const EMPHASIZE_RED             = 0b00100000;
        const EMPHASIZE_GREEN           = 0b01000000;
        const EMPHASIZE_BLUE            = 0b10000000;
    }
}

impl MaskRegister {
    pub fn new() -> Self {
        MaskRegister::from_bits_truncate(0)
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}