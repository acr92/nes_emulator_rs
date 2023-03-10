use crate::register::{
    is_read_allowed, is_write_allowed, RegisterField, Registers, PPU_REGISTERS_MAP,
};
use core::cartridge::Mirroring;
use core::mem::Mem;

mod registers;
mod register;

const PATTERN_TABLE_START: u16 = 0x0000;
const PATTERN_TABLE_END: u16 = 0x1FFF;
const NAMETABLE_START: u16 = 0x2000;
const NAMETABLE_END: u16 = 0x2FFF;
const NAMETABLE_MIRROR_START: u16 = 0x3000;
const NAMETABLE_MIRROR_END: u16 = 0x3EFF;
const PALETTE_RAM_START: u16 = 0x3F00;
const PALETTE_RAM_END: u16 = 0x3F1F;

const PALETTE_TABLE_SIZE: usize = 32;
const PPU_VRAM_SIZE: usize = 2048;
const OAM_DATA_SIZE: usize = 256;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    palette_table: [u8; PALETTE_TABLE_SIZE],
    vram: [u8; PPU_VRAM_SIZE],
    oam_data: [u8; OAM_DATA_SIZE],
    mirroring: Mirroring,

    registers: Registers,
    internal_data_buf: u8,
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
            internal_data_buf: 0,
        }
    }

    fn increment_vram_addr(&mut self) {
        self.registers
            .address
            .increment(self.registers.control.vram_address_increment());
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.registers.address.get();
        self.increment_vram_addr();

        match addr {
            PATTERN_TABLE_START..=PATTERN_TABLE_END => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[addr as usize];
                result
            }
            NAMETABLE_START..=NAMETABLE_END => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            NAMETABLE_MIRROR_START..=NAMETABLE_MIRROR_END => panic!(
                "addr space 0x3000..0x3EFF is not expected to be used, requested = {:04X}",
                addr
            ),
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                self.palette_table[(addr - PALETTE_RAM_START) as usize]
            }
            _ => panic!("Unexpected access to mirrored space {:04X}", addr),
        }
    }

    fn write_data(&mut self, value: u8) {
        let addr = self.registers.address.get();
        self.increment_vram_addr();

        match addr {
            PATTERN_TABLE_START..=PATTERN_TABLE_END => panic!("Write to chr_rom not allowed"),
            NAMETABLE_START..=NAMETABLE_END => todo!("Write to RAM"),
            NAMETABLE_MIRROR_START..=NAMETABLE_MIRROR_END => panic!(
                "addr space 0x3000..0x3EFF is not expected to be used, requested = {:04X}",
                addr
            ),
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                self.palette_table[(addr - PALETTE_RAM_START) as usize] = value
            }
            _ => panic!("Unexpected access to mirrored space {:04X}", addr),
        }
    }
    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index

        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }
}

impl Mem for PPU {
    fn mem_read(&mut self, addr: u16) -> u8 {
        let register = PPU_REGISTERS_MAP
            .get(&addr)
            .expect(&format!("Unexpected addr {:04X}", addr));

        if !is_read_allowed(register) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Status => self.registers.status.bits(),
            RegisterField::OAMData => self.registers.oam_data,
            RegisterField::Data => self.read_data(),
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
            RegisterField::Control => self.registers.control.update(value),
            RegisterField::Mask => self.registers.mask.update(value),
            RegisterField::OAMAddress => self.registers.oam_address = value,
            RegisterField::OAMData => self.registers.oam_data = value,
            RegisterField::Scroll => self.registers.scroll = value,
            RegisterField::Address => self.registers.address.update(value),
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

    #[test]
    fn test_memory_access_emulation_is_correct() {
        let mut ppu = PPU::new();
        ppu.chr_rom = vec![0xFF; 2048];

        ppu.mem_write(0x2006, 0x06);
        ppu.mem_write(0x2006, 0x00);
        assert_eq!(ppu.mem_read(0x2007), 0x00);
        assert_eq!(ppu.mem_read(0x2007), 0xFF);
    }
}
