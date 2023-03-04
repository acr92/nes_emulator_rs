use bitflags::bitflags;
use crate::opcodes;
use crate::opcodes::{AddressingMode, Instruction};

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

pub enum RegisterField {
    A,
    X,
    Y,
    SP
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;

pub struct Register {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub sp: u8,
    pub status: CpuFlags,
}

impl Register {
    fn read(&self, field: &RegisterField) -> u8 {
        match field {
            RegisterField::A => self.a,
            RegisterField::X => self.x,
            RegisterField::Y => self.y,
            RegisterField::SP => self.sp,
        }
    }

    fn write(&mut self, field: &RegisterField, value: u8) {
        match field {
            RegisterField::A => { self.a = value }
            RegisterField::X => { self.x = value }
            RegisterField::Y => { self.y = value }
            RegisterField::SP => { self.sp = value }
        }
    }
}

pub struct CPU {
    pub register: Register,
    memory: [u8; 0xFFFF],
}

trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, value: u8);

    fn mem_read_u16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr.wrapping_add(1)) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, addr: u16, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr.wrapping_add(1), hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register: Register {
                a: 0,
                x: 0,
                y: 0,
                pc: 0,
                sp: STACK_RESET,
                status: CpuFlags::from_bits_truncate(0b100100),
            },
            memory: [0; 0xFFFF],
        }
    }

    pub fn reset(&mut self) {
        self.register = Register {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: STACK_RESET,
            status: CpuFlags::from_bits_truncate(0b100100),
        };

        self.register.pc = self.mem_read_u16(0xFFFC);
    }

    pub fn load_and_run(&mut self, program: &[u8]) {
        self.load_program_into_memory(program);
        self.reset();
        self.run()
    }
    fn load_program_into_memory(&mut self, program: &[u8]) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(program);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    fn run(&mut self) {
        let ref opcodes = *opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.register.pc);
            self.register.pc += 1;
            let program_counter_state = self.register.pc;

            let opcode = opcodes.get(&code).expect(&format!("Opcode {:x} is not recognized", code));

            match opcode.instruction {
                Instruction::BRK => { return; }
                Instruction::NOP => { }

                Instruction::TAX => { self.transfer(RegisterField::A, RegisterField::X) }
                Instruction::TXA => { self.transfer(RegisterField::X, RegisterField::A) }
                Instruction::TYA => { self.transfer(RegisterField::Y, RegisterField::A) }

                Instruction::INX => { self.inx() }
                Instruction::LDA => { self.load(RegisterField::A, &opcode.mode) }
                Instruction::LDX => { self.load(RegisterField::X, &opcode.mode) }
                Instruction::LDY => { self.load(RegisterField::Y, &opcode.mode) }
                Instruction::STA => { self.sta(&opcode.mode) }
                _ => { todo!("Unknown opcode 0x{:X} {:#?}", code, opcode.instruction) }
            }

            if program_counter_state == self.register.pc {
                self.register.pc = self.register.pc.wrapping_add((opcode.len - 1) as u16);
            }
        }
    }

    fn transfer(&mut self, source: RegisterField, dest: RegisterField) {
        self.register.write(&dest, self.register.read(&source));
        self.update_zero_and_negative_flags(self.register.read(&dest));
    }

    fn load(&mut self, dest: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register.write(&dest, value);
        self.update_zero_and_negative_flags(value);
    }

    fn inx(&mut self) {
        self.register.x = self.register.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register.x);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register.a = value;
        self.update_zero_and_negative_flags(self.register.a);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register.a)
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // set Zero Flag if A = 0
        if result == 0 {
            self.register.status.insert(CpuFlags::ZERO);
        } else {
            self.register.status.remove(CpuFlags::ZERO);
        }

        // set Negative flag if bit 7 of A is set
        if result & 0b1000_0000 != 0 {
            self.register.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.register.status.remove(CpuFlags::NEGATIVE);
        }
    }



    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register.pc,
            AddressingMode::ZeroPage => self.mem_read(self.register.pc) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.register.pc),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.x as u16) as u16;
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.y as u16) as u16;
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.register.pc);
                let ptr = base.wrapping_add(self.register.x);
                self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.register.pc);
                let deref_base = self.mem_read_u16(base as u16);
                deref_base.wrapping_add(self.register.y as u16)
            }
            AddressingMode::NoneAddressing => { panic!("mode {:?} not supported", mode) }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cpu::{CPU, Mem};

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.a, 0x05);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xa5_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x00);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x55);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x00);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(cpu.register.x, cpu.register.a);
    }

    #[test]
    fn test_0xaa_txa_move_x_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa2, 0x10, 0x8a, 0x00]);
        assert_eq!(cpu.register.a, cpu.register.x);
        assert_eq!(cpu.register.a, 0x10);
    }


    #[test]
    fn test_0xaa_tya_move_y_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa0, 0x10, 0x98, 0x00]);
        assert_eq!(cpu.register.y, cpu.register.a);
        assert_eq!(cpu.register.a, 0x10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register.x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register.x, 1)
    }

    #[test]
    fn test_0x85_sta_write_accum_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xBA, 0x85, 0xAA, 0x00]);
        assert_eq!(cpu.register.a, 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

}