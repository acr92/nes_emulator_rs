use crate::bus::Mem;

pub struct PPU {
    registers: [u8; 8],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            registers: [0x00; 8],
        }
    }
}

impl PPU {}

impl Mem for PPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.registers[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.registers[addr as usize] = value
    }
}
