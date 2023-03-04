use crate::opcodes;
use crate::opcodes::{AddressingMode, Instruction};
use crate::register::{CpuFlags, Register, RegisterField};

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
            register: Register::new(),
            memory: [0; 0xFFFF],
        }
    }

    pub fn reset(&mut self) {
        self.register = Register::new();
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
                Instruction::NOP => {}


                Instruction::DEX => { self.decrement(RegisterField::X) }
                Instruction::DEY => { self.decrement(RegisterField::Y) }

                Instruction::INX => { self.increment(RegisterField::X) }
                Instruction::INY => { self.increment(RegisterField::Y) }

                Instruction::LDA => { self.load(RegisterField::A, &opcode.mode) }
                Instruction::LDX => { self.load(RegisterField::X, &opcode.mode) }
                Instruction::LDY => { self.load(RegisterField::Y, &opcode.mode) }

                Instruction::SEC => { self.register.status.insert(CpuFlags::CARRY) }
                Instruction::SED => { self.register.status.insert(CpuFlags::DECIMAL_MODE) }
                Instruction::SEI => { self.register.status.insert(CpuFlags::INTERRUPT_DISABLE) }

                Instruction::STA => { self.store(RegisterField::A, &opcode.mode) }
                Instruction::STX => { self.store(RegisterField::X, &opcode.mode) }
                Instruction::STY => { self.store(RegisterField::Y, &opcode.mode) }

                Instruction::TAX => { self.transfer(RegisterField::A, RegisterField::X) }
                Instruction::TAY => { self.transfer(RegisterField::A, RegisterField::Y) }
                Instruction::TSX => { self.transfer(RegisterField::SP, RegisterField::X) }
                Instruction::TXA => { self.transfer(RegisterField::X, RegisterField::A) }
                Instruction::TXS => { self.transfer(RegisterField::X, RegisterField::SP) }
                Instruction::TYA => { self.transfer(RegisterField::Y, RegisterField::A) }

                _ => { todo!("Unknown opcode 0x{:X} {:#?}", code, opcode.instruction) }
            }

            if program_counter_state == self.register.pc {
                self.register.pc = self.register.pc.wrapping_add((opcode.len - 1) as u16);
            }
        }
    }

    fn transfer(&mut self, source: RegisterField, target: RegisterField) {
        self.register.write(&target, self.register.read(&source));
    }

    fn load(&mut self, target: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register.write(&target, value);
    }

    fn increment(&mut self, target: RegisterField) {
        let value = self.register.read(&target).wrapping_add(1);
        self.register.write(&target, value);
    }

    fn decrement(&mut self, target: RegisterField) {
        let value = self.register.read(&target).wrapping_sub(1);
        self.register.write(&target, value);
    }

    fn store(&mut self, source: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register.read(&source))
    }

    fn and(&self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register.pc,
            AddressingMode::ZeroPage => self.mem_read(self.register.pc) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.register.pc),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.read(&RegisterField::A)) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.read(&RegisterField::Y)) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.read(&RegisterField::X) as u16) as u16;
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.read(&RegisterField::Y) as u16) as u16;
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.register.pc);
                let ptr = base.wrapping_add(self.register.read(&RegisterField::X));
                self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.register.pc);
                let deref_base = self.mem_read_u16(base as u16);
                deref_base.wrapping_add(self.register.read(&RegisterField::Y) as u16)
            }
            AddressingMode::NoneAddressing => { panic!("mode {:?} not supported", mode) }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cpu::{CPU, CpuFlags, Mem};
    use crate::register::{RegisterField, STACK_RESET};

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), 0x05);
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
        assert_eq!(cpu.register.read(&RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x00);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x55);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x00);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }


    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), 1)
    }


    #[test]
    fn test_iny_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA0, 0xff, 0xaa, 0xC8, 0xC8, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::Y), 1)
    }

    #[test]
    fn test_dex_underflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xCA, 0xCA, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_dey_underflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0x88, 0x88, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::Y), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x85_sta_write_accum_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xBA, 0x85, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }


    #[test]
    fn test_0x86_stx_write_x_reg_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA2, 0xBA, 0x86, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x84_sty_write_y_reg_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA0, 0xBA, 0x84, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::Y), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), cpu.register.read(&RegisterField::A));
    }

    #[test]
    fn test_0xaa_txa_move_x_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa2, 0x10, 0x8a, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::A), cpu.register.read(&RegisterField::X));
        assert_eq!(cpu.register.read(&RegisterField::A), 0x10);
    }


    #[test]
    fn test_0xaa_tya_move_y_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa0, 0x10, 0x98, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::Y), cpu.register.read(&RegisterField::A));
        assert_eq!(cpu.register.read(&RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_txs_move_x_to_sp() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA2, 0x10, 0x9A, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), cpu.register.sp);
        assert_eq!(cpu.register.sp, 0x10);
    }

    #[test]
    fn test_0xaa_tsx_move_sp_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xBA, 0x00]);
        assert_eq!(cpu.register.read(&RegisterField::X), STACK_RESET);
    }

    #[test]
    fn test_0x38_set_carry_flag() {
        let mut cpu = CPU::new();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.load_and_run(&[0x38, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xf8_set_decimal_flag() {
        let mut cpu = CPU::new();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.load_and_run(&[0xf8, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x78_set_interrupt_disable_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0x78, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }
}