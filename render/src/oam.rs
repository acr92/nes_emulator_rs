use bitflags::bitflags;
use ppu::PPU;

bitflags! {
    /// # Controller Register (PPUCTRL) https://www.nesdev.org/wiki/PPU_registers
    ///
    ///  7  bit  0
    ///  ---- ----
    ///  VPHB SINN
    ///  |||| ||||
    ///  |||| ||++- Base nametable address
    ///  |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    ///  |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    ///  |||| |     (0: add 1, going across; 1: add 32, going down)
    ///  |||| +---- Sprite pattern table address for 8x8 sprites
    ///  ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    ///  |||+------ Background pattern table address (0: $0000; 1: $1000)
    ///  ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
    ///  |+-------- PPU master/slave select
    ///  |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    ///  +--------- Generate an NMI at the start of the
    ///             vertical blanking interval (0: off; 1: on)
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

pub(crate) struct OAM {
    pub tile_y: usize,
    pub tile_x: usize,
    pub tile_index: u16,
    attributes: OAMAttribute,
}

impl OAM {
    fn new(bytes: &[u8]) -> Self {
        OAM {
            tile_y: bytes[0] as usize,
            tile_index: bytes[1] as u16,
            attributes: OAMAttribute::from_bits_truncate(bytes[2]),
            tile_x: bytes[3] as usize,
        }
    }

    pub(crate) fn oam_iter(ppu: &PPU) -> impl Iterator<Item = OAM> + '_ {
        ppu.oam_data
            .as_slice()
            .chunks_exact(4)
            .rev()
            .map(|chunk| OAM::new(chunk))
    }

    pub(crate) fn palette_index(&self) -> u8 {
        self.attributes.bits & (OAMAttribute::PALETTE1.bits | OAMAttribute::PALETTE2.bits)
    }

    pub(crate) fn flip_horizontal(&self) -> bool {
        return self
            .attributes
            .contains(OAMAttribute::FLIP_SPRITE_HORIZONTALLY);
    }

    pub(crate) fn flip_vertical(&self) -> bool {
        return self
            .attributes
            .contains(OAMAttribute::FLIP_SPRITE_VERTICALLY);
    }
}
