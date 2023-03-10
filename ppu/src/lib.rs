use core::mem::Mem;
use core::cartridge::Mirroring;
use crate::register::Registers;

mod register;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_SIZE: usize = 0x08;
const PPU_REGISTERS_END: u16 = PPU_REGISTERS_START + (PPU_REGISTERS_SIZE as u16) - 1;

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
        match addr {
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.registers.mem_read(addr),
            _ => panic!("Handle read to addr {:04X}", addr),
        }
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        match addr {
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.registers.mem_write(addr, value),
            _ => panic!("Handle write to addr {:04X}", addr),
        }
    }
}

#[cfg(test)]
mod test {

}