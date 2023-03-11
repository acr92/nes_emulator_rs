use crate::palette;

pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;
    pub const RGB_SIZE: usize = 3;

    pub fn new() -> Self {
        Frame {
            data: vec![0; (Frame::WIDTH) * (Frame::HEIGHT) * Frame::RGB_SIZE],
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * Frame::RGB_SIZE * Frame::WIDTH + x * Frame::RGB_SIZE;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }

    pub fn show_tiles(chr_rom: &[u8], bank: usize) -> Frame {
        assert!(bank <= 1);
        let bank = bank * ppu::CHR_ROM_BANK_SIZE;

        let mut frame = Frame::new();
        for tile_n in 0..Frame::WIDTH {
            let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let value = (1 & upper) << 1 | (1 & lower);
                    upper >>= 1;
                    lower >>= 1;

                    let rgb = match value {
                        0 => palette::SYSTEM_PALLETE[0x01],
                        1 => palette::SYSTEM_PALLETE[0x23],
                        2 => palette::SYSTEM_PALLETE[0x27],
                        3 => palette::SYSTEM_PALLETE[0x30],
                        _ => panic!("impossible"),
                    };
                    frame.set_pixel(((tile_n * 10) % 200) + x, ((tile_n / 20) * 10) + y, rgb);
                }
            }
        }

        frame
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame::new()
    }
}
