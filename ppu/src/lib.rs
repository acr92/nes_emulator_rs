use lazy_static::lazy_static;
use core::mem::Mem;
use std::collections::HashMap;

pub struct PPU {
    registers: [u8; 8],
}

#[derive(Clone, Copy)]
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

enum RegisterAccess {
    ReadWrite,
    ReadOnly,
    WriteOnly,
}

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


impl PPU {
    pub fn new() -> Self {
        PPU {
            registers: [0x00; 8],
        }
    }
}

impl PPU {}

impl Mem for PPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.registers[(addr - 0x2000) as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.registers[(addr - 0x2000) as usize] = value
    }
}

#[cfg(test)]
mod test {

}