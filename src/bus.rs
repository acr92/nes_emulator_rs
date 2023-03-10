//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               |
// | $0000-$07FF   |       |               |
// |_ _ _ _ _ _ _ _| $0800 |               |
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|

use crate::cartridge::Rom;
use core::mem::Mem;
use ppu::PPU;

const CPU_VRAM_SIZE: usize = 0x800;
const RAM_START: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x2000 - 1;
const RAM_MIRRORS_MASK: u16 = 0x800 - 1;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_SIZE: usize = 0x08;
const PPU_REGISTERS_END: u16 = PPU_REGISTERS_START + (PPU_REGISTERS_SIZE as u16) - 1;
const PPU_REGISTERS_MIRRORS_START: u16 = PPU_REGISTERS_END + 1;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const PPU_REGISTER_OAMDMA: u16 = 0x4014;

const APU_REGISTERS_START: u16 = 0x4000;
const APU_REGISTERS_SIZE: usize = 0x18 + 0x08;
const APU_REGISTERS_END: u16 = APU_REGISTERS_START + (APU_REGISTERS_SIZE as u16) - 1;

const PRG_START: u16 = 0x8000;
const PRG_END: u16 = 0xFFFF;

pub struct Bus {
    cpu_vram: [u8; CPU_VRAM_SIZE],
    apu: [u8; APU_REGISTERS_SIZE],
    pub ppu: PPU,
    pub rom: Option<Box<Rom>>,
}

impl Bus {
    pub fn new(ppu: PPU) -> Self {
        Bus {
            cpu_vram: [0; CPU_VRAM_SIZE],
            apu: [0xFF; APU_REGISTERS_SIZE],
            ppu,
            rom: None,
        }
    }
}

impl Bus {
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        if let Some(rom) = &self.rom {
            addr -= PRG_START;
            if rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
                addr %= 0x4000;
            }
            rom.prg_rom[addr as usize]
        } else {
            0xFF
        }
    }
}

impl Mem for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM_START..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & RAM_MIRRORS_MASK;
                self.cpu_vram[mirror_down_addr as usize]
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END | PPU_REGISTER_OAMDMA => {
                self.ppu.mem_read(addr)
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                self.mem_read(addr & PPU_REGISTERS_END)
            }
            APU_REGISTERS_START..=APU_REGISTERS_END => {
                self.apu[(addr - APU_REGISTERS_START) as usize]
            }
            PRG_START..=PRG_END => self.read_prg_rom(addr),
            _ => {
                println!("WARN: Ignoring read 0x{:X}", addr);
                0x00
            }
        }
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        match addr {
            RAM_START..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & RAM_MIRRORS_MASK;
                self.cpu_vram[mirror_down_addr as usize] = value;
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END | PPU_REGISTER_OAMDMA => {
                self.ppu.mem_write(addr, value)
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                self.mem_write(addr & PPU_REGISTERS_END, value)
            }
            APU_REGISTERS_START..=APU_REGISTERS_END => {
                self.apu[(addr - APU_REGISTERS_START) as usize] = value;
            }
            PRG_START..=PRG_END => {
                panic!("Attempt to write to Cartridge ROM space")
            }
            _ => {
                println!("WARN: Ignoring write 0x{:X} = 0x{:X}", addr, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_read() {
        let bus = Bus::new(PPU::new());
        let value = bus.mem_read(0x0001);
        assert_eq!(value, 0);
    }

    #[test]
    fn test_ram_write() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write(0x0001, 0xAA);
        assert_eq!(bus.cpu_vram[0x0001], 0xAA);
    }

    #[test]
    fn test_ram_read_and_write() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write(0x800, 0xCA);
        assert_eq!(bus.mem_read(0x800), 0xCA);
    }

    #[test]
    fn test_ram_read_and_write_mirror() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write(0x000, 0x01);
        bus.mem_write(0x800, bus.mem_read(0x800) + 1);
        bus.mem_write(0x1000, bus.mem_read(0x1000) + 1);
        bus.mem_write(0x1800, bus.mem_read(0x1800) + 1);
        assert_eq!(bus.mem_read(0x1800), 4);
    }

    #[test]
    fn test_ppu_read() {
        let bus = Bus::new(PPU::new());
        assert_eq!(bus.mem_read(0x2007), 0x00);
    }

    #[test]
    fn test_ppu_write() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write(0x2007, 0xBB);
        assert_eq!(bus.mem_read(0x2007), 0xBB);
    }

    #[test]
    fn test_ppu_mask() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write(0x200F, 0xBB);
        assert_eq!(bus.mem_read(0x2007), 0xBB)
    }

    #[test]
    fn test_cartridge_read() {
        let mut bus = Bus::new(PPU::new());
        bus.rom = Some(Box::from(crate::cartridge::test::create_example_rom()));
        assert_eq!(bus.mem_read(PRG_START + 0x800), 0x01);
    }

    #[test]
    #[should_panic(expected = "Attempt to write to Cartridge ROM space")]
    fn test_cannot_write_to_cartridge() {
        let mut bus = Bus::new(PPU::new());
        bus.mem_write_u16(0xFFFC, 0x1234);
        assert_eq!(bus.mem_read_u16(0xFFFC), 0x1234);
    }
}
