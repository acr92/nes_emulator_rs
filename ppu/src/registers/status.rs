use bitflags::bitflags;

bitflags! {
    /// # Status Register (PPUSTATUS) https://www.nesdev.org/wiki/PPU_registers
    ///
    /// 7  bit  0
    /// ---- ----
    /// VSO. ....
    /// |||| ||||
    /// |||+-++++- PPU open bus. Returns stale PPU bus contents.
    /// ||+------- Sprite overflow. The intent was for this flag to be set
    /// ||         whenever more than eight sprites appear on a scanline, but a
    /// ||         hardware bug causes the actual behavior to be more complicated
    /// ||         and generate false positives as well as false negatives; see
    /// ||         PPU sprite evaluation. This flag is set during sprite
    /// ||         evaluation and cleared at dot 1 (the second dot) of the
    /// ||         pre-render line.
    /// |+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
    /// |          a nonzero background pixel; cleared at dot 1 of the pre-render
    /// |          line.  Used for raster timing.
    /// +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
    ///            Set at dot 1 of line 241 (the line *after* the post-render
    ///            line); cleared after reading $2002 and at dot 1 of the
    ///            pre-render line.
    ///
    pub(crate) struct StatusRegister: u8 {
        const PPU_OPEN_BUS_0            = 0b00000001;
        const PPU_OPEN_BUS_1            = 0b00000010;
        const PPU_OPEN_BUS_2            = 0b00000100;
        const PPU_OPEN_BUS_3            = 0b00001000;
        const PPU_OPEN_BUS_4            = 0b00010000;
        const SPRITE_OVERFLOW           = 0b00100000;
        const SPRITE_ZERO_HIT           = 0b01000000;
        const VERTICAL_BLANK_STARTED    = 0b10000000;
    }
}

impl StatusRegister {
    pub fn new() -> Self {
        StatusRegister::from_bits_truncate(0)
    }
}