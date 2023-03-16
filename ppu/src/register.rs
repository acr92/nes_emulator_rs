use crate::registers::address::AddressRegister;
use crate::registers::control::ControlRegister;
use crate::registers::mask::MaskRegister;
use crate::registers::scroll::ScrollRegister;
use crate::registers::status::StatusRegister;
use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::registers::loopy::LoopyRegister;

lazy_static! {
    pub(crate) static ref PPU_REGISTERS: Vec<Register> = vec![
        Register::new(0x2000, RegisterField::Control, RegisterAccess::WriteOnly),
        Register::new(0x2001, RegisterField::Mask, RegisterAccess::WriteOnly),
        Register::new(0x2002, RegisterField::Status, RegisterAccess::ReadOnly),
        Register::new(0x2003, RegisterField::OAMAddress, RegisterAccess::WriteOnly),
        Register::new(0x2004, RegisterField::OAMData, RegisterAccess::ReadWrite),
        Register::new(0x2005, RegisterField::Scroll, RegisterAccess::WriteOnly),
        Register::new(0x2006, RegisterField::Address, RegisterAccess::WriteOnly),
        Register::new(0x2007, RegisterField::Data, RegisterAccess::ReadWrite),
    ];
    pub(crate) static ref PPU_REGISTERS_MAP: HashMap<u16, &'static Register> = {
        let mut map = HashMap::new();
        for register in &*PPU_REGISTERS {
            map.insert(register.address, register);
        }
        map
    };
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterField {
    Control,
    Mask,
    Status,
    OAMAddress,
    OAMData,
    Scroll,
    Address,
    Data,
}

#[derive(Debug, Copy, Clone)]
enum RegisterAccess {
    ReadWrite,
    ReadOnly,
    WriteOnly,
}

pub(crate) fn is_read_allowed(register: &Register) -> bool {
    matches!(register.access, RegisterAccess::ReadWrite)
        || matches!(register.access, RegisterAccess::ReadOnly)
}

pub(crate) fn is_write_allowed(register: &Register) -> bool {
    matches!(register.access, RegisterAccess::ReadWrite)
        || matches!(register.access, RegisterAccess::WriteOnly)
}

#[derive(Debug)]
pub(crate) struct Register {
    pub address: u16,
    pub field: RegisterField,
    access: RegisterAccess,
}

impl Register {
    fn new(address: u16, field: RegisterField, access: RegisterAccess) -> Self {
        Register {
            address,
            field,
            access,
        }
    }
}

pub struct Registers {
    pub control: ControlRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub oam_address: u8,

    pub vram_addr: LoopyRegister,
    pub tram_addr: LoopyRegister,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            control: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            oam_address: 0,

            vram_addr: LoopyRegister::new(),
            tram_addr: LoopyRegister::new(),
        }
    }
}

impl Default for Registers {
    fn default() -> Self {
        Registers::new()
    }
}
