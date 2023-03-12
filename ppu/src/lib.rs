use crate::register::{
    is_read_allowed, is_write_allowed, RegisterField, Registers, PPU_REGISTERS_MAP,
};
use core::cartridge::Mirroring;
use core::mem::Mem;

mod register;
mod registers;

const PATTERN_TABLE_START: u16 = 0x0000;
const PATTERN_TABLE_END: u16 = 0x1FFF;
const NAMETABLE_START: u16 = 0x2000;
const NAMETABLE_END: u16 = 0x2FFF;
const NAMETABLE_MIRROR_START: u16 = 0x3000;
const NAMETABLE_MIRROR_END: u16 = 0x3EFF;
const PALETTE_RAM_START: u16 = 0x3F00;
const PALETTE_RAM_END: u16 = 0x3FFF;

const PALETTE_TABLE_SIZE: usize = 32;
const PPU_VRAM_SIZE: usize = 2048;
pub const CHR_ROM_BANK_SIZE: usize = 0x1000;
pub const OAM_DATA_SIZE: usize = 256;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; PALETTE_TABLE_SIZE],
    pub vram: [u8; PPU_VRAM_SIZE],
    pub oam_data: [u8; OAM_DATA_SIZE],
    pub mirroring: Mirroring,
    pub registers: Registers,
    internal_data_buf: u8,

    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: Option<u8>,
}

impl PPU {
    pub fn new_empty_rom() -> Self {
        PPU::new(vec![0; 2048], Mirroring::Horizontal)
    }

    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            palette_table: [0; PALETTE_TABLE_SIZE],
            vram: [0; PPU_VRAM_SIZE],
            oam_data: [0; OAM_DATA_SIZE],
            mirroring,

            registers: Registers::new(),
            internal_data_buf: 0,

            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            if self.is_sprite_zero_hit(self.cycles) {
                self.registers.status.set_sprite_zero_hit(true);
            }

            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.registers.status.set_vblank_status(true);
                self.registers.status.set_sprite_zero_hit(false);
                if self.registers.control.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                    return true;
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.registers.status.set_sprite_zero_hit(false);
                self.registers.status.reset_vblank_status();
                // self.registers.control = ControlRegister::new();
            }
        }

        false
    }

    fn is_sprite_zero_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.registers.mask.show_sprites()
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
                self.palette_table[self.map_palette_table_address_to_index(addr)]
            }
            _ => panic!("Unexpected access to mirrored space {:04X}", addr),
        }
    }

    fn write_to_data(&mut self, value: u8) {
        let addr = self.registers.address.get();

        match addr {
            PATTERN_TABLE_START..=PATTERN_TABLE_END => panic!("Write to chr_rom not allowed"),
            NAMETABLE_START..=NAMETABLE_END => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value
            }
            NAMETABLE_MIRROR_START..=NAMETABLE_MIRROR_END => unimplemented!(
                "addr space 0x3000..0x3EFF is not expected to be used, requested = {:04X}",
                addr
            ),
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                self.palette_table[self.map_palette_table_address_to_index(addr)] = value;
            }
            _ => panic!("Unexpected access to mirrored space {:04X}", addr),
        }

        self.increment_vram_addr();
    }

    fn map_palette_table_address_to_index(&self, addr: u16) -> usize {
        let addr = (addr - PALETTE_RAM_START) as usize % self.palette_table.len();

        // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
        match addr {
            0x10 | 0x14 | 0x18 | 0x1C => addr - 0x10,
            _ => addr,
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

    fn write_to_ppu_address(&mut self, value: u8) {
        self.registers.address.update(value)
    }

    fn write_to_control(&mut self, value: u8) {
        let before_nmi_status = self.registers.control.generate_vblank_nmi();
        self.registers.control.update(value);
        if !before_nmi_status
            && self.registers.control.generate_vblank_nmi()
            && self.registers.status.is_in_vblank()
        {
            self.nmi_interrupt = Some(1)
        }
    }

    fn read_status(&mut self) -> u8 {
        let data = self.registers.status.snapshot();
        self.registers.status.reset_vblank_status();
        self.registers.address.reset_latch();
        self.registers.scroll.reset_latch();
        data
    }

    fn write_to_oam_address(&mut self, value: u8) {
        self.registers.oam_address = value
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.registers.oam_address as usize] = value;
        self.registers.oam_address = self.registers.oam_address.wrapping_add(1);
    }

    fn read_oam_data(&mut self) -> u8 {
        self.oam_data[self.registers.oam_address as usize]
    }

    pub fn write_oam_dma(&mut self, data: &[u8; OAM_DATA_SIZE]) {
        for x in data.iter() {
            self.oam_data[self.registers.oam_address as usize] = *x;
            self.registers.oam_address = self.registers.oam_address.wrapping_add(1);
        }
    }
}

impl Mem for PPU {
    fn mem_read(&mut self, addr: u16) -> u8 {
        let register = PPU_REGISTERS_MAP
            .get(&addr)
            .unwrap_or_else(|| panic!("Unexpected addr {:04X}", addr));

        if !is_read_allowed(register) {
            println!("Tried to read from write-only {:#?}", register);
            return 0;
        }

        match register.field {
            RegisterField::Status => self.read_status(),
            RegisterField::OAMData => self.read_oam_data(),
            RegisterField::Data => self.read_data(),
            _ => panic!("Unexpected read on {:#?}", register),
        }
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        let register = PPU_REGISTERS_MAP
            .get(&addr)
            .unwrap_or_else(|| panic!("Unexpected addr {:04X}", addr));

        if !is_write_allowed(register) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Control => self.write_to_control(value),
            RegisterField::Mask => self.registers.mask.update(value),
            RegisterField::OAMAddress => self.write_to_oam_address(value),
            RegisterField::OAMData => self.write_to_oam_data(value),
            RegisterField::Scroll => self.registers.scroll.write(value),
            RegisterField::Address => self.write_to_ppu_address(value),
            RegisterField::Data => self.write_to_data(value),
            _ => panic!("Unexpected write on {:#?}", register),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::registers::control::ControlRegister;
    use crate::registers::mask::MaskRegister;
    use crate::registers::status::StatusRegister;
    use k9::assert_equal;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_ppu_address(0x23);
        ppu.write_to_ppu_address(0x05);
        ppu.write_to_data(0x66);

        assert_equal!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_control(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_address(0x23);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_equal!(ppu.registers.address.get(), 0x2306);
        assert_equal!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_control(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_address(0x21);
        ppu.write_to_ppu_address(0xff);

        ppu.read_data(); //load_into_buffer
        assert_equal!(ppu.read_data(), 0x66);
        assert_equal!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_control(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_address(0x21);
        ppu.write_to_ppu_address(0xff);

        ppu.read_data(); //load_into_buffer
        assert_equal!(ppu.read_data(), 0x66);
        assert_equal!(ppu.read_data(), 0x77);
        assert_equal!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_ppu_address(0x24);
        ppu.write_to_ppu_address(0x05);

        ppu.write_to_data(0x66); //write to a

        ppu.write_to_ppu_address(0x28);
        ppu.write_to_ppu_address(0x05);

        ppu.write_to_data(0x77); //write to B

        ppu.write_to_ppu_address(0x20);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load into buffer
        assert_equal!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_address(0x2C);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load into buffer
        assert_equal!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = PPU::new(vec![0; 2048], Mirroring::Vertical);

        ppu.write_to_ppu_address(0x20);
        ppu.write_to_ppu_address(0x05);

        ppu.write_to_data(0x66); //write to A

        ppu.write_to_ppu_address(0x2C);
        ppu.write_to_ppu_address(0x05);

        ppu.write_to_data(0x77); //write to b

        ppu.write_to_ppu_address(0x28);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load into buffer
        assert_equal!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_address(0x24);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load into buffer
        assert_equal!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = PPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_address(0x21);
        ppu.write_to_ppu_address(0x23);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_address(0x23);
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_equal!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_control(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_address(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_address(0x05);

        ppu.read_data(); //load into_buffer
        assert_equal!(ppu.read_data(), 0x66);
        // assert_equal!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_equal!(status >> 7, 1);
        assert_equal!(ppu.registers.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_to_oam_address(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_address(0x10);
        assert_equal!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_address(0x11);
        assert_equal!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = PPU::new_empty_rom();

        let mut data = [0x66; OAM_DATA_SIZE];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_address(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_address(0xf); //wrap around
        assert_equal!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_address(0x10);
        assert_equal!(ppu.read_oam_data(), 0x77);

        ppu.write_to_oam_address(0x11);
        assert_equal!(ppu.read_oam_data(), 0x66);
    }

    fn tick_one_scanline(ppu: &mut PPU) -> bool {
        ppu.tick(100);
        ppu.tick(241)
    }

    #[test]
    fn test_tick_cycles_less_than_341_scanline_should_not_change() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);

        // Case 1: Cycles is less than 341, scanline should not change
        let result = ppu.tick(100);
        assert_equal!(result, false);
        assert_equal!(ppu.cycles, 100);
        assert_equal!(ppu.scanline, 0);
        assert_equal!(ppu.nmi_interrupt, None);
    }

    #[test]
    fn test_tick_cycles_greater_than_341_scanline_less_than_241_scanline_increase_by_one() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);

        let result = ppu.tick(100);
        assert_equal!(result, false);
        assert_equal!(ppu.cycles, 100);
        assert_equal!(ppu.scanline, 0);
        assert_equal!(ppu.nmi_interrupt, None);

        // Case 2: Cycles is greater than or equal to 341 but scanline < 241
        // scanline should increase by 1
        let result = ppu.tick(241);
        assert_equal!(result, false);
        assert_equal!(ppu.cycles, 0);
        assert_equal!(ppu.scanline, 1);
        assert_equal!(ppu.nmi_interrupt, None);
    }

    #[test]
    fn test_tick_on_241_scanlines_assign_nmi_interrupt_and_return_true_for_new_frame() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);

        // Generate 240 scanlines
        for _ in 0..240 {
            tick_one_scanline(&mut ppu);
        }

        // Case 3: Cycles is greater than or equal to 341 and scanline is 241
        // vblank status should be set and nmi interrupt should be generated if configured to do so
        assert!(tick_one_scanline(&mut ppu));
        assert_equal!(ppu.cycles, 0);
        assert_equal!(ppu.scanline, 241);
        assert_equal!(ppu.nmi_interrupt, Some(1));
        assert!(ppu
            .registers
            .status
            .contains(StatusRegister::VBLANK_STARTED));
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));
    }

    #[test]
    fn test_tick_resets_nmi_after_262_scanlines() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);

        // Generate 261 scanlines
        for _ in 0..261 {
            tick_one_scanline(&mut ppu);
        }
        assert_equal!(ppu.nmi_interrupt, Some(1));

        // After 262 scanlines, remove NMI interrupt
        let result = tick_one_scanline(&mut ppu);
        assert_equal!(result, false);
        assert_equal!(ppu.cycles, 0);
        assert_equal!(ppu.scanline, 0);
        assert_equal!(ppu.nmi_interrupt, None);
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::VBLANK_STARTED));
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));
    }

    #[test]
    fn test_tick_checks_if_sprite_zero_is_hit_on_every_cycle() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);
        ppu.registers.mask.set(MaskRegister::SHOW_SPRITES, true);

        ppu.oam_data[0] = 10; // sprite_zero_hit scanline = 10
        ppu.oam_data[3] = 0; // sprite_zero_hit 0 <= cycle

        tick_one_scanline(&mut ppu);
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));

        ppu.oam_data[0] = 1; // sprite_zero_hit scanline = 1
        ppu.oam_data[3] = 0; // sprite_zero_hit 0 <= cycle

        tick_one_scanline(&mut ppu);
        assert!(ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));
    }

    #[test]
    fn test_tick_resets_sprite_zero_hit_during_vblank() {
        let mut ppu = PPU::new_empty_rom();
        ppu.registers
            .control
            .set(ControlRegister::GENERATE_NMI_AT_VBI, true);
        ppu.registers.mask.set(MaskRegister::SHOW_SPRITES, true);

        ppu.oam_data[0] = 10; // sprite_zero_hit scanline = 10
        ppu.oam_data[3] = 0; // sprite_zero_hit 0 <= cycle

        tick_one_scanline(&mut ppu);
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));

        ppu.oam_data[0] = 1; // sprite_zero_hit scanline = 1
        ppu.oam_data[3] = 0; // sprite_zero_hit 0 <= cycle

        tick_one_scanline(&mut ppu);
        assert!(ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));

        for _ in 1..240 {
            tick_one_scanline(&mut ppu);
        }

        assert_equal!(ppu.scanline, 241);
        assert!(!ppu
            .registers
            .status
            .contains(StatusRegister::SPRITE_ZERO_HIT));
    }

    #[test]
    fn test_read_data_palette() {
        let mut ppu = PPU::new_empty_rom();
        ppu.palette_table[0] = 0xFF;
        ppu.palette_table[31] = 0xAA;

        ppu.registers.address.update(0x3F);
        ppu.registers.address.update(0x00);
        assert_equal!(ppu.read_data(), 0xFF);

        ppu.registers.address.update(0x3F);
        ppu.registers.address.update(0x1F);
        assert_equal!(ppu.read_data(), 0xAA);
    }

    #[test]
    fn test_read_data_palette_mirroring() {
        let mut ppu = PPU::new_empty_rom();
        ppu.palette_table[0] = 0xFF;
        ppu.palette_table[31] = 0xAA;

        ppu.registers.address.update(0x3F);
        ppu.registers.address.update(0x10);
        assert_equal!(ppu.read_data(), 0xFF);

        ppu.registers.address.update(0x3F);
        ppu.registers.address.update(0x3F);
        assert_equal!(ppu.read_data(), 0xAA);
    }
}
