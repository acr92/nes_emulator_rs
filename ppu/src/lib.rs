use crate::oam::Oam;
use crate::palette::SYSTEM_PALLETE;
use crate::register::{
    is_read_allowed, is_write_allowed, RegisterField, Registers, PPU_REGISTERS_MAP,
};
use crate::registers::control::ControlRegister;
use crate::registers::mask::MaskRegister;
use crate::registers::status::StatusRegister;
use core::cartridge::Mirroring;
use core::mem::Mem;

pub mod oam;
mod palette;
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

const FRAME_SIZE: usize = 256 * 240 * 3;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; PALETTE_TABLE_SIZE],
    pub vram: [u8; PPU_VRAM_SIZE],
    pub oam_data: [u8; OAM_DATA_SIZE],
    pub mirroring: Mirroring,
    pub registers: Registers,
    internal_data_buf: u8,

    pub scanline: i16,
    pub cycles: usize,
    pub nmi_interrupt: Option<u8>,

    pub frame: [u8; FRAME_SIZE],

    bg_next_tile_id: u8,
    bg_next_tile_attrib: u8,
    bg_next_tile_lsb: u8,
    bg_next_tile_msb: u8,
    bg_shifter_pattern_lo: u16,
    bg_shifter_pattern_hi: u16,
    bg_shifter_attrib_lo: u16,
    bg_shifter_attrib_hi: u16,
    address_latch: u8,
    fine_x: u8,

    sprite_scanline: Vec<Oam>,
    sprite_shifter_pattern_lo: [u8; 8],
    sprite_shifter_pattern_hi: [u8; 8],

    sprite_zero_hit_possible: bool,
    sprite_zero_being_rendered: bool,
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

            frame: [0x00; FRAME_SIZE],

            bg_next_tile_id: 0,
            bg_next_tile_attrib: 0,
            bg_next_tile_lsb: 0,
            bg_next_tile_msb: 0,
            bg_shifter_pattern_lo: 0,
            bg_shifter_pattern_hi: 0,
            bg_shifter_attrib_lo: 0,
            bg_shifter_attrib_hi: 0,
            address_latch: 0,
            fine_x: 0,

            sprite_scanline: vec![],
            sprite_shifter_pattern_lo: [0; 8],
            sprite_shifter_pattern_hi: [0; 8],
            sprite_zero_hit_possible: false,
            sprite_zero_being_rendered: false,
        }
    }

    fn increment_scroll_x(&mut self) {
        if self.registers.mask.show_sprites() || self.registers.mask.show_background() {
            if self.registers.vram_addr.get_coarse_x() == 31 {
                self.registers.vram_addr.set_coarse_x(0);
                self.registers
                    .vram_addr
                    .set_nametable_x(!self.registers.vram_addr.get_nametable_x());
            } else {
                self.registers
                    .vram_addr
                    .set_coarse_x(self.registers.vram_addr.get_coarse_x().wrapping_add(1));
            }
        }
    }

    fn increment_scroll_y(&mut self) {
        if self.registers.mask.show_sprites() || self.registers.mask.show_background() {
            if self.registers.vram_addr.get_fine_y() < 7 {
                self.registers
                    .vram_addr
                    .set_fine_y(self.registers.vram_addr.get_fine_y().wrapping_add(1))
            } else {
                self.registers.vram_addr.set_fine_y(0);

                if self.registers.vram_addr.get_coarse_y() == 29 {
                    self.registers.vram_addr.set_coarse_y(0);
                    self.registers
                        .vram_addr
                        .set_nametable_y(!self.registers.vram_addr.get_nametable_y())
                } else if self.registers.vram_addr.get_coarse_y() == 31 {
                    self.registers.vram_addr.set_coarse_y(0);
                } else {
                    self.registers
                        .vram_addr
                        .set_coarse_y(self.registers.vram_addr.get_coarse_y().wrapping_add(1));
                }
            }
        }
    }

    fn transfer_address_x(&mut self) {
        if self.registers.mask.show_sprites() || self.registers.mask.show_background() {
            self.registers
                .vram_addr
                .set_nametable_x(self.registers.tram_addr.get_nametable_x());
            self.registers
                .vram_addr
                .set_coarse_x(self.registers.tram_addr.get_coarse_x());
        }
    }

    fn transfer_address_y(&mut self) {
        if self.registers.mask.show_sprites() || self.registers.mask.show_background() {
            self.registers
                .vram_addr
                .set_fine_y(self.registers.tram_addr.get_fine_y());
            self.registers
                .vram_addr
                .set_nametable_y(self.registers.tram_addr.get_nametable_y());
            self.registers
                .vram_addr
                .set_coarse_y(self.registers.tram_addr.get_coarse_y());
        }
    }

    fn load_background_shifters(&mut self) {
        self.bg_shifter_pattern_lo =
            (self.bg_shifter_pattern_lo & 0xFF00) | self.bg_next_tile_lsb as u16;
        self.bg_shifter_pattern_hi =
            (self.bg_shifter_pattern_hi & 0xFF00) | self.bg_next_tile_msb as u16;

        self.bg_shifter_attrib_lo = (self.bg_shifter_attrib_lo & 0xFF00);
        self.bg_shifter_attrib_lo |= if self.bg_next_tile_attrib & 0b01 > 0 {
            0xFF
        } else {
            0x00
        };

        self.bg_shifter_attrib_hi = (self.bg_shifter_attrib_hi & 0xFF00);
        self.bg_shifter_attrib_hi |= if self.bg_next_tile_attrib & 0b10 > 0 {
            0xFF
        } else {
            0x00
        };
    }

    fn update_shifters(&mut self) {
        if self.registers.mask.show_background() {
            self.bg_shifter_pattern_lo <<= 1;
            self.bg_shifter_pattern_hi <<= 1;

            self.bg_shifter_attrib_lo <<= 1;
            self.bg_shifter_attrib_hi <<= 1;
        }

        if self.registers.mask.show_sprites() && self.cycles >= 1 && self.cycles < 258 {
            for (index, mut oam) in self.sprite_scanline.iter_mut().enumerate() {
                if oam.tile_x > 0 {
                    oam.tile_x -= 1;
                } else {
                    self.sprite_shifter_pattern_lo[index] <<= 1;
                    self.sprite_shifter_pattern_hi[index] <<= 1;
                }
            }
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut frame_complete = false;

        for _ in 0..cycles {
            if self.scanline >= -1 && self.scanline < 240 {
                if self.scanline == 0 && self.cycles == 0 {
                    // "Odd Frame" cycle skip
                    self.cycles = 1;
                }

                if self.scanline == -1 && self.cycles == 1 {
                    self.registers.status.reset_vblank_status();
                    self.registers.status.set_sprite_overflow(false);
                    self.registers.status.set_sprite_zero_hit(false);

                    // clear shifters
                    self.sprite_shifter_pattern_lo = [0; 8];
                    self.sprite_shifter_pattern_hi = [0; 8];
                }

                if (self.cycles >= 2 && self.cycles < 258)
                    || (self.cycles >= 321 && self.cycles < 338)
                {
                    self.update_shifters();

                    let cycle_group = (self.cycles - 1) % 8;
                    match cycle_group {
                        0 => {
                            self.load_background_shifters();

                            self.bg_next_tile_id = self
                                .ppu_read(0x2000 | (self.registers.vram_addr.get_bits() & 0x0FFF));
                        }
                        2 => {
                            let mut addr = 0x23C0;
                            addr |= self.registers.vram_addr.get_nametable_y() << 11;
                            addr |= self.registers.vram_addr.get_nametable_x() << 10;
                            addr |= (self.registers.vram_addr.get_coarse_y() >> 2) << 3;
                            addr |= (self.registers.vram_addr.get_coarse_x() >> 2);

                            self.bg_next_tile_attrib = self.ppu_read(addr);

                            if self.registers.vram_addr.get_coarse_y() & 0x02 > 0 {
                                self.bg_next_tile_attrib >>= 4;
                            }
                            if self.registers.vram_addr.get_coarse_x() & 0x02 > 0 {
                                self.bg_next_tile_attrib >>= 2;
                            }
                            self.bg_next_tile_attrib &= 0x03;
                        }
                        4 => {
                            let mut addr = ((if self
                                .registers
                                .control
                                .contains(ControlRegister::BACKGROUND_PATTERN_ADDR)
                            {
                                1
                            } else {
                                0
                            }) as u16)
                                << 12;
                            addr += ((self.bg_next_tile_id as u16) << 4) as u16;
                            addr += (self.registers.vram_addr.get_fine_y());
                            self.bg_next_tile_lsb = self.ppu_read(addr);
                        }
                        6 => {
                            let mut addr = if self
                                .registers
                                .control
                                .contains(ControlRegister::BACKGROUND_PATTERN_ADDR)
                            {
                                1
                            } else {
                                0
                            } << 12;
                            addr += ((self.bg_next_tile_id as u16) << 4) as u16;
                            addr += (self.registers.vram_addr.get_fine_y() + 8);
                            self.bg_next_tile_msb = self.ppu_read(addr);
                        }
                        7 => {
                            self.increment_scroll_x();
                        }
                        _ => {}
                    }
                }

                if self.cycles == 256 {
                    self.increment_scroll_y()
                }

                if self.cycles == 257 {
                    self.load_background_shifters();
                    self.transfer_address_x();
                }

                if self.cycles == 338 || self.cycles == 340 {
                    self.bg_next_tile_id =
                        self.ppu_read(0x2000 | (self.registers.vram_addr.get_bits() & 0x0FFF));
                }

                if self.scanline == -1 && self.cycles >= 280 && self.cycles < 305 {
                    // End of vertical blank period so reset the Y address ready for rendering
                    self.transfer_address_y();
                }

                if self.cycles == 257 && self.scanline >= 0 {
                    self.sprite_scanline = vec![];

                    let mut sprite_count = 0;
                    for oam in Oam::oam_iter(&self.oam_data) {
                        if sprite_count >= 9 {
                            break;
                        }

                        let diff = self.scanline - (oam.tile_y as i16);

                        if diff >= 0 && diff < self.registers.control.sprite_size() as i16 {
                            if self.sprite_scanline.len() < 8 {
                                self.sprite_scanline.push(oam.clone());
                            }
                        }
                    }

                    self.registers.status.set_sprite_overflow(sprite_count > 8);
                }

                if self.cycles == 340 {
                    for (index, oam) in self.sprite_scanline.iter().enumerate() {
                        // god what a mess..........
                        let addr_lo = if !self
                            .registers
                            .control
                            .contains(ControlRegister::SPRITE_SIZE)
                        {
                            // 8x8 mode
                            if !oam.flip_vertical() {
                                // normal
                                self.registers.control.sprite_pattern_table_address()
                                    | (oam.tile_index << 4) // which cell
                                    | (self.scanline as u16 - oam.tile_y as u16)
                            /* which row in cell */
                            } else {
                                // flipped vertically
                                self.registers.control.sprite_pattern_table_address()
                                    | (oam.tile_index << 4)
                                    | (7 - self.scanline as u16 - oam.tile_y as u16)
                            }
                        } else {
                            // 8x16 mode
                            if !oam.flip_vertical() {
                                if self.scanline as usize - oam.tile_y < 8 {
                                    self.registers.control.sprite_pattern_table_address()
                                        | (oam.tile_index << 4)
                                        | ((self.scanline as u16 - oam.tile_y as u16) & 0x07)
                                } else {
                                    self.registers.control.sprite_pattern_table_address()
                                        | (((oam.tile_index & 0xFE) + 1) << 4)
                                        | ((self.scanline as u16 - oam.tile_y as u16) & 0x07)
                                }
                            } else {
                                if self.scanline as usize - oam.tile_y < 8 {
                                    self.registers.control.sprite_pattern_table_address()
                                        | (((oam.tile_index & 0xFE) + 1) << 4)
                                        | (7 - (self.scanline as u16 - oam.tile_y as u16) & 0x07)
                                } else {
                                    self.registers.control.sprite_pattern_table_address()
                                        | (oam.tile_index << 4)
                                        | (7 - (self.scanline as u16 - oam.tile_y as u16) & 0x07)
                                }
                            }
                        };

                        let addr_hi = addr_lo + 8;

                        let mut bits_lo = self.ppu_read(addr_lo);
                        let mut bits_hi = self.ppu_read(addr_hi);

                        if oam.flip_horizontal() {
                            fn flipbyte(input: u8) -> u8 {
                                let mut b = (input & 0xF0) >> 4 | (input & 0x0F) << 4;
                                b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
                                b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
                                b
                            }

                            bits_lo = flipbyte(bits_lo);
                            bits_hi = flipbyte(bits_hi);
                        }

                        self.sprite_shifter_pattern_lo[index] = bits_lo;
                        self.sprite_shifter_pattern_hi[index] = bits_hi;
                    }
                }
            }

            if self.scanline == 240 {
                // Post render scanline - do nothing
            }

            if self.scanline >= 241 && self.scanline < 261 {
                if self.scanline == 241 && self.cycles == 1 {
                    self.registers.status.set_vblank_status(true);

                    if self.registers.control.generate_vblank_nmi() {
                        self.nmi_interrupt = Some(1);
                        frame_complete = true
                    }
                }
            }

            let mut bg_pixel: u8 = 0x00;
            let mut bg_palette: u8 = 0x00;

            if self.registers.mask.show_background() {
                let bit_mux = 0x8000 >> self.fine_x;

                let p0_pixel: u8 = (self.bg_shifter_pattern_lo & bit_mux > 0) as u8;
                let p1_pixel: u8 = (self.bg_shifter_pattern_hi & bit_mux > 0) as u8;

                bg_pixel = (p1_pixel << 1) | p0_pixel;

                let bg_pal0: u8 = (self.bg_shifter_attrib_lo & bit_mux > 0) as u8;
                let bg_pal1: u8 = (self.bg_shifter_attrib_hi & bit_mux > 0) as u8;

                bg_palette = (bg_pal1 << 1) | bg_pal0;
            }

            let mut fg_pixel = 0u8;
            let mut fg_palette = 0u8;
            let mut fg_priority = false;

            if self.registers.mask.show_sprites() {
                for (index, oam) in self.sprite_scanline.iter().enumerate() {
                    if oam.tile_x == 0 {
                        let fg_pixel_lo = self.sprite_shifter_pattern_lo[index] & 0x80 >> 7;
                        let fg_pixel_hi = self.sprite_shifter_pattern_hi[index] & 0x80 >> 7;
                        fg_pixel = (fg_pixel_hi << 1) | fg_pixel_lo;

                        fg_palette = oam.palette_index() + 0x04;
                        fg_priority = oam.priority_in_front_of_background();

                        // non transparent pixel
                        if fg_pixel != 0 {
                            break;
                        }
                    }
                }
            }

            let (pixel, palette) = if bg_pixel == 0 && fg_pixel == 0 {
                (0x00, 0x00)
            } else if bg_pixel == 0 && fg_pixel > 0 {
                (fg_pixel, fg_palette)
            } else if bg_pixel > 0 && fg_pixel == 0 {
                (bg_pixel, bg_palette)
            } else {
                if fg_priority {
                    (fg_pixel, fg_palette)
                } else {
                    (bg_pixel, bg_palette)
                }
            };

            let rgb = self.get_color_from_palette_ram(pixel, palette);
            self.set_pixel(self.cycles.wrapping_sub(1), self.scanline as usize, rgb);

            // Advance renderer
            self.cycles += 1;
            if self.cycles >= 341 {
                self.cycles = 0;
                self.scanline += 1;
                if self.scanline >= 261 {
                    self.frame.fill(0x00);
                    self.scanline = -1;
                    frame_complete = true;
                }
            }
        }

        frame_complete
    }

    fn get_color_from_palette_ram(&mut self, mut pixel: u8, mut palette: u8) -> (u8, u8, u8) {
        let index = self.ppu_read(0x3F00 + (palette << 2) as u16 + pixel as u16) & 0x3F;
        SYSTEM_PALLETE[index as usize]
    }

    fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        if x >= 256 {
            return;
        }

        if y >= 240 {
            return;
        }

        self.frame[y * (256 * 3) + (x * 3) + 0] = rgb.0;
        self.frame[y * (256 * 3) + (x * 3) + 1] = rgb.1;
        self.frame[y * (256 * 3) + (x * 3) + 2] = rgb.2;
    }

    fn is_sprite_zero_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.registers.mask.show_sprites()
    }

    fn increment_vram_addr(&mut self) {
        let value = self.registers.vram_addr.get_bits()
            + (self.registers.control.vram_address_increment() as u16);
        self.registers.vram_addr.set_bits(value)
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.registers.vram_addr.get_bits();

        let mut result = self.internal_data_buf;
        self.internal_data_buf = self.ppu_read(addr);
        if addr >= PALETTE_RAM_START {
            result = self.internal_data_buf;
        }

        self.increment_vram_addr();

        result
    }

    fn write_to_data(&mut self, value: u8) {
        self.ppu_write(self.registers.vram_addr.get_bits(), value);
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
        if self.address_latch == 0 {
            self.registers.tram_addr.set_bits(
                (((value & 0x03F) as u16) << 8) | (self.registers.tram_addr.get_bits() & 0x00FF),
            );
            self.address_latch = 1;
        } else {
            self.registers
                .tram_addr
                .set_bits((self.registers.tram_addr.get_bits() & 0xFF00) | (value as u16));
            self.registers
                .vram_addr
                .set_bits(self.registers.tram_addr.get_bits());
            self.address_latch = 0;
        }
    }

    fn write_to_control(&mut self, value: u8) {
        self.registers.control.update(value);
        self.registers.tram_addr.set_nametable_x(
            if self.registers.control.contains(ControlRegister::NAMETABLE1) {
                1
            } else {
                0
            },
        );
        self.registers.tram_addr.set_nametable_y(
            if self.registers.control.contains(ControlRegister::NAMETABLE2) {
                1
            } else {
                0
            },
        );
    }

    fn read_status(&mut self) -> u8 {
        let data = self.registers.status.snapshot();
        self.registers.status.reset_vblank_status();
        self.address_latch = 0;
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

    fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.chr_rom[addr as usize],
            0x2000..=0x3EFF => self.vram[self.mirror_vram_addr(addr) as usize],
            0x3F00..=0x3FFF => {
                let mut addr = addr & 0x001F;
                if addr == 0x0010 {
                    addr = 0x0000;
                }
                if addr == 0x0014 {
                    addr = 0x0004;
                }
                if addr == 0x0018 {
                    addr = 0x0008;
                }
                if addr == 0x001C {
                    addr = 0x000C;
                }

                let palette_mask = if self.registers.mask.is_grayscale() {
                    0x30
                } else {
                    0x3F
                };
                self.palette_table[addr as usize] & palette_mask
            }
            _ => panic!("Unknown address {:04X}", addr),
        }
    }

    fn ppu_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.chr_rom[addr as usize] = value,
            0x2000..=0x3EFF => self.vram[self.mirror_vram_addr(addr) as usize] = value,
            0x3F00..=0x3FFF => {
                let mut addr = addr & 0x001F;
                if addr == 0x0010 {
                    addr = 0x0000;
                }
                if addr == 0x0014 {
                    addr = 0x0004;
                }
                if addr == 0x0018 {
                    addr = 0x0008;
                }
                if addr == 0x001C {
                    addr = 0x000C;
                }

                self.palette_table[addr as usize] = value
            }
            _ => panic!("Unknown address {:04X}", addr),
        }
    }
}

impl Mem for PPU {
    fn mem_read(&mut self, addr: u16) -> u8 {
        let register_result = PPU_REGISTERS_MAP.get(&addr);

        if let Some(register) = register_result {
            if !is_read_allowed(register) {
                println!("Tried to read from write-only {:#?}", register);
                return 0;
            }

            return match register.field {
                RegisterField::Status => {
                    (self.read_status() & 0xE0) | (self.internal_data_buf & 0x1F)
                }
                RegisterField::OAMData => self.read_oam_data(),
                RegisterField::Data => self.read_data(),
                _ => panic!("Unexpected read on {:#?}", register),
            };
        }

        0x00
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        let register_result = PPU_REGISTERS_MAP.get(&addr);

        if let Some(register) = register_result {
            if !is_write_allowed(register) {
                panic!("Tried to write to readonly {:#?}", register);
            }

            match register.field {
                RegisterField::Control => self.write_to_control(value),
                RegisterField::Mask => self.registers.mask.update(value),
                RegisterField::OAMAddress => self.write_to_oam_address(value),
                RegisterField::OAMData => self.write_to_oam_data(value),
                RegisterField::Scroll => {
                    if self.address_latch == 0 {
                        self.fine_x = value & 0x07;
                        self.registers.tram_addr.set_coarse_x((value >> 3) as u16);
                        self.address_latch = 1;
                    } else {
                        self.registers.tram_addr.set_fine_y((value & 0x07) as u16);
                        self.registers.tram_addr.set_coarse_y((value >> 3) as u16);
                        self.address_latch = 0;
                    }
                }
                RegisterField::Address => self.write_to_ppu_address(value),
                RegisterField::Data => self.write_to_data(value),
                _ => panic!("Unexpected write on {:#?}", register),
            }

            return;
        }
    }
}

#[cfg(NEVER)]
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

    #[test]
    fn test_status_read_contains_bits_from_internal_data_buffer() {
        let mut ppu = PPU::new_empty_rom();

        // Read status
        assert_equal!(ppu.mem_read(0x2002), 0);

        // Write to 0x20AA = 0xFA
        ppu.mem_write(0x2006, 0x20);
        ppu.mem_write(0x2006, 0xAA);
        ppu.mem_write(0x2007, 0xFA);

        // First read will return internal_data_buf (0x00), but assign 0xFA to internal_data_buf
        ppu.mem_write(0x2006, 0x20);
        ppu.mem_write(0x2006, 0xAA);
        assert_equal!(ppu.mem_read(0x2007), 0x00);

        // Now the lower status bits should be affected
        assert_equal!(ppu.mem_read(0x2002), 0x1A);
    }
}
