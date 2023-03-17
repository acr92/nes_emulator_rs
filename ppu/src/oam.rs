use crate::PPU;
use bitflags::bitflags;

bitflags! {
    /// 76543210
    /// ||||||||
    /// ||||||++- Palette (4 to 7) of sprite
    /// |||+++--- Unimplemented (read 0)
    /// ||+------ Priority (0: in front of background; 1: behind background)
    /// |+------- Flip sprite horizontally
    /// +-------- Flip sprite vertically
    ///
    struct OAMAttribute: u8 {
        const PALETTE1                  = 0b00000001;
        const PALETTE2                  = 0b00000010;
        const UNUSED1                   = 0b00000100;
        const UNUSED2                   = 0b00001000;
        const UNUSED3                   = 0b00010000;
        const PRIORITY_FG_OR_BG         = 0b00100000;
        const FLIP_SPRITE_HORIZONTALLY  = 0b01000000;
        const FLIP_SPRITE_VERTICALLY    = 0b10000000;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Oam {
    pub tile_y: usize,
    pub tile_x: usize,
    pub tile_index: u16,
    attributes: OAMAttribute,
}

impl Oam {
    fn new(bytes: &[u8]) -> Self {
        Oam {
            tile_y: bytes[0] as usize,
            tile_index: bytes[1] as u16,
            attributes: OAMAttribute::from_bits_truncate(bytes[2]),
            tile_x: bytes[3] as usize,
        }
    }

    pub fn oam_iter(oam_data: &[u8]) -> impl Iterator<Item = Oam> + '_ {
        oam_data.chunks_exact(4).map(Oam::new)
    }

    pub fn palette_index(&self) -> u8 {
        self.attributes.bits & (OAMAttribute::PALETTE1.bits | OAMAttribute::PALETTE2.bits)
    }

    pub fn flip_horizontal(&self) -> bool {
        self.attributes
            .contains(OAMAttribute::FLIP_SPRITE_HORIZONTALLY)
    }

    pub fn flip_vertical(&self) -> bool {
        self.attributes
            .contains(OAMAttribute::FLIP_SPRITE_VERTICALLY)
    }

    pub fn priority_in_front_of_background(&self) -> bool {
        self.attributes.contains(OAMAttribute::PRIORITY_FG_OR_BG) == false
    }
}
