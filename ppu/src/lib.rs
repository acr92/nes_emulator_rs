use crate::register::{
    is_read_allowed, is_write_allowed, RegisterField, Registers, PPU_REGISTERS_MAP,
};
use core::cartridge::Mirroring;
use core::mem::Mem;

mod register;

const PALETTE_TABLE_SIZE: usize = 32;
const PPU_VRAM_SIZE: usize = 2048;
const OAM_DATA_SIZE: usize = 256;

pub struct PPU {
    chr_rom: Vec<u8>,
    palette_table: [u8; PALETTE_TABLE_SIZE],
    vram: [u8; PPU_VRAM_SIZE],
    oam_data: [u8; OAM_DATA_SIZE],
    mirroring: Mirroring,

    registers: Registers,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            chr_rom: vec![],
            palette_table: [0; PALETTE_TABLE_SIZE],
            vram: [0; PPU_VRAM_SIZE],
            oam_data: [0; OAM_DATA_SIZE],
            mirroring: Mirroring::Vertical,

            registers: Registers::new(),
        }
    }
}

impl PPU {}

impl Mem for PPU {
    fn mem_read(&self, addr: u16) -> u8 {
        let register = PPU_REGISTERS_MAP
            .get(&addr)
            .expect(&format!("Unexpected addr {:04X}", addr));

        if !is_read_allowed(register) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Status => self.registers.status.bits(),
            RegisterField::OAMData => self.registers.oam_data,
            RegisterField::Data => self.registers.data,
            _ => panic!("Unexpected read on {:#?}", register),
        }
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        let register = PPU_REGISTERS_MAP
            .get(&addr)
            .expect(&format!("Unexpected addr {:04X}", addr));

        if !is_write_allowed(register) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Controller => self.registers.set_controller(value),
            RegisterField::Mask => self.registers.set_mask(value),
            RegisterField::OAMAddress => self.registers.oam_address = value,
            RegisterField::OAMData => self.registers.oam_data = value,
            RegisterField::Scroll => self.registers.scroll = value,
            RegisterField::Address => self.registers.address = value,
            RegisterField::Data => self.registers.data = value,
            RegisterField::OAMDMA => self.registers.oam_dma = value,
            _ => panic!("Unexpected write on {:#?}", register),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::PPU;
    use core::mem::Mem;

    #[ignore]
    #[test]
    fn test_memory_access_emulation_is_correct() {
        let mut ppu = PPU::new();
        ppu.mem_write(0x2006, 0x06);
        ppu.mem_write(0x2006, 0x00);
        assert_eq!(ppu.mem_read(0x2007), 0x00);
        assert_eq!(ppu.mem_read(0x2007), 0xFF);
    }
}
