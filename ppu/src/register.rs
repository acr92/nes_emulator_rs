use bitflags::bitflags;
use core::mem::Mem;
use std::collections::HashMap;
use lazy_static::lazy_static;


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
    pub(crate) struct ControllerFlags: u8 {
        const NAMETABLE1                = 0b00000001;
        const NAMETABLE2                = 0b00000010;
        const VRAM_ADD_INCREMENT        = 0b00000100;
        const SPRITE_PATTERN_ADDR       = 0b00001000;
        const BACKGROUND_PATTERN_ADDR   = 0b00010000;
        const SPRITE_SIZE               = 0b00100000;
        const PPU_MASTER_SLAVE_SELECT   = 0b01000000;
        const GENERATE_NMI_AT_VBI       = 0b10000000;
    }

    /// # Mask Register (PPUMASK) https://www.nesdev.org/wiki/PPU_registers
    ///
    /// 7  bit  0
    /// ---- ----
    /// BGRs bMmG
    /// |||| ||||
    /// |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
    /// |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    /// |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    /// |||| +---- 1: Show background
    /// |||+------ 1: Show sprites
    /// ||+------- Emphasize red (green on PAL/Dendy)
    /// |+-------- Emphasize green (red on PAL/Dendy)
    /// +--------- Emphasize blue
    ///
    pub(crate) struct MaskFlags: u8 {
        const GRAYSCALE                 = 0b00000001;
        const SHOW_BACKGROUND_LEFTMOST  = 0b00000010;
        const SHOW_SPRITES_LEFTMOST     = 0b00000100;
        const SHOW_BACKGROUND           = 0b00001000;
        const SHOW_SPRITES              = 0b00010000;
        const EMPHASIZE_RED             = 0b00100000;
        const EMPHASIZE_GREEN           = 0b01000000;
        const EMPHASIZE_BLUE            = 0b10000000;
    }

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
    pub(crate) struct StatusFlags: u8 {
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


#[derive(Debug, Clone, Copy)]
enum RegisterField {
    Controller,
    Mask,
    Status,
    OAMAddress,
    OAMData,
    Scroll,
    Address,
    Data,
    OAMDMA,
}

#[derive(Debug, Copy, Clone)]
enum RegisterAccess {
    ReadWrite,
    ReadOnly,
    WriteOnly,
}

fn is_read_allowed(access: RegisterAccess) -> bool {
    return matches!(access, RegisterAccess::ReadWrite) ||
        matches!(access, RegisterAccess::ReadOnly)
}

fn is_write_allowed(access: RegisterAccess) -> bool {
    return matches!(access, RegisterAccess::ReadWrite) ||
        matches!(access, RegisterAccess::WriteOnly)
}

#[derive(Debug)]
struct Register {
    absolute_address: u16,
    field: RegisterField,
    access: RegisterAccess,
}

impl Register {
    fn new(absolute_address: u16, field: RegisterField, access: RegisterAccess) -> Self {
        Register {
            absolute_address,
            field,
            access,
        }
    }
}

lazy_static! {
    static ref PPU_REGISTERS: Vec<Register> = vec![
        Register::new(0x2000, RegisterField::Controller, RegisterAccess::WriteOnly),
        Register::new(0x2001, RegisterField::Mask, RegisterAccess::WriteOnly),
        Register::new(0x2002, RegisterField::Status, RegisterAccess::ReadOnly),
        Register::new(0x2003, RegisterField::OAMAddress, RegisterAccess::WriteOnly),
        Register::new(0x2004, RegisterField::OAMData, RegisterAccess::ReadWrite),
        Register::new(0x2005, RegisterField::Scroll, RegisterAccess::WriteOnly),
        Register::new(0x2006, RegisterField::Address, RegisterAccess::WriteOnly),
        Register::new(0x2007, RegisterField::Data, RegisterAccess::ReadWrite),
        Register::new(0x4014, RegisterField::OAMDMA, RegisterAccess::WriteOnly),
    ];

    static ref PPU_REGISTERS_MAP: HashMap<u16, &'static Register> = {
        let mut map = HashMap::new();
        for register in &*PPU_REGISTERS {
            map.insert(register.absolute_address, register);
        }
        map
    };
}

pub struct Registers {
    controller: ControllerFlags,
    mask: MaskFlags,
    status: StatusFlags,
    oam_address: u8,
    oam_data: u8,
    scroll: u8,
    address: u8,
    data: u8,
    oam_dma: u8,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            controller: ControllerFlags::from_bits_truncate(0),
            mask: MaskFlags::from_bits_truncate(0),
            status: StatusFlags::from_bits_truncate(0),
            oam_address: 0,
            oam_data: 0,
            scroll: 0,
            address: 0,
            data: 0,
            oam_dma: 0,
        }
    }
}

impl Mem for Registers {
    fn mem_read(&self, addr: u16) -> u8 {
        let register = PPU_REGISTERS_MAP.get(&addr)
            .expect(&format!("Unexpected addr {:04X}", addr));

        if !is_read_allowed(register.access) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Status => self.status.bits,
            RegisterField::OAMData => self.oam_data,
            RegisterField::Data => self.data,
            _ => panic!("Unexpected read on {:#?}", register)
        }
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        let register = PPU_REGISTERS_MAP.get(&addr)
            .expect(&format!("Unexpected addr {:04X}", addr));

        if !is_write_allowed(register.access) {
            panic!("Tried to write to readonly {:#?}", register);
        }

        match register.field {
            RegisterField::Controller => self.controller.bits = value,
            RegisterField::Mask => self.mask.bits = value,
            RegisterField::OAMAddress => self.oam_address = value,
            RegisterField::OAMData => self.oam_data = value,
            RegisterField::Scroll => self.scroll = value,
            RegisterField::Address => self.address = value,
            RegisterField::Data => self.data = value,
            RegisterField::OAMDMA => self.oam_dma = value,
            _ => panic!("Unexpected write on {:#?}", register)
        }
    }
}