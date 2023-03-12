use bitflags::bitflags;
use core::ppu::{NAMETABLE_0, NAMETABLE_1, NAMETABLE_2, NAMETABLE_3};

bitflags! {
    /// # Controller Register (PPUCTRL) https://www.nesdev.org/wiki/PPU_registers
    ///
    ///  7  bit  0
    ///  ---- ----
    ///  VPHB SINN
    ///  |||| ||||
    ///  |||| ||++- Base nametable address
    ///  |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    ///  |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    ///  |||| |     (0: add 1, going across; 1: add 32, going down)
    ///  |||| +---- Sprite pattern table address for 8x8 sprites
    ///  ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    ///  |||+------ Background pattern table address (0: $0000; 1: $1000)
    ///  ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
    ///  |+-------- PPU master/slave select
    ///  |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    ///  +--------- Generate an NMI at the start of the
    ///             vertical blanking interval (0: off; 1: on)
    ///
    pub struct ControlRegister: u8 {
        const NAMETABLE1                = 0b00000001;
        const NAMETABLE2                = 0b00000010;
        const VRAM_ADD_INCREMENT        = 0b00000100;
        const SPRITE_PATTERN_ADDR       = 0b00001000;
        const BACKGROUND_PATTERN_ADDR   = 0b00010000;
        const SPRITE_SIZE               = 0b00100000;
        const PPU_MASTER_SLAVE_SELECT   = 0b01000000;
        const GENERATE_NMI_AT_VBI       = 0b10000000;
    }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0)
    }

    pub fn nametable_address(&self) -> u16 {
        match self.bits & 0b11 {
            0 => NAMETABLE_0,
            1 => NAMETABLE_1,
            2 => NAMETABLE_2,
            3 => NAMETABLE_3,
            _ => panic!("Not possible"),
        }
    }

    pub fn vram_address_increment(&self) -> u8 {
        if self.contains(ControlRegister::VRAM_ADD_INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn sprite_pattern_table_address(&self) -> u16 {
        if self.contains(ControlRegister::SPRITE_PATTERN_ADDR) {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn background_pattern_table_address(&self) -> u16 {
        if self.contains(ControlRegister::BACKGROUND_PATTERN_ADDR) {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if self.contains(ControlRegister::SPRITE_SIZE) {
            16
        } else {
            8
        }
    }

    pub fn master_slave_select(&self) -> u8 {
        if self.contains(ControlRegister::PPU_MASTER_SLAVE_SELECT) {
            1
        } else {
            0
        }
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.contains(ControlRegister::GENERATE_NMI_AT_VBI)
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}

impl Default for ControlRegister {
    fn default() -> Self {
        ControlRegister::new()
    }
}
