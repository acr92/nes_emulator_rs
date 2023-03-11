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
use crate::joypad::Joypad;
use core::mem::Mem;
use ppu::{OAM_DATA_SIZE, PPU};

const CPU_VRAM_SIZE: usize = 0x800;
const RAM_START: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x2000 - 1;
const RAM_MIRRORS_MASK: u16 = 0x800 - 1;

const PPU_REGISTERS_START: u16 = 0x2000;
const PPU_REGISTERS_SIZE: usize = 0x08;
const PPU_REGISTERS_END: u16 = PPU_REGISTERS_START + (PPU_REGISTERS_SIZE as u16) - 1;
const PPU_REGISTERS_MIRRORS_START: u16 = PPU_REGISTERS_END + 1;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;
const PPU_REGISTER_OAM_DMA: u16 = 0x4014;

const JOYPAD_1_ADDR: u16 = 0x4016;
const JOYPAD_2_ADDR: u16 = 0x4017;

const PRG_START: u16 = 0x8000;
const PRG_END: u16 = 0xFFFF;

pub struct Bus<'call> {
    cpu_vram: [u8; CPU_VRAM_SIZE],
    pub ppu: PPU,
    pub rom: Option<Box<Rom>>,
    pub joypad1: Joypad,

    pub cycles: usize,
    gameloop_callback: Box<dyn FnMut(&PPU, &mut Joypad) + 'call>,
}

impl<'a> Bus<'a> {
    pub fn new<'call>(ppu: PPU) -> Self {
        Bus::new_with_callback(ppu, |_ppu, _joypad| {})
    }

    pub fn new_with_callback<'call, F>(ppu: PPU, gameloop_callback: F) -> Self
    where
        F: FnMut(&PPU, &mut Joypad) + 'call + 'a,
    {
        Bus {
            cpu_vram: [0; CPU_VRAM_SIZE],
            ppu,
            rom: None,
            joypad1: Joypad::new(),

            cycles: 0,
            gameloop_callback: Box::from(gameloop_callback),
        }
    }

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

    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;

        let new_frame = self.ppu.tick(cycles * 3);

        if new_frame {
            (self.gameloop_callback)(&self.ppu, &mut self.joypad1);
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.nmi_interrupt.take()
    }
}

impl Mem for Bus<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM_START..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & RAM_MIRRORS_MASK;
                self.cpu_vram[mirror_down_addr as usize]
            }
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.ppu.mem_read(addr),
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                self.mem_read(addr & PPU_REGISTERS_END)
            }
            0x4000..=0x4015 => {
                // Ignore APU
                0xFF
            }
            JOYPAD_1_ADDR => self.joypad1.read(),
            JOYPAD_2_ADDR => 0x00,
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
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.ppu.mem_write(addr, value),
            PPU_REGISTER_OAM_DMA => {
                let mut buffer: [u8; OAM_DATA_SIZE] = [0; OAM_DATA_SIZE];
                let hi: u16 = (value as u16) << 8;
                for i in 0..OAM_DATA_SIZE {
                    buffer[i as usize] = self.mem_read(hi + (i as u16));
                }

                self.ppu.write_oam_dma(&buffer);
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                self.mem_write(addr & PPU_REGISTERS_END, value)
            }
            0x4000..=0x4013 | 0x4015 => {
                // Ignore APU
            }
            JOYPAD_1_ADDR => self.joypad1.write(value),
            JOYPAD_2_ADDR => {
                // We only use 1 joy pad
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
        let mut bus = Bus::new(PPU::new_empty_rom());
        let value = bus.mem_read(0x0001);
        assert_eq!(value, 0);
    }

    #[test]
    fn test_ram_write() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(0x0001, 0xAA);
        assert_eq!(bus.cpu_vram[0x0001], 0xAA);
    }

    #[test]
    fn test_ram_read_and_write() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(0x800, 0xCA);
        assert_eq!(bus.mem_read(0x800), 0xCA);
    }

    #[test]
    fn test_ram_read_and_write_mirror() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(0x000, 0x01);

        let value = bus.mem_read(0x800) + 1;
        bus.mem_write(0x800, value);

        let value = bus.mem_read(0x1000) + 1;
        bus.mem_write(0x1000, value);

        let value = bus.mem_read(0x1800) + 1;
        bus.mem_write(0x1800, value);

        assert_eq!(bus.mem_read(0x1800), 4);
    }

    #[test]
    fn test_ppu_read() {
        let ppu = PPU::new_empty_rom();
        let mut bus = Bus::new(ppu);
        assert_eq!(bus.mem_read(0x2004), 0x00);
    }

    #[test]
    fn test_ppu_write() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(0x2006, 0x21);
        bus.mem_write(0x2006, 0x00);
        bus.mem_write(0x2007, 0xBB);

        bus.mem_write(0x2006, 0x21);
        bus.mem_write(0x2006, 0x00);
        bus.mem_read(0x2007);
        assert_eq!(bus.mem_read(0x2007), 0xBB);
    }

    #[test]
    fn test_ppu_mask() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(0x200E, 0x21);
        bus.mem_write(0x200E, 0x00);
        bus.mem_write(0x200F, 0xBB);

        bus.mem_write(0x200E, 0x21);
        bus.mem_write(0x200E, 0x00);
        bus.mem_read(0x200F);
        assert_eq!(bus.mem_read(0x2007), 0xBB);
    }

    #[test]
    fn test_cartridge_read() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.rom = Some(Box::from(crate::cartridge::test::create_example_rom()));
        assert_eq!(bus.mem_read(PRG_START + 0x800), 0x01);
    }

    #[test]
    #[should_panic(expected = "Attempt to write to Cartridge ROM space")]
    fn test_cannot_write_to_cartridge() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write_u16(0xFFFC, 0x1234);
        assert_eq!(bus.mem_read_u16(0xFFFC), 0x1234);
    }
}
