use bitflags::bitflags;

bitflags! {
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
    ///
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    ///
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const BREAK2            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

pub const STACK: u16 = 0x0100;
pub const STACK_RESET: u8 = 0xFD;

#[derive(Clone, Copy)]
pub enum RegisterField {
    A,
    X,
    Y,
    SP,
    STATUS,
}

pub struct Register {
    a: u8,
    x: u8,
    y: u8,
    pub pc: u16,
    pub sp: u8,
    pub status: CpuFlags,
}

impl Register {
    pub fn new() -> Self {
        Register {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: STACK_RESET,
            status: CpuFlags::from_bits_truncate(0b100100),
        }
    }

    pub fn read(&self, field: RegisterField) -> u8 {
        match field {
            RegisterField::A => self.a,
            RegisterField::X => self.x,
            RegisterField::Y => self.y,
            RegisterField::SP => self.sp,
            RegisterField::STATUS => self.status.bits,
        }
    }

    pub fn write(&mut self, field: RegisterField, value: u8) {
        match field {
            RegisterField::A => self.a = value,
            RegisterField::X => self.x = value,
            RegisterField::Y => self.y = value,
            RegisterField::SP => self.sp = value,
            RegisterField::STATUS => self.status.bits = value,
        }

        match field {
            RegisterField::SP => {}
            _ => self.update_zero_and_negative_flags(value),
        }
    }

    pub fn update_zero_and_negative_flags(&mut self, result: u8) {
        // set Zero Flag if A = 0
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        // set Negative flag if bit 7 of A is set
        if result & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }
}
