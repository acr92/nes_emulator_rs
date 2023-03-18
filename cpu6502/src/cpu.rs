use crate::opcodes;
use crate::opcodes::{is_addressing_absolute, AddressingMode, Instruction};
use crate::register::{CpuFlags, Register, RegisterField, STACK};
use core::mem::{Mem, VECTOR_NMI_INTERRUPT_HANDLER, VECTOR_RESET_HANDLER};

pub struct CPU {
    pub register: Register,

    pub complete: bool,
    pub cycles: u8,
}

fn page_cross(a: u16, b: u16) -> bool {
    (a & 0xFF00) != (b & 0xFF00)
}

impl Default for CPU {
    fn default() -> Self {
        CPU::new()
    }
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            register: Register::new(),
            complete: false,
            cycles: 0,
        }
    }

    pub fn reset(&mut self, bus: &mut impl Mem) {
        self.register = Register::new();
        self.register.pc = bus.mem_read_u16(VECTOR_RESET_HANDLER);
    }

    #[cfg(test)]
    fn eval(&mut self, bus: &mut impl Mem, program: &[u8]) {
        let base = 0x0600;
        for (pos, &e) in program.iter().enumerate() {
            bus.mem_write((base + pos) as u16, e)
        }
        self.reset(bus);
        self.register.pc = base as u16;

        let mut instructions = 0;
        while !self.complete && instructions < 1000 {
            self.tick(bus);
            instructions += 1;
        }
    }

    pub fn tick(&mut self, bus: &mut impl Mem) {
        if self.cycles > 0 {
            self.cycles -= 1;
            return;
        }

        let code = bus.mem_read(self.register.pc);
        self.register.pc = self.register.pc.wrapping_add(1);
        let program_counter_state = self.register.pc;

        let opcode = (*opcodes::OPCODES_LIST)[code as usize];

        match opcode.instruction {
            Instruction::BRK => {
                self.complete = true;
                return;
            }
            Instruction::NOP => {}
            Instruction::DOP => {}
            Instruction::TOP => {
                if self.page_crossed(bus, &opcode.mode) {
                    self.cycles += 1
                }
            }

            // Logical Operations
            Instruction::AND => {
                self.logic(bus, &opcode.mode, |a, b| a & b);
                self.tick_on_page_cross(bus, &opcode.mode);
            }
            Instruction::EOR => {
                self.logic(bus, &opcode.mode, |a, b| a ^ b);
                self.tick_on_page_cross(bus, &opcode.mode);
            }
            Instruction::ORA => {
                self.logic(bus, &opcode.mode, |a, b| a | b);
                self.tick_on_page_cross(bus, &opcode.mode);
            }
            Instruction::SAX => self.sax(bus, &opcode.mode),

            // Arithmetic Operations
            Instruction::ADC => self.adc(bus, &opcode.mode),
            Instruction::SBC => self.sbc(bus, &opcode.mode),
            Instruction::ASL => self.arithmetic_shift(bus, &opcode.mode, asl),
            Instruction::BIT => self.bit(bus, &opcode.mode),
            Instruction::DEC => self.decrement_memory(bus, &opcode.mode),
            Instruction::DEX => self.decrement_register(RegisterField::X),
            Instruction::DEY => self.decrement_register(RegisterField::Y),
            Instruction::INC => self.increment_memory(bus, &opcode.mode),
            Instruction::INX => self.increment_register(RegisterField::X),
            Instruction::INY => self.increment_register(RegisterField::Y),
            Instruction::LSR => self.arithmetic_shift(bus, &opcode.mode, lsr),
            Instruction::ROL => self.arithmetic_shift(bus, &opcode.mode, rol),
            Instruction::ROR => self.arithmetic_shift(bus, &opcode.mode, ror),

            // Branch Operations
            Instruction::BCC => self.branch(bus, !self.register.status.contains(CpuFlags::CARRY)),
            Instruction::BCS => self.branch(bus, self.register.status.contains(CpuFlags::CARRY)),
            Instruction::BNE => self.branch(bus, !self.register.status.contains(CpuFlags::ZERO)),
            Instruction::BEQ => self.branch(bus, self.register.status.contains(CpuFlags::ZERO)),
            Instruction::BPL => {
                self.branch(bus, !self.register.status.contains(CpuFlags::NEGATIVE))
            }
            Instruction::BMI => self.branch(bus, self.register.status.contains(CpuFlags::NEGATIVE)),
            Instruction::BVC => {
                self.branch(bus, !self.register.status.contains(CpuFlags::OVERFLOW))
            }
            Instruction::BVS => self.branch(bus, self.register.status.contains(CpuFlags::OVERFLOW)),

            // Jump
            Instruction::JMP if is_addressing_absolute(opcode.mode) => {
                self.jmp_absolute(bus);
            }
            Instruction::JMP => {
                self.jmp_indirect(bus);
            }
            Instruction::JSR => self.jsr(bus),
            Instruction::RTI => self.rti(bus),
            Instruction::RTS => self.rts(bus),

            // Stack
            Instruction::PHA => self.pha(bus),
            Instruction::PHP => self.php(bus),
            Instruction::PLA => self.pla(bus),
            Instruction::PLP => self.plp(bus),

            // Compare Operations
            Instruction::CMP => {
                self.compare(bus, RegisterField::A, &opcode.mode);
                self.tick_on_page_cross(bus, &opcode.mode);
            }
            Instruction::CPX => {
                self.compare(bus, RegisterField::X, &opcode.mode);
                self.tick_on_page_cross(bus, &opcode.mode);
            }
            Instruction::CPY => {
                self.compare(bus, RegisterField::Y, &opcode.mode);
                self.tick_on_page_cross(bus, &opcode.mode);
            }

            // Clear & Set Registers
            Instruction::CLC => self.register.status.remove(CpuFlags::CARRY),
            Instruction::CLD => self.register.status.remove(CpuFlags::DECIMAL_MODE),
            Instruction::CLI => self.register.status.remove(CpuFlags::INTERRUPT_DISABLE),
            Instruction::CLV => self.register.status.remove(CpuFlags::OVERFLOW),
            Instruction::SEC => self.register.status.insert(CpuFlags::CARRY),
            Instruction::SED => self.register.status.insert(CpuFlags::DECIMAL_MODE),
            Instruction::SEI => self.register.status.insert(CpuFlags::INTERRUPT_DISABLE),

            // Load Operations
            Instruction::LDA => self.load(bus, RegisterField::A, &opcode.mode),
            Instruction::LDX => self.load(bus, RegisterField::X, &opcode.mode),
            Instruction::LDY => self.load(bus, RegisterField::Y, &opcode.mode),
            Instruction::LAX => self.lax(bus, &opcode.mode),

            // Store Operations
            Instruction::STA => self.store(bus, RegisterField::A, &opcode.mode),
            Instruction::STX => self.store(bus, RegisterField::X, &opcode.mode),
            Instruction::STY => self.store(bus, RegisterField::Y, &opcode.mode),

            // Transfer Operations
            Instruction::TAX => self.transfer(RegisterField::A, RegisterField::X),
            Instruction::TAY => self.transfer(RegisterField::A, RegisterField::Y),
            Instruction::TSX => self.transfer(RegisterField::SP, RegisterField::X),
            Instruction::TXA => self.transfer(RegisterField::X, RegisterField::A),
            Instruction::TXS => self.transfer(RegisterField::X, RegisterField::SP),
            Instruction::TYA => self.transfer(RegisterField::Y, RegisterField::A),

            Instruction::DCP => self.dcp(bus, &opcode.mode),
            Instruction::ISB => self.isb(bus, &opcode.mode),
            Instruction::SLO => self.slo(bus, &opcode.mode),
            Instruction::RLA => self.rla(bus, &opcode.mode),
            Instruction::SRE => self.sre(bus, &opcode.mode),
            Instruction::RRA => self.rra(bus, &opcode.mode),

            _ => {
                panic!(
                    "Unknown opcode: {:#02X} instruction: {:#?}",
                    code, opcode.instruction
                )
            }
        }

        self.cycles += opcode.cycles;

        if program_counter_state == self.register.pc {
            self.register.pc = self.register.pc.wrapping_add((opcode.len - 1) as u16);
        }
    }

    fn transfer(&mut self, source: RegisterField, target: RegisterField) {
        self.register.write(target, self.register.read(source));
    }

    fn load(&mut self, bus: &mut impl Mem, target: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let value = bus.mem_read(addr);

        self.register.write(target, value);

        if self.page_crossed(bus, mode) {
            self.cycles += 1
        }
    }

    fn increment_register(&mut self, target: RegisterField) {
        let value = self.register.read(target).wrapping_add(1);
        self.register.write(target, value);
    }

    fn increment_memory(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let mut value = bus.mem_read(addr);

        value = value.wrapping_add(1);

        bus.mem_write(addr, value);
        self.register.update_zero_and_negative_flags(value);
    }

    fn decrement_register(&mut self, target: RegisterField) {
        let value = self.register.read(target).wrapping_sub(1);
        self.register.write(target, value);
    }

    fn decrement_memory(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let mut value = bus.mem_read(addr);

        value = value.wrapping_sub(1);

        bus.mem_write(addr, value);
        self.register.update_zero_and_negative_flags(value);
    }

    fn store(&mut self, bus: &mut impl Mem, source: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        bus.mem_write(addr, self.register.read(source))
    }

    fn compare(&mut self, bus: &mut impl Mem, source: RegisterField, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let data = bus.mem_read(addr);

        let compare_with = self.register.read(source);

        let result = compare_with.wrapping_sub(data);

        self.register
            .status
            .set(CpuFlags::CARRY, compare_with >= data);
        self.register.update_zero_and_negative_flags(result);
    }

    fn tick_on_page_cross(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        if self.page_crossed(bus, mode) {
            self.cycles += 1;
        }
    }

    fn logic<F>(&mut self, bus: &mut impl Mem, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, u8) -> u8,
    {
        let addr = self.get_operand_address(bus, mode);
        let value = op(self.register.read(RegisterField::A), bus.mem_read(addr));
        self.register.write(RegisterField::A, value);
    }

    fn stack_push(&mut self, bus: &mut impl Mem, value: u8) {
        bus.mem_write(STACK + self.register.sp as u16, value);
        self.register.sp = self.register.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self, bus: &mut impl Mem) -> u8 {
        self.register.sp = self.register.sp.wrapping_add(1);
        bus.mem_read(STACK + self.register.sp as u16)
    }

    fn stack_push_u16(&mut self, bus: &mut impl Mem, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.stack_push(bus, hi);
        self.stack_push(bus, lo);
    }

    fn stack_pop_u16(&mut self, bus: &mut impl Mem) -> u16 {
        let lo = self.stack_pop(bus) as u16;
        let hi = self.stack_pop(bus) as u16;

        hi << 8 | lo
    }

    fn dcp(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.decrement_memory(bus, mode);
        self.compare(bus, RegisterField::A, mode);
    }

    fn lax(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.load(bus, RegisterField::A, mode);
        self.register
            .write(RegisterField::X, self.register.read(RegisterField::A))
    }

    fn pla(&mut self, bus: &mut impl Mem) {
        let value = self.stack_pop(bus);
        self.register.write(RegisterField::A, value);
    }

    fn pha(&mut self, bus: &mut impl Mem) {
        self.stack_push(bus, self.register.read(RegisterField::A))
    }

    fn plp(&mut self, bus: &mut impl Mem) {
        let new_status = self.stack_pop(bus);
        self.register.write(RegisterField::Status, new_status);
        self.register.status.remove(CpuFlags::BREAK);
        self.register.status.insert(CpuFlags::BREAK2);
    }

    fn php(&mut self, bus: &mut impl Mem) {
        let mut flags = self.register.status;
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(bus, flags.bits());
    }

    fn adc(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let data = bus.mem_read(addr);
        self.add_to_register_a(data);
    }

    fn sbc(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let data = bus.mem_read(addr);
        self.add_to_register_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    fn add_to_register_a(&mut self, data: u8) {
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

    fn arithmetic_shift<F>(&mut self, bus: &mut impl Mem, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, bool) -> (u8, bool),
    {
        if matches!(mode, AddressingMode::Accumulator) {
            self.arithmetic_accumulator(&op);
        } else {
            self.arithmetic_mem(bus, mode, op);
        }
    }

    fn arithmetic_accumulator<F>(&mut self, op: &F)
    where
        F: Fn(u8, bool) -> (u8, bool),
    {
        let data = self.register.read(RegisterField::A);
        let carry = self.register.status.contains(CpuFlags::CARRY);

        let (data, carry) = op(data, carry);
        self.register.status.set(CpuFlags::CARRY, carry);

        self.register.write(RegisterField::A, data);
    }

    fn arithmetic_mem<F>(&mut self, bus: &mut impl Mem, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, bool) -> (u8, bool),
    {
        let addr = self.get_operand_address(bus, mode);
        let data = bus.mem_read(addr);
        let carry = self.register.status.contains(CpuFlags::CARRY);

        let (data, carry) = op(data, carry);
        self.register.status.set(CpuFlags::CARRY, carry);

        bus.mem_write(addr, data);
        self.register.update_zero_and_negative_flags(data);
    }

    fn bit(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let data = bus.mem_read(addr);

        let mask = self.register.read(RegisterField::A) & data;
        self.register.status.set(CpuFlags::ZERO, mask == 0);

        self.register
            .status
            .set(CpuFlags::NEGATIVE, data & 0b1000_0000 > 0);
        self.register
            .status
            .set(CpuFlags::OVERFLOW, data & 0b0100_0000 > 0);
    }

    fn sax(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        let addr = self.get_operand_address(bus, mode);
        let data = self.register.read(RegisterField::X) & self.register.read(RegisterField::A);
        bus.mem_write(addr, data);
    }

    fn isb(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.increment_memory(bus, mode);
        self.sbc(bus, mode);
    }

    fn slo(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.arithmetic_shift(bus, mode, asl);
        self.logic(bus, mode, |a, b| a | b);
    }
    fn rla(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.arithmetic_shift(bus, mode, rol);
        self.logic(bus, mode, |a, b| a & b);
    }

    fn sre(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.arithmetic_shift(bus, mode, lsr);
        self.logic(bus, mode, |a, b| a ^ b);
    }

    fn rra(&mut self, bus: &mut impl Mem, mode: &AddressingMode) {
        self.arithmetic_shift(bus, mode, ror);
        self.adc(bus, mode);
    }

    fn branch(&mut self, bus: &mut impl Mem, condition: bool) {
        if condition {
            self.cycles += 1;

            let jump: i8 = bus.mem_read(self.register.pc) as i8;
            let jump_addr = self.register.pc.wrapping_add(1).wrapping_add(jump as u16);

            if page_cross(self.register.pc.wrapping_add(1), jump_addr) {
                self.cycles += 1;
            }

            self.register.pc = jump_addr
        }
    }

    fn jmp_absolute(&mut self, bus: &mut impl Mem) {
        let addr = self.get_operand_address(bus, &AddressingMode::Absolute);
        self.register.pc = addr;
    }

    fn jmp_indirect(&mut self, bus: &mut impl Mem) {
        let addr = self.get_operand_address(bus, &AddressingMode::Absolute);

        // 6502 bug mode with with page boundary:
        //  if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
        // the result of JMP ($30FF) will be a transfer of control to $4080 rather than $5080 as you intended
        // i.e. the 6502 took the low byte of the address from $30FF and the high byte from $3000

        let indirect_ref = if addr & 0x00FF == 0x00FF {
            let lo = bus.mem_read(addr);
            let hi = bus.mem_read(addr & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {
            bus.mem_read_u16(addr)
        };

        self.register.pc = indirect_ref;
    }

    fn jsr(&mut self, bus: &mut impl Mem) {
        self.stack_push_u16(bus, self.register.pc + 2 /* op arg */ - 1 /* spec */);
        let addr = self.get_operand_address(bus, &AddressingMode::Absolute);
        self.register.pc = addr;
    }

    fn rti(&mut self, bus: &mut impl Mem) {
        self.plp(bus);
        self.register.pc = self.stack_pop_u16(bus);
    }

    fn rts(&mut self, bus: &mut impl Mem) {
        let addr = self.stack_pop_u16(bus) + 1;
        self.register.pc = addr;
    }

    fn page_crossed(&mut self, bus: &mut impl Mem, mode: &AddressingMode) -> bool {
        let addr = self.register.pc;

        match mode {
            AddressingMode::Absolute_X => {
                let base = bus.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::X) as u16);
                page_cross(base, addr)
            }
            AddressingMode::Absolute_Y => {
                let base = bus.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                page_cross(base, addr)
            }
            AddressingMode::Indirect_Y => {
                let base = bus.mem_read(addr);

                let lo = bus.mem_read(base as u16);
                let hi = bus.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                page_cross(deref, deref_base)
            }
            _ => false,
        }
    }

    pub fn get_absolute_address(
        &mut self,
        bus: &mut impl Mem,
        mode: &AddressingMode,
        addr: u16,
    ) -> u16 {
        match mode {
            AddressingMode::ZeroPage => bus.mem_read(addr) as u16,

            AddressingMode::Absolute => bus.mem_read_u16(addr),

            AddressingMode::ZeroPage_X => {
                let pos = bus.mem_read(addr);
                pos.wrapping_add(self.register.read(RegisterField::X)) as u16
            }
            AddressingMode::ZeroPage_Y => {
                let pos = bus.mem_read(addr);
                pos.wrapping_add(self.register.read(RegisterField::Y)) as u16
            }

            AddressingMode::Absolute_X => {
                let base = bus.mem_read_u16(addr);
                base.wrapping_add(self.register.read(RegisterField::X) as u16)
            }
            AddressingMode::Absolute_Y => {
                let base = bus.mem_read_u16(addr);
                base.wrapping_add(self.register.read(RegisterField::Y) as u16)
            }

            AddressingMode::Indirect_X => {
                let base = bus.mem_read(addr);

                let ptr: u8 = base.wrapping_add(self.register.read(RegisterField::X));
                let lo = bus.mem_read(ptr as u16);
                let hi = bus.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = bus.mem_read(addr);

                let lo = bus.mem_read(base as u16);
                let hi = bus.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                deref_base.wrapping_add(self.register.read(RegisterField::Y) as u16)
            }

            _ => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    fn get_operand_address(&mut self, bus: &mut impl Mem, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register.pc,
            _ => self.get_absolute_address(bus, mode, self.register.pc),
        }
    }

    pub fn interrupt_nmi(&mut self, bus: &mut impl Mem) {
        self.stack_push_u16(bus, self.register.pc);
        let mut flag = self.register.status;
        flag.set(CpuFlags::BREAK, false);
        flag.set(CpuFlags::BREAK2, true);

        self.stack_push(bus, flag.bits());
        self.register.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.cycles = 8;
        self.register.pc = bus.mem_read_u16(VECTOR_NMI_INTERRUPT_HANDLER);
    }
}

fn asl(data: u8, _: bool) -> (u8, bool) {
    let carry = data >> 7 == 1;
    let result = data << 1;
    (result, carry)
}

fn lsr(data: u8, _: bool) -> (u8, bool) {
    let result = data >> 1;
    let carry = data & 0x1 == 1;
    (result, carry)
}

fn rol(data: u8, carry: bool) -> (u8, bool) {
    let new_carry = data >> 7 == 1;
    let result = data << 1 | (carry as u8);
    (result, new_carry)
}

fn ror(data: u8, carry: bool) -> (u8, bool) {
    let new_carry = data & 0x1 == 1;
    let result = data >> 1 | ((carry as u8) << 7);
    (result, new_carry)
}

#[cfg(test)]
mod test {
    use crate::cpu::{CpuFlags, CPU};
    use crate::mock_bus::MockBus;
    use crate::opcodes;
    use crate::opcodes::AddressingMode;
    use crate::register::{RegisterField, STACK_RESET};
    use core::mem::Mem;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x05);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xa5_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x10, 0x55);
        cpu.eval(&mut bus, &[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x10, 0x00);
        cpu.eval(&mut bus, &[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write_u16(0x1020, 0x55);
        cpu.eval(&mut bus, &[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write_u16(0x1020, 0x00);
        cpu.eval(&mut bus, &[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xc1)
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 1)
    }

    #[test]
    fn test_0xc8_iny_overflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA0, 0xff, 0xaa, 0xC8, 0xC8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 1)
    }

    #[test]
    fn test_0xe6_inc() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xCA, 0x02);
        cpu.eval(&mut bus, &[0xE6, 0xCA, 0x00]);
        assert_eq!(bus.mem_read(0xCA), 0x03);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xCA, 0x02);
        cpu.eval(&mut bus, &[0xC6, 0xCA, 0x00]);
        assert_eq!(bus.mem_read(0xCA), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec_to_zero() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xCA, 0x02);
        cpu.eval(&mut bus, &[0xC6, 0xCA, 0xC6, 0xCA, 0x00]);
        assert_eq!(bus.mem_read(0xCA), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xca_dex_underflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xCA, 0xCA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x88_dey_underflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x88, 0x88, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x85_sta_write_accum_to_memory() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0xBA, 0x85, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xBA);
        assert_eq!(bus.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x86_stx_write_x_reg_to_memory() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA2, 0xBA, 0x86, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xBA);
        assert_eq!(bus.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x84_sty_write_y_reg_to_memory() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA0, 0xBA, 0x84, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 0xBA);
        assert_eq!(bus.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::X),
            cpu.register.read(RegisterField::A)
        );
    }

    #[test]
    fn test_0xaa_txa_move_x_to_a() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa2, 0x10, 0x8a, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::A),
            cpu.register.read(RegisterField::X)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_tya_move_y_to_a() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xa0, 0x10, 0x98, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::Y),
            cpu.register.read(RegisterField::A)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_txs_move_x_to_sp() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA2, 0x10, 0x9A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), cpu.register.sp);
        assert_eq!(cpu.register.sp, 0x10);
    }

    #[test]
    fn test_0xaa_tsx_move_sp_to_x() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xBA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), STACK_RESET);
    }

    #[test]
    fn test_0x38_set_carry_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.eval(&mut bus, &[0x38, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xf8_set_decimal_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.eval(&mut bus, &[0xf8, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x78_set_interrupt_disable_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x78, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }

    #[test]
    fn test_0x18_clear_carry_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.eval(&mut bus, &[0x38, 0x18, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xd8_clear_decimal_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.eval(&mut bus, &[0xf8, 0xd8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x58_clear_interrupt_disable_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x78, 0x58, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }

    #[test]
    fn test_0xb8_clear_overflow_flag() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xF0);
        cpu.eval(&mut bus, &[0xA9, 0x70, 0x24, 0xAA, 0xB8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_overflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0x70);
        cpu.eval(&mut bus, &[0xA9, 0x70, 0x24, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_zero() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0x0F);
        cpu.eval(&mut bus, &[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_negative() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xB0);
        cpu.eval(&mut bus, &[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x29_logical_and_on_immediate() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x29, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x2d_logical_and_on_absolute() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x1234, 0x07);
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x2D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x49_eor_exclusive_or_on_immediate() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x49, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x5d_eor_exclusive_or_on_absolute() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x1234, 0x07);
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x5D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x09_ora_logical_eor_on_immediate() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x09, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x0d_ora_exclusive_or_on_absolute() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x1234, 0x07);
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0x0D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x69_adc_no_overflow_no_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x02, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x04);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_overflow_carry_bit_set() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0xFF, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_zero() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0xFF, 0x69, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_sign_bit_incorrect() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x80, 0x69, 0x80, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_no_overflow() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x08, 0xE9, 0x04, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_overflow_carry_bit_set() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x18, 0xA9, 0x80, 0xE9, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x7E);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_zero() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x01, 0x38, 0xE9, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_sign_bit_incorrect() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x18, 0xA9, 0x01, 0xE9, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xFE);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x0a_asl_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x81, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x0a_asl_no_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x41, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x82);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x06_asl_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x40, 0x81);
        cpu.eval(&mut bus, &[0x06, 0x40, 0x00]);
        assert_eq!(bus.mem_read(0x40), 0x02);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x81, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x40);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_no_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x40, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x46_lsr_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x40, 0x81);
        cpu.eval(&mut bus, &[0x46, 0x40, 0x00]);
        assert_eq!(bus.mem_read(0x40), 0x40);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x81, 0x2A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol_no_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x40, 0x2A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x80);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2e_rol_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x40, 0x81);
        cpu.eval(&mut bus, &[0x2E, 0x40, 0x00]);
        assert_eq!(bus.mem_read(0x40), 0x02);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x81, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x40);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_no_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x40, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_carry_flag_already_set() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x40, 0x38, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xA0);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_ror_update_memory_and_set_carry() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x40, 0x81);
        cpu.eval(&mut bus, &[0x6E, 0x40, 0x00]);
        assert_eq!(bus.mem_read(0x40), 0x40);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_equal() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0xAA, 0xC9, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_gt_eq() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0xFF, 0xC9, 0x00, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xc5_cmp_equal() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xF0);
        cpu.eval(&mut bus, &[0xA9, 0xF0, 0xC5, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xe0_cpx() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA2, 0xAA, 0xE0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xec_cpx() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xF0);
        cpu.eval(&mut bus, &[0xA2, 0xF0, 0xEC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc0_cpy() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA0, 0xAA, 0xC0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xcc_cpy() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xF0);
        cpu.eval(&mut bus, &[0xA0, 0xF0, 0xCC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x90_bcc_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x90, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(bus.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xb0_bcs_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xB0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(bus.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0xf0_beq_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xF0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(bus.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x30_bmi_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x30, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(bus.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xd0_bne_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xD0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x03);
        assert_eq!(bus.mem_read(0x0201), 0x03);
    }

    #[test]
    fn test_0x10_bpl_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x10, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(bus.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0x50_bvc_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xA9, 0xF0, 0x85, 0x44, 0xCA, 0x24, 0x44, 0xE0, 0x03, 0x50, 0xF9, 0x8E,
                0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(bus.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x70_bvs_loop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x70, 0xF8, 0x8E, 0x01, 0x02, 0x00,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(bus.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x4c_jmp_absolute() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA9, 0x03, 0x4C, 0x08, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(bus.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write_u16(0x0610, 0x0608);
        cpu.eval(
            &mut bus,
            &[
                0xA9, 0x03, 0x6C, 0x10, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(bus.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect_6502_bug() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0x08FF, 0x08);
        bus.mem_write(0x0800, 0x06);
        cpu.eval(
            &mut bus,
            &[
                0xA9, 0x03, 0x6C, 0xFF, 0x08, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(bus.mem_read(0x0200), 0x03);
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
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0x20, 0x10, 0x06, 0x20, 0x0A, 0x06, 0x20, 0x09, 0x06, 0x00, 0xE8, 0xE0, 0x05, 0xD0,
                0xFB, 0x60, 0xA2, 0x00, 0x60,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::X), 0x05);
        // end: is a subroutine, so stack isn't completely reset
        assert_eq!(cpu.register.sp, STACK_RESET - 2);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.stack_push_u16(&mut bus, 0xCAFE);
        cpu.stack_push_u16(&mut bus, 0xAABB);
        cpu.stack_push_u16(&mut bus, 0xCCDD);
        assert_eq!(cpu.stack_pop_u16(&mut bus), 0xCCDD);
        assert_eq!(cpu.stack_pop_u16(&mut bus), 0xAABB);
        assert_eq!(cpu.stack_pop_u16(&mut bus), 0xCAFE);
    }

    #[test]
    fn test_0x48_pha() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x20, 0x48, 0x00]);
        assert_eq!(cpu.stack_pop(&mut bus), 0x20);
    }

    #[test]
    fn test_0x08_php() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0x08, 0x00]);
        assert_eq!(cpu.stack_pop(&mut bus), 0b110100);
    }

    #[test]
    fn test_0x68_pla() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x20, 0x48, 0xA9, 0x30, 0x68, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
    }

    #[test]
    fn test_0x28_plp() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        /*
           SEC
           PHP
           SEI
           PLP
        */
        cpu.eval(&mut bus, &[0x38, 0x08, 0x78, 0x28, 0x00]);
        assert_eq!(cpu.register.status.bits(), 0b100101);
    }

    #[test]
    fn test_0x28_plp_sets_correct_flags() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        /*
           LDA #$FF
           PHA
           PLP
        */
        cpu.eval(&mut bus, &[0xA9, 0xFF, 0x48, 0x28, 0x00]);
        assert_eq!(cpu.register.status.bits(), 0xEF);
    }

    #[test]
    fn test_0xaf_lax() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        bus.mem_write(0xAA, 0xBB);
        cpu.eval(&mut bus, &[0xAF, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xBB);
        assert_eq!(cpu.register.read(RegisterField::X), 0xBB);
        assert_eq!(cpu.register.read(RegisterField::Y), 0x00);
    }

    #[test]
    fn test_0x83_sax_should_not_affect_flags() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(&mut bus, &[0xA9, 0x04, 0xA2, 0x02, 0x83, 0x49, 0x00]);

        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_stack_program_multiple_loops() {
        /*
          LDX #$00
          LDY #$00
        firstloop:
          TXA
          STA $0200,Y
          PHA
          INX
          INY
          CPY #$10
          BNE firstloop ;loop until Y is $10
        secondloop:
          PLA
          STA $0200,Y
          INY
          CPY #$20      ;loop until Y is $20
          BNE secondloop
         */
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.eval(
            &mut bus,
            &[
                0xA2, 0x00, 0xA0, 0x00, 0x8A, 0x99, 0x00, 0x02, 0x48, 0xE8, 0xC8, 0xC0, 0x10, 0xD0,
                0xF5, 0x68, 0x99, 0x00, 0x02, 0xC8, 0xC0, 0x20, 0xD0, 0xF7,
            ],
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.read(RegisterField::X), 0x10);
        assert_eq!(cpu.register.read(RegisterField::Y), 0x20);
        assert_eq!(bus.mem_read(0x0200), 0x00);
        assert_eq!(bus.mem_read(0x0201), 0x01);
        assert_eq!(bus.mem_read(0x0210), 0x0F);
    }

    #[test]
    fn test_all_official_operations_implemented() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        let ref opcodes = *opcodes::CPU_OPCODES;

        for op in opcodes {
            if op.unofficial_name == None {
                cpu.eval(&mut bus, &[op.code, 0x00, 0x00, 0x00, 0x00]);
            }
        }
    }

    #[test]
    fn test_immediate_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x200;
        let value = cpu.get_operand_address(&mut bus, &AddressingMode::Immediate);
        assert_eq!(cpu.register.pc, value);
    }

    #[test]
    fn test_zero_page_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write(0x10, 0x42);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::ZeroPage),
            0x42
        );
    }

    #[test]
    fn test_absolute_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write_u16(0x10, 0x1234);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::Absolute),
            0x1234
        );
    }

    #[test]
    fn test_zero_page_x_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write(0x10, 0x10);
        cpu.register.write(RegisterField::X, 0x32);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::ZeroPage_X),
            0x42
        );
    }

    #[test]
    fn test_zero_page_y_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write(0x10, 0x10);
        cpu.register.write(RegisterField::Y, 0x22);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::ZeroPage_Y),
            0x32
        );
    }

    #[test]
    fn test_absolute_x_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write_u16(0x10, 0x1234);
        cpu.register.write(RegisterField::X, 0x05);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::Absolute_X),
            0x1239
        );
    }

    #[test]
    fn test_absolute_y_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write_u16(0x10, 0x1000);
        cpu.register.write(RegisterField::Y, 0x05);
        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::Absolute_Y),
            0x1005
        );
    }

    #[test]
    fn test_indirect_x_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write(0x10, 0x80);
        cpu.register.write(RegisterField::X, 0x05);
        bus.mem_write_u16(0x85, 0x2000);

        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::Indirect_X),
            0x2000
        );
    }

    #[test]
    fn test_indirect_y_mode() {
        let mut cpu = CPU::new();
        let mut bus = MockBus::new();
        cpu.register.pc = 0x10;
        bus.mem_write(0x10, 0x50);
        bus.mem_write_u16(0x50, 0x2000);
        cpu.register.write(RegisterField::Y, 0x05);

        assert_eq!(
            cpu.get_operand_address(&mut bus, &AddressingMode::Indirect_Y),
            0x2005
        );
    }

    #[test]
    #[should_panic]
    fn test_get_operand_address_invalid_mode_should_panic() {
        let mut bus = MockBus::new();
        CPU::new().get_operand_address(&mut bus, &AddressingMode::Accumulator);
    }
}
