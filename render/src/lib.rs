use crate::frame::Frame;
use crate::oam::Oam;
use crate::palette::{background_palette, sprite_palette};
use crate::rectangle::Rectangle;
use core::cartridge::Mirroring;
use core::ppu::{NAMETABLE_0, NAMETABLE_1, NAMETABLE_2, NAMETABLE_3};
use ppu::PPU;

mod debug;
pub mod frame;
mod oam;
mod palette;
mod rectangle;

/*
WARNING This is quite a drastic simplification that limits the types of games it will be
possible to play on the emulator.

More advanced games used a lot of tricks to enrich the gaming experience. For example,
changing scroll in the middle of the frame (split scroll) or changing palette colors.

This simplification wouldn't affect first-gen NES games much. The majority of NES games would
require more accuracy in PPU emulation, however.

- https://bugzmanov.github.io/nes_ebook/chapter_6_4.html
 */
const SPRITE_COLOR_INDEX_TRANSPARENT: u8 = 0;

pub fn render(ppu: &PPU, frame: &mut Frame) {
    let scroll_x = (ppu.registers.scroll.scroll_x) as usize;
    let scroll_y = (ppu.registers.scroll.scroll_y) as usize;

    let nametable_address = ppu.registers.control.nametable_address();
    let (main_nametable, second_nametable) = match (&ppu.mirroring, nametable_address) {
        (Mirroring::Vertical, NAMETABLE_0)
        | (Mirroring::Vertical, NAMETABLE_2)
        | (Mirroring::Horizontal, NAMETABLE_0)
        | (Mirroring::Horizontal, NAMETABLE_1) => (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800]),
        (Mirroring::Vertical, NAMETABLE_1)
        | (Mirroring::Vertical, NAMETABLE_3)
        | (Mirroring::Horizontal, NAMETABLE_2)
        | (Mirroring::Horizontal, NAMETABLE_3) => (&ppu.vram[0x400..0x800], &ppu.vram[0..0x400]),
        (_, _) => {
            panic!("Not supported mirroring type {:?}", ppu.mirroring);
        }
    };

    render_name_table(
        ppu,
        frame,
        main_nametable,
        Rectangle::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
    );

    if scroll_x > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rectangle::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
        );
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rectangle::new(0, 0, 256, scroll_y),
            0,
            (240 - scroll_y) as isize,
        );
    }

    if let Some((_, y)) = ppu.sprite_zero_hit {
        let scroll_x_before = (ppu.registers.scroll_before_sprite_zero.scroll_x) as usize;
        let scroll_y_before = (ppu.registers.scroll_before_sprite_zero.scroll_y) as usize;

        // This is a hack, we round the rendering to the nearest full name table tile.
        // Probably won't work for every game, but works for Super Mario Bros
        let y2 = (((y + 31) / 32) * 32) as usize;

        render_name_table(
            ppu,
            frame,
            &ppu.vram[0..0x400],
            Rectangle::new(scroll_x_before, scroll_y_before, 256, y2),
            -(scroll_x_before as isize),
            -(scroll_y_before as isize),
        );
    }

    for oam in Oam::oam_iter(ppu) {
        let sprite_palette = sprite_palette(ppu, oam.palette_index());

        let bank = ppu.registers.control.sprite_pattern_table_address();
        let tile_ram_position = (bank + oam.tile_index * 16) as usize;
        let tile = &ppu.chr_rom[tile_ram_position..=tile_ram_position + 15];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;

                let rgb = match value {
                    SPRITE_COLOR_INDEX_TRANSPARENT => continue,
                    1 => palette::SYSTEM_PALLETE[sprite_palette[1] as usize],
                    2 => palette::SYSTEM_PALLETE[sprite_palette[2] as usize],
                    3 => palette::SYSTEM_PALLETE[sprite_palette[3] as usize],
                    _ => panic!("can't happen"),
                };

                match (oam.flip_horizontal(), oam.flip_vertical()) {
                    (false, false) => frame.set_pixel(oam.tile_x + x, oam.tile_y + y, rgb),
                    (true, false) => frame.set_pixel(oam.tile_x + 7 - x, oam.tile_y + y, rgb),
                    (false, true) => frame.set_pixel(oam.tile_x + x, oam.tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(oam.tile_x + 7 - x, oam.tile_y + 7 - y, rgb),
                }
            }
        }
    }
}

fn render_name_table(
    ppu: &PPU,
    frame: &mut Frame,
    name_table: &[u8],
    viewport: Rectangle,
    shift_x: isize,
    shift_y: isize,
) {
    let bank = ppu.registers.control.background_pattern_table_address();

    let attribute_table = &name_table[0x3C0..0x400];

    for (i, &name) in name_table.iter().enumerate().take(0x3C0) {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_index = name as u16;
        let tile_ram_start = (bank + tile_index * 16) as usize;
        let tile = &ppu.chr_rom[tile_ram_start..=tile_ram_start + 15];
        let palette = background_palette(ppu, attribute_table, tile_column, tile_row);

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
                    _ => panic!("can't happen"),
                };

                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= viewport.x1
                    && pixel_x < viewport.x2
                    && pixel_y >= viewport.y1
                    && pixel_y < viewport.y2
                {
                    let shifted_x = (shift_x + pixel_x as isize) as usize;
                    let shifted_y = (shift_y + pixel_y as isize) as usize;
                    frame.set_pixel(shifted_x, shifted_y, rgb);
                }
            }
        }
    }
}
