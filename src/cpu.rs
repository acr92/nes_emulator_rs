use crate::opcodes;
use crate::opcodes::{
    is_addressing_absolute, is_addressing_accumulator, AddressingMode, Instruction,
};
use crate::register::{CpuFlags, Register, RegisterField, STACK};

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

            let opcode = opcodes
                .get(&code)
                .expect(&format!("Opcode {:x} is not recognized", code));

            println!(
                "Processing {:#?} pc={:x}",
                opcode.instruction, self.register.pc
            );

            match opcode.instruction {
                Instruction::BRK => {
                    return;
                }
                Instruction::NOP => {}

                // Logical Operations
                Instruction::AND => self.logic(&opcode.mode, |a, b| a & b),
                Instruction::EOR => self.logic(&opcode.mode, |a, b| a ^ b),
                Instruction::ORA => self.logic(&opcode.mode, |a, b| a | b),

                // Arithmetic Operations
                Instruction::ADC => self.adc(&opcode.mode),
                Instruction::ASL if is_addressing_accumulator(opcode.mode) => {
                    self.asl_accumulator()
                }
                Instruction::ASL => self.asl_memory(&opcode.mode),
                Instruction::BIT => self.bit(&opcode.mode),
                Instruction::DEC => self.decrement_memory(&opcode.mode),
                Instruction::DEX => self.decrement_register(RegisterField::X),
                Instruction::DEY => self.decrement_register(RegisterField::Y),
                Instruction::INC => self.increment_memory(&opcode.mode),
                Instruction::INX => self.increment_register(RegisterField::X),
                Instruction::INY => self.increment_register(RegisterField::Y),

                // Branch Operations
                Instruction::BCC => self.branch(!self.register.status.contains(CpuFlags::CARRY)),
                Instruction::BCS => self.branch(self.register.status.contains(CpuFlags::CARRY)),
                Instruction::BNE => self.branch(!self.register.status.contains(CpuFlags::ZERO)),
                Instruction::BEQ => self.branch(self.register.status.contains(CpuFlags::ZERO)),
                Instruction::BPL => self.branch(!self.register.status.contains(CpuFlags::NEGATIVE)),
                Instruction::BMI => self.branch(self.register.status.contains(CpuFlags::NEGATIVE)),
                Instruction::BVC => self.branch(!self.register.status.contains(CpuFlags::OVERFLOW)),
                Instruction::BVS => self.branch(self.register.status.contains(CpuFlags::OVERFLOW)),

                // Jump
                Instruction::JMP if is_addressing_absolute(opcode.mode) => {
                    self.jmp_absolute();
                }
                Instruction::JMP => {
                    self.jmp_indirect();
                }
                Instruction::JSR => self.jsr(),
                Instruction::RTS => self.rts(),

                // Compare Operations
                Instruction::CMP => self.compare(RegisterField::A, &opcode.mode),
                Instruction::CPX => self.compare(RegisterField::X, &opcode.mode),
                Instruction::CPY => self.compare(RegisterField::Y, &opcode.mode),

                // Clear & Set Registers
                Instruction::CLC => self.register.status.remove(CpuFlags::CARRY),
                Instruction::CLD => self.register.status.remove(CpuFlags::DECIMAL_MODE),
                Instruction::CLI => self.register.status.remove(CpuFlags::INTERRUPT_DISABLE),
                Instruction::CLV => self.register.status.remove(CpuFlags::OVERFLOW),
                Instruction::SEC => self.register.status.insert(CpuFlags::CARRY),
                Instruction::SED => self.register.status.insert(CpuFlags::DECIMAL_MODE),
                Instruction::SEI => self.register.status.insert(CpuFlags::INTERRUPT_DISABLE),

                // Load Operations
                Instruction::LDA => self.load(RegisterField::A, &opcode.mode),
                Instruction::LDX => self.load(RegisterField::X, &opcode.mode),
                Instruction::LDY => self.load(RegisterField::Y, &opcode.mode),
                Instruction::LSR if is_addressing_accumulator(opcode.mode) => {
                    self.lsr_accumulator()
                }
                Instruction::LSR => self.lsr_memory(&opcode.mode),

                // Store Operations
                Instruction::STA => self.store(RegisterField::A, &opcode.mode),
                Instruction::STX => self.store(RegisterField::X, &opcode.mode),
                Instruction::STY => self.store(RegisterField::Y, &opcode.mode),

                // Transfer Operations
                Instruction::TAX => self.transfer(RegisterField::A, RegisterField::X),
                Instruction::TAY => self.transfer(RegisterField::A, RegisterField::Y),
                Instruction::TSX => self.transfer(RegisterField::SP, RegisterField::X),
                Instruction::TXA => self.transfer(RegisterField::X, RegisterField::A),
                Instruction::TXS => self.transfer(RegisterField::X, RegisterField::SP),
                Instruction::TYA => self.transfer(RegisterField::Y, RegisterField::A),

                _ => {
                    todo!("Unknown opcode 0x{:X} {:#?}", code, opcode.instruction)
                }
            }

            if program_counter_state == self.register.pc {
                self.register.pc = self.register.pc.wrapping_add((opcode.len - 1) as u16);
            }
        }
    }

    fn transfer(&mut self, source: RegisterField, target: RegisterField) {
        self.register.write(target, self.register.read(source));
    }

    fn load(&mut self, target: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register.write(target, value);
    }

    fn increment_register(&mut self, target: RegisterField) {
        let value = self.register.read(target).wrapping_add(1);
        self.register.write(target, value);
    }

    fn increment_memory(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        value = value.wrapping_add(1);

        self.mem_write(addr, value);
        self.register.update_zero_and_negative_flags(value);
    }

    fn decrement_register(&mut self, target: RegisterField) {
        let value = self.register.read(target).wrapping_sub(1);
        self.register.write(target, value);
    }

    fn decrement_memory(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);

        value = value.wrapping_sub(1);

        self.mem_write(addr, value);
        self.register.update_zero_and_negative_flags(value);
    }

    fn store(&mut self, source: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register.read(source))
    }

    fn compare(&mut self, source: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);

        let lhs = self.register.read(source);
        let rhs = self.mem_read(addr);

        let result = lhs.wrapping_sub(rhs);

        self.register.status.set(CpuFlags::CARRY, lhs >= rhs);
        self.register.status.set(CpuFlags::ZERO, lhs == rhs);
        self.register.status.set(CpuFlags::NEGATIVE, result >= 0x80);
    }

    fn logic<F>(&mut self, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, u8) -> u8,
    {
        let addr = self.get_operand_address(mode);
        let value = op(self.register.read(RegisterField::A), self.mem_read(addr));
        self.register.write(RegisterField::A, value);
    }

    fn stack_push(&mut self, value: u8) {
        self.mem_write((STACK as u16) + self.register.sp as u16, value);
        self.register.sp = self.register.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.register.sp = self.register.sp.wrapping_add(1);
        self.mem_read((STACK as u16) + self.register.sp as u16)
    }

    fn stack_push_u16(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        hi << 8 | lo
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let a = self.register.read(RegisterField::A);
        let carry = if self.register.status.contains(CpuFlags::CARRY) {
            1
        } else {
            0
        };

        let sum = a as u16 + data as u16 + carry;
        self.register.status.set(CpuFlags::CARRY, sum > 0xFF);

        let result = sum as u8;

        self.register.status.set(
            CpuFlags::OVERFLOW,
            (data ^ result) & (result ^ a) & 0x80 != 0,
        );
        self.register.write(RegisterField::A, result);
    }

    fn asl_accumulator(&mut self) {
        let mut data = self.register.read(RegisterField::A);

        self.register.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;

        self.register.write(RegisterField::A, data);
    }

    fn asl_memory(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);

        self.register.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;

        self.mem_write(addr, data);
        self.register.update_zero_and_negative_flags(data);
    }

    fn lsr_accumulator(&mut self) {
        let mut data = self.register.read(RegisterField::A);

        self.register.status.set(CpuFlags::CARRY, data & 0x1 == 1);
        data >>= 1;

        self.register.write(RegisterField::A, data);
    }

    fn lsr_memory(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);

        self.register.status.set(CpuFlags::CARRY, data & 0x1 == 1);
        data >>= 1;

        self.mem_write(addr, data);
        self.register.update_zero_and_negative_flags(data);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.register.read(RegisterField::A) & self.mem_read(addr);

        self.register
            .status
            .set(CpuFlags::NEGATIVE, value & 0b1000_0000 != 0);
        self.register
            .status
            .set(CpuFlags::OVERFLOW, value & 0b0100_0000 != 0);
        self.register.status.set(CpuFlags::ZERO, value == 0);
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump: i8 = self.mem_read(self.register.pc) as i8;
            let jump_addr = self.register.pc.wrapping_add(1).wrapping_add(jump as u16);
            self.register.pc = jump_addr
        }
    }

    fn jmp_absolute(&mut self) {
        let addr = self.get_operand_address(&AddressingMode::Absolute);
        self.register.pc = addr;
    }

    fn jmp_indirect(&mut self) {
        let addr = self.get_operand_address(&AddressingMode::Absolute);

        // 6502 bug mode with with page boundary:
        //  if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
        // the result of JMP ($30FF) will be a transfer of control to $4080 rather than $5080 as you intended
        // i.e. the 6502 took the low byte of the address from $30FF and the high byte from $3000

        let indirect_ref = if addr & 0x00FF == 0x00FF {
            let lo = self.mem_read(addr);
            let hi = self.mem_read(addr & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {
            self.mem_read_u16(addr)
        };

        self.register.pc = indirect_ref;
    }

    fn jsr(&mut self) {
        self.stack_push_u16(self.register.pc + 2 /* op arg */ - 1 /* spec */);
        let addr = self.get_operand_address(&AddressingMode::Absolute);
        self.register.pc = addr;
    }

    fn rts(&mut self) {
        let addr = self.stack_pop_u16() + 1;
        self.register.pc = addr;
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register.pc,
            AddressingMode::ZeroPage => self.mem_read(self.register.pc) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.register.pc),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.read(RegisterField::A)) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.register.pc);
                let addr = pos.wrapping_add(self.register.read(RegisterField::Y)) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.read(RegisterField::X) as u16) as u16;
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.register.pc);
                let addr = base.wrapping_add(self.register.read(RegisterField::Y) as u16) as u16;
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.register.pc);
                let ptr = base.wrapping_add(self.register.read(RegisterField::X));
                self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.register.pc);
                let deref_base = self.mem_read_u16(base as u16);
                deref_base.wrapping_add(self.register.read(RegisterField::Y) as u16)
            }
            AddressingMode::Accumulator => {
                panic!("mode {:?} not supported", mode)
            }
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} not supported", mode)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cpu::{CpuFlags, Mem, CPU};
    use crate::opcodes;
    use crate::register::{RegisterField, STACK_RESET};

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x05);
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
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x00);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x55);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x00);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xc1)
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 1)
    }

    #[test]
    fn test_0xc8_iny_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA0, 0xff, 0xaa, 0xC8, 0xC8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 1)
    }

    #[test]
    fn test_0xe6_inc() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xCA, 0x02);
        cpu.load_and_run(&[0xE6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x03);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xCA, 0x02);
        cpu.load_and_run(&[0xC6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec_to_zero() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xCA, 0x02);
        cpu.load_and_run(&[0xC6, 0xCA, 0xC6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xca_dex_underflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xCA, 0xCA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x88_dey_underflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0x88, 0x88, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x85_sta_write_accum_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xBA, 0x85, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x86_stx_write_x_reg_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA2, 0xBA, 0x86, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x84_sty_write_y_reg_to_memory() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA0, 0xBA, 0x84, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::X),
            cpu.register.read(RegisterField::A)
        );
    }

    #[test]
    fn test_0xaa_txa_move_x_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa2, 0x10, 0x8a, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::A),
            cpu.register.read(RegisterField::X)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_tya_move_y_to_a() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa0, 0x10, 0x98, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::Y),
            cpu.register.read(RegisterField::A)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_txs_move_x_to_sp() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA2, 0x10, 0x9A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), cpu.register.sp);
        assert_eq!(cpu.register.sp, 0x10);
    }

    #[test]
    fn test_0xaa_tsx_move_sp_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xBA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), STACK_RESET);
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

    #[test]
    fn test_0x18_clear_carry_flag() {
        let mut cpu = CPU::new();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.load_and_run(&[0x38, 0x18, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xd8_clear_decimal_flag() {
        let mut cpu = CPU::new();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.load_and_run(&[0xf8, 0xd8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x58_clear_interrupt_disable_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0x78, 0x58, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }

    #[test]
    fn test_0xb8_clear_overflow_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xF0);
        cpu.load_and_run(&[0xA9, 0x70, 0x24, 0xAA, 0xB8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_overflow() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xF0);
        cpu.load_and_run(&[0xA9, 0x70, 0x24, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_zero() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0x0F);
        cpu.load_and_run(&[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_negative() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xB0);
        cpu.load_and_run(&[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x29_logical_and_on_immediate() {
        let mut cpu = CPU::new();
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.load_and_run(&[0xA9, 0xAA, 0x29, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x2d_logical_and_on_absolute() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.load_and_run(&[0xA9, 0xAA, 0x2D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x49_eor_exclusive_or_on_immediate() {
        let mut cpu = CPU::new();
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.load_and_run(&[0xA9, 0xAA, 0x49, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x5d_eor_exclusive_or_on_absolute() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.load_and_run(&[0xA9, 0xAA, 0x5D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x09_ora_logical_eor_on_immediate() {
        let mut cpu = CPU::new();
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.load_and_run(&[0xA9, 0xAA, 0x09, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x0d_ora_exclusive_or_on_absolute() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.load_and_run(&[0xA9, 0xAA, 0x0D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x69_adc_no_overflow_no_carry() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x02, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x04);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_overflow_carry_bit_set() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xFF, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_zero() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xFF, 0x69, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_sign_bit_incorrect() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x80, 0x69, 0x80, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x0a_asl_carry() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x81, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x0a_asl_no_carry() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x41, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x82);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x06_asl_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x40, 0x81);
        cpu.load_and_run(&[0x06, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x02);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_carry() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x81, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x40);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_no_carry() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0x40, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x46_lsr_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x40, 0x81);
        cpu.load_and_run(&[0x46, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x40);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_equal() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xAA, 0xC9, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_gt_eq() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA9, 0xFF, 0xC9, 0x00, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xc5_cmp_equal() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xF0);
        cpu.load_and_run(&[0xA9, 0xF0, 0xC5, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xe0_cpx() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA2, 0xAA, 0xE0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xec_cpx() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xF0);
        cpu.load_and_run(&[0xA2, 0xF0, 0xEC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc0_cpy() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xA0, 0xAA, 0xC0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xcc_cpy() {
        let mut cpu = CPU::new();
        cpu.mem_write(0xAA, 0xF0);
        cpu.load_and_run(&[0xA0, 0xF0, 0xCC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x90_bcc_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x90, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xb0_bcs_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xB0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(cpu.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0xf0_beq_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xF0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x30_bmi_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x30, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xd0_bne_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xD0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x03);
        assert_eq!(cpu.mem_read(0x0201), 0x03);
    }

    #[test]
    fn test_0x10_bpl_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x10, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(cpu.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0x50_bvc_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xA9, 0xF0, 0x85, 0x44, 0xCA, 0x24, 0x44, 0xE0, 0x03, 0x50, 0xF9, 0x8E,
            0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x70_bvs_loop() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x70, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x4c_jmp_absolute() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0xA9, 0x03, 0x4C, 0x08, 0x80, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(cpu.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x0610, 0x8008);
        cpu.load_and_run(&[
            0xA9, 0x03, 0x6C, 0x10, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(cpu.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect_6502_bug() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x06FF, 0x08);
        cpu.mem_write(0x0600, 0x80);
        cpu.load_and_run(&[
            0xA9, 0x03, 0x6C, 0xFF, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(cpu.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x20_jsr_and_0x60_rts() {
        /*
          JSR init
          JSR loop
          JSR end

        end:
          BRK

        loop:
          INX
          CPX #$05
          BNE loop
          RTS

        init:
          LDX #$00
          RTS

         */
        let mut cpu = CPU::new();
        cpu.load_and_run(&[
            0x20, 0x10, 0x80, 0x20, 0x0A, 0x80, 0x20, 0x09, 0x80, 0x00, 0xE8, 0xE0, 0x05, 0xD0,
            0xFB, 0x60, 0xA2, 0x00, 0x60,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x05);
        // end: is a subroutine, so stack isn't completely reset
        assert_eq!(cpu.register.sp, STACK_RESET - 2);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut cpu = CPU::new();
        cpu.stack_push_u16(0xCAFE);
        cpu.stack_push_u16(0xAABB);
        cpu.stack_push_u16(0xCCDD);
        assert_eq!(cpu.stack_pop_u16(), 0xCCDD);
        assert_eq!(cpu.stack_pop_u16(), 0xAABB);
        assert_eq!(cpu.stack_pop_u16(), 0xCAFE);
    }

    #[test]
    fn test_all_operations_implemented() {
        let mut cpu = CPU::new();
        let ref opcodes = *opcodes::CPU_OPCODES;

        for op in opcodes {
            cpu.load_and_run(&[op.code, 0x00, 0x00, 0x00, 0x00]);
        }
    }
}
