use crate::frame::Frame;
use crate::palette::background_palette;
use ppu::PPU;

pub mod frame;
mod palette;

/*
WARNING This is quite a drastic simplification that limits the types of games it will be
possible to play on the emulator.

More advanced games used a lot of tricks to enrich the gaming experience. For example,
changing scroll in the middle of the frame (split scroll) or changing palette colors.

This simplification wouldn't affect first-gen NES games much. The majority of NES games would
require more accuracy in PPU emulation, however.

- https://bugzmanov.github.io/nes_ebook/chapter_6_4.html
 */

pub fn render(ppu: &PPU, frame: &mut Frame) {
    let bank = ppu.registers.control.background_pattern_table_address();

    for i in 0..0x03C0 {
        let tile = ppu.vram[i] as u16;
        let tile_x = i % 32;
        let tile_y = i / 32;
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];
        let palette = background_palette(ppu, tile_x, tile_y);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALLETE[ppu.palette_table[0] as usize],
                    1 => palette::SYSTEM_PALLETE[palette[1] as usize],
                    2 => palette::SYSTEM_PALLETE[palette[2] as usize],
                    3 => palette::SYSTEM_PALLETE[palette[3] as usize],
                    _ => panic!("can't be"),
                };

                frame.set_pixel(tile_x * 8 + x, tile_y * 8 + y, rgb)
            }
        }
    }
}
