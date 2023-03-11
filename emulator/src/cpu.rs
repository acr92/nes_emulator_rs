use crate::bus::Bus;
use crate::opcodes;
use crate::opcodes::{is_addressing_absolute, AddressingMode, Instruction};
use crate::register::{CpuFlags, Register, RegisterField, STACK};
use core::mem::{Mem, VECTOR_NMI_INTERRUPT_HANDLER, VECTOR_RESET_HANDLER};

pub struct CPU<'a> {
    pub register: Register,
    pub bus: Bus<'a>,
}

impl<'a> Mem for CPU<'a> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.bus.mem_write(addr, value)
    }
}

fn page_cross(a: u16, b: u16) -> bool {
    (a & 0xFF00) != (b & 0xFF00)
}

impl<'a> CPU<'a> {
    pub fn new(bus: Bus) -> CPU {
        CPU {
            register: Register::new(),
            bus,
        }
    }

    pub fn reset(&mut self) {
        self.register = Register::new();
        self.register.pc = self.mem_read_u16(VECTOR_RESET_HANDLER);
    }

    #[cfg(test)]
    fn eval(&mut self, program: &[u8]) {
        let base = 0x0600;
        for (pos, &e) in program.iter().enumerate() {
            self.mem_write((base + pos) as u16, e)
        }
        self.reset();
        self.register.pc = base as u16;
        self.run()
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes = *opcodes::OPCODES_MAP;

        loop {
            if let Some(_nmi) = self.bus.poll_nmi_status() {
                self.interrupt_nmi();
            }

            callback(self);

            let code = self.mem_read(self.register.pc);
            self.register.pc = self.register.pc.wrapping_add(1);
            let program_counter_state = self.register.pc;

            let opcode = opcodes
                .get(&code)
                .expect(&format!("Opcode {:x} is not recognized", code));

            match opcode.instruction {
                Instruction::BRK => {
                    return;
                }
                Instruction::NOP => {}
                Instruction::DOP => {}
                Instruction::TOP => {
                    if self.page_crossed(&opcode.mode) {
                        self.bus.tick(1)
                    }
                }

                // Logical Operations
                Instruction::AND => self
                    .tick_on_page_cross(&opcode.mode, |cpu| cpu.logic(&opcode.mode, |a, b| a & b)),
                Instruction::EOR => self
                    .tick_on_page_cross(&opcode.mode, |cpu| cpu.logic(&opcode.mode, |a, b| a ^ b)),
                Instruction::ORA => self
                    .tick_on_page_cross(&opcode.mode, |cpu| cpu.logic(&opcode.mode, |a, b| a | b)),
                Instruction::SAX => self.sax(&opcode.mode),

                // Arithmetic Operations
                Instruction::ADC => self.adc(&opcode.mode),
                Instruction::SBC => self.sbc(&opcode.mode),
                Instruction::ASL => self.arithmetic_shift(&opcode.mode, asl),
                Instruction::BIT => self.bit(&opcode.mode),
                Instruction::DEC => self.decrement_memory(&opcode.mode),
                Instruction::DEX => self.decrement_register(RegisterField::X),
                Instruction::DEY => self.decrement_register(RegisterField::Y),
                Instruction::INC => self.increment_memory(&opcode.mode),
                Instruction::INX => self.increment_register(RegisterField::X),
                Instruction::INY => self.increment_register(RegisterField::Y),
                Instruction::LSR => self.arithmetic_shift(&opcode.mode, lsr),
                Instruction::ROL => self.arithmetic_shift(&opcode.mode, rol),
                Instruction::ROR => self.arithmetic_shift(&opcode.mode, ror),

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
                Instruction::RTI => self.rti(),
                Instruction::RTS => self.rts(),

                // Stack
                Instruction::PHA => self.pha(),
                Instruction::PHP => self.php(),
                Instruction::PLA => self.pla(),
                Instruction::PLP => self.plp(),

                // Compare Operations
                Instruction::CMP => self.tick_on_page_cross(&opcode.mode, |cpu| {
                    cpu.compare(RegisterField::A, &opcode.mode)
                }),
                Instruction::CPX => self.tick_on_page_cross(&opcode.mode, |cpu| {
                    cpu.compare(RegisterField::X, &opcode.mode)
                }),
                Instruction::CPY => self.tick_on_page_cross(&opcode.mode, |cpu| {
                    cpu.compare(RegisterField::Y, &opcode.mode)
                }),

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
                Instruction::LAX => self.lax(&opcode.mode),

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

                Instruction::DCP => self.dcp(&opcode.mode),
                Instruction::ISB => self.isb(&opcode.mode),
                Instruction::SLO => self.slo(&opcode.mode),
                Instruction::RLA => self.rla(&opcode.mode),
                Instruction::SRE => self.sre(&opcode.mode),
                Instruction::RRA => self.rra(&opcode.mode),

                _ => {
                    panic!(
                        "Unknown opcode: {:#02X} instruction: {:#?}",
                        code, opcode.instruction
                    )
                }
            }

            self.bus.tick(opcode.cycles);

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

        if self.page_crossed(&mode) {
            self.bus.tick(1)
        }
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
        let data = self.mem_read(addr);

        let compare_with = self.register.read(source);

        let result = compare_with.wrapping_sub(data);

        self.register
            .status
            .set(CpuFlags::CARRY, compare_with >= data);
        self.register.update_zero_and_negative_flags(result);
    }

    fn tick_on_page_cross<F>(&mut self, mode: &AddressingMode, function: F)
    where
        F: FnOnce(&mut CPU),
    {
        function(self);

        if self.page_crossed(mode) {
            self.bus.tick(1);
        }
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

    fn dcp(&mut self, mode: &AddressingMode) {
        self.decrement_memory(&mode);
        self.compare(RegisterField::A, &mode);
    }

    fn lax(&mut self, mode: &AddressingMode) {
        self.load(RegisterField::A, mode);
        self.register
            .write(RegisterField::X, self.register.read(RegisterField::A))
    }

    fn pla(&mut self) {
        let value = self.stack_pop();
        self.register.write(RegisterField::A, value);
    }

    fn pha(&mut self) {
        self.stack_push(self.register.read(RegisterField::A))
    }

    fn plp(&mut self) {
        let new_status = self.stack_pop();
        self.register.write(RegisterField::STATUS, new_status);
        self.register.status.remove(CpuFlags::BREAK);
        self.register.status.insert(CpuFlags::BREAK2);
    }

    fn php(&mut self) {
        let mut flags = self.register.status.clone();
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(flags.bits());
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.add_to_register_a(data);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
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

    fn arithmetic_shift<F>(&mut self, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, bool) -> (u8, bool),
    {
        if matches!(mode, AddressingMode::Accumulator) {
            self.arithmetic_accumulator(&op);
        } else {
            self.arithmetic_mem(mode, op);
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

    fn arithmetic_mem<F>(&mut self, mode: &AddressingMode, op: F)
    where
        F: Fn(u8, bool) -> (u8, bool),
    {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let carry = self.register.status.contains(CpuFlags::CARRY);

        let (data, carry) = op(data, carry);
        self.register.status.set(CpuFlags::CARRY, carry);

        self.mem_write(addr, data);
        self.register.update_zero_and_negative_flags(data);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        let mask = self.register.read(RegisterField::A) & data;
        self.register.status.set(CpuFlags::ZERO, mask == 0);

        self.register
            .status
            .set(CpuFlags::NEGATIVE, data & 0b1000_0000 > 0);
        self.register
            .status
            .set(CpuFlags::OVERFLOW, data & 0b0100_0000 > 0);
    }

    fn sax(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.register.read(RegisterField::X) & self.register.read(RegisterField::A);
        self.mem_write(addr, data);
    }

    fn isb(&mut self, mode: &AddressingMode) {
        self.increment_memory(mode);
        self.sbc(mode);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        self.arithmetic_shift(mode, asl);
        self.logic(mode, |a, b| a | b);
    }
    fn rla(&mut self, mode: &AddressingMode) {
        self.arithmetic_shift(mode, rol);
        self.logic(mode, |a, b| a & b);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        self.arithmetic_shift(&mode, lsr);
        self.logic(&mode, |a, b| a ^ b);
    }

    fn rra(&mut self, mode: &AddressingMode) {
        self.arithmetic_shift(&mode, ror);
        self.adc(&mode);
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            self.bus.tick(1);

            let jump: i8 = self.mem_read(self.register.pc) as i8;
            let jump_addr = self.register.pc.wrapping_add(1).wrapping_add(jump as u16);

            if page_cross(self.register.pc.wrapping_add(1), jump_addr) {
                self.bus.tick(1);
            }

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

    fn rti(&mut self) {
        self.plp();
        self.register.pc = self.stack_pop_u16();
    }

    fn rts(&mut self) {
        let addr = self.stack_pop_u16() + 1;
        self.register.pc = addr;
    }

    fn page_crossed(&mut self, mode: &AddressingMode) -> bool {
        let addr = self.register.pc;

        match mode {
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::X) as u16);
                page_cross(base, addr)
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                page_cross(base, addr)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(addr);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                page_cross(deref, deref_base)
            }
            _ => false,
        }
    }

    pub fn get_absolute_address(&mut self, mode: &AddressingMode, addr: u16) -> u16 {
        match mode {
            AddressingMode::ZeroPage => self.mem_read(addr) as u16,

            AddressingMode::Absolute => self.mem_read_u16(addr),

            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register.read(RegisterField::X)) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register.read(RegisterField::Y)) as u16;
                addr
            }

            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::X) as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                addr
            }

            AddressingMode::Indirect_X => {
                let base = self.mem_read(addr);

                let ptr: u8 = (base as u8).wrapping_add(self.register.read(RegisterField::X));
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(addr);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register.read(RegisterField::Y) as u16);
                deref
            }

            _ => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register.pc,
            _ => self.get_absolute_address(mode, self.register.pc),
        }
    }

    fn interrupt_nmi(&mut self) {
        self.stack_push_u16(self.register.pc);
        let mut flag = self.register.status.clone();
        flag.set(CpuFlags::BREAK, false);
        flag.set(CpuFlags::BREAK2, true);

        self.stack_push(flag.bits());
        self.register.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.bus.tick(2);
        self.register.pc = self.mem_read_u16(VECTOR_NMI_INTERRUPT_HANDLER);
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
    use crate::bus::Bus;
    use crate::cpu::{CpuFlags, CPU};
    use crate::opcodes;
    use crate::opcodes::AddressingMode;
    use crate::register::{RegisterField, STACK_RESET};
    use core::mem::Mem;
    use ppu::PPU;

    fn create() -> CPU<'static> {
        let ppu = PPU::new_empty_rom();
        let bus = Bus::new(ppu);
        return CPU::new(bus);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = create();
        cpu.eval(&[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x05);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = create();
        cpu.eval(&[0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xa5_lda_immediate_load_data() {
        let mut cpu = create();
        cpu.mem_write(0x10, 0x55);
        cpu.eval(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = create();
        cpu.mem_write(0x10, 0x00);
        cpu.eval(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = create();
        cpu.mem_write_u16(0x1020, 0x55);
        cpu.eval(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x55);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = create();
        cpu.mem_write_u16(0x1020, 0x00);
        cpu.eval(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = create();
        cpu.eval(&[0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xc1)
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu = create();
        cpu.eval(&[0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 1)
    }

    #[test]
    fn test_0xc8_iny_overflow() {
        let mut cpu = create();
        cpu.eval(&[0xA0, 0xff, 0xaa, 0xC8, 0xC8, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 1)
    }

    #[test]
    fn test_0xe6_inc() {
        let mut cpu = create();
        cpu.mem_write(0xCA, 0x02);
        cpu.eval(&[0xE6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x03);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec() {
        let mut cpu = create();
        cpu.mem_write(0xCA, 0x02);
        cpu.eval(&[0xC6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec_to_zero() {
        let mut cpu = create();
        cpu.mem_write(0xCA, 0x02);
        cpu.eval(&[0xC6, 0xCA, 0xC6, 0xCA, 0x00]);
        assert_eq!(cpu.mem_read(0xCA), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xca_dex_underflow() {
        let mut cpu = create();
        cpu.eval(&[0xCA, 0xCA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x88_dey_underflow() {
        let mut cpu = create();
        cpu.eval(&[0x88, 0x88, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 254);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x85_sta_write_accum_to_memory() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0xBA, 0x85, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x86_stx_write_x_reg_to_memory() {
        let mut cpu = create();
        cpu.eval(&[0xA2, 0xBA, 0x86, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0x84_sty_write_y_reg_to_memory() {
        let mut cpu = create();
        cpu.eval(&[0xA0, 0xBA, 0x84, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::Y), 0xBA);
        assert_eq!(cpu.mem_read(0xAA), 0xBA);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = create();
        cpu.eval(&[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::X),
            cpu.register.read(RegisterField::A)
        );
    }

    #[test]
    fn test_0xaa_txa_move_x_to_a() {
        let mut cpu = create();
        cpu.eval(&[0xa2, 0x10, 0x8a, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::A),
            cpu.register.read(RegisterField::X)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_tya_move_y_to_a() {
        let mut cpu = create();
        cpu.eval(&[0xa0, 0x10, 0x98, 0x00]);
        assert_eq!(
            cpu.register.read(RegisterField::Y),
            cpu.register.read(RegisterField::A)
        );
        assert_eq!(cpu.register.read(RegisterField::A), 0x10);
    }

    #[test]
    fn test_0xaa_txs_move_x_to_sp() {
        let mut cpu = create();
        cpu.eval(&[0xA2, 0x10, 0x9A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), cpu.register.sp);
        assert_eq!(cpu.register.sp, 0x10);
    }

    #[test]
    fn test_0xaa_tsx_move_sp_to_x() {
        let mut cpu = create();
        cpu.eval(&[0xBA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::X), STACK_RESET);
    }

    #[test]
    fn test_0x38_set_carry_flag() {
        let mut cpu = create();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.eval(&[0x38, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xf8_set_decimal_flag() {
        let mut cpu = create();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.eval(&[0xf8, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x78_set_interrupt_disable_flag() {
        let mut cpu = create();
        cpu.eval(&[0x78, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }

    #[test]
    fn test_0x18_clear_carry_flag() {
        let mut cpu = create();
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        cpu.eval(&[0x38, 0x18, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xd8_clear_decimal_flag() {
        let mut cpu = create();
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
        cpu.eval(&[0xf8, 0xd8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::DECIMAL_MODE));
    }

    #[test]
    fn test_0x58_clear_interrupt_disable_flag() {
        let mut cpu = create();
        cpu.eval(&[0x78, 0x58, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }

    #[test]
    fn test_0xb8_clear_overflow_flag() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xF0);
        cpu.eval(&[0xA9, 0x70, 0x24, 0xAA, 0xB8, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_overflow() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0x70);
        cpu.eval(&[0xA9, 0x70, 0x24, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_zero() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0x0F);
        cpu.eval(&[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x24_bit_test_should_only_set_negative() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xB0);
        cpu.eval(&[0xA9, 0xF0, 0x24, 0xAA, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x29_logical_and_on_immediate() {
        let mut cpu = create();
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.eval(&[0xA9, 0xAA, 0x29, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x2d_logical_and_on_absolute() {
        let mut cpu = create();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 & 0b0111 = 0b0000_0010 = 0x02
        cpu.eval(&[0xA9, 0xAA, 0x2D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
    }

    #[test]
    fn test_0x49_eor_exclusive_or_on_immediate() {
        let mut cpu = create();
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.eval(&[0xA9, 0xAA, 0x49, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x5d_eor_exclusive_or_on_absolute() {
        let mut cpu = create();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 ^ 0b0111 = 0b1010_1101 = 0xAD
        cpu.eval(&[0xA9, 0xAA, 0x5D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAD);
    }

    #[test]
    fn test_0x09_ora_logical_eor_on_immediate() {
        let mut cpu = create();
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.eval(&[0xA9, 0xAA, 0x09, 0x07, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x0d_ora_exclusive_or_on_absolute() {
        let mut cpu = create();
        cpu.mem_write(0x1234, 0x07);
        // 0b1010_1010 | 0b0111 = 0b1010_1101 = 0xAF
        cpu.eval(&[0xA9, 0xAA, 0x0D, 0x34, 0x12, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xAF);
    }

    #[test]
    fn test_0x69_adc_no_overflow_no_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x02, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x04);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_overflow_carry_bit_set() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0xFF, 0x69, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x01);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_zero() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0xFF, 0x69, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x69_adc_sign_bit_incorrect() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x80, 0x69, 0x80, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_no_overflow() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x08, 0xE9, 0x04, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_overflow_carry_bit_set() {
        let mut cpu = create();
        cpu.eval(&[0x18, 0xA9, 0x80, 0xE9, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x7E);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_zero() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x01, 0x38, 0xE9, 0x01, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(!cpu.register.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(!cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xe9_sbc_sign_bit_incorrect() {
        let mut cpu = create();
        cpu.eval(&[0x18, 0xA9, 0x01, 0xE9, 0x02, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xFE);
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0x0a_asl_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x81, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x0a_asl_no_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x41, 0x0A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x82);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x06_asl_update_memory_and_set_carry() {
        let mut cpu = create();
        cpu.mem_write(0x40, 0x81);
        cpu.eval(&[0x06, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x02);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x81, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x40);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4a_lsr_no_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x40, 0x4A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x46_lsr_update_memory_and_set_carry() {
        let mut cpu = create();
        cpu.mem_write(0x40, 0x81);
        cpu.eval(&[0x46, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x40);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x81, 0x2A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x02);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol_no_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x40, 0x2A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x80);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2e_rol_update_memory_and_set_carry() {
        let mut cpu = create();
        cpu.mem_write(0x40, 0x81);
        cpu.eval(&[0x2E, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x02);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x81, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x40);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_no_carry() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x40, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_carry_flag_already_set() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x40, 0x38, 0x6A, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xA0);
        assert!(!cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_ror_update_memory_and_set_carry() {
        let mut cpu = create();
        cpu.mem_write(0x40, 0x81);
        cpu.eval(&[0x6E, 0x40, 0x00]);
        assert_eq!(cpu.mem_read(0x40), 0x40);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_equal() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0xAA, 0xC9, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc9_cmp_gt_eq() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0xFF, 0xC9, 0x00, 0x00]);
        assert!(!cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
        assert!(cpu.register.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_0xc5_cmp_equal() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xF0);
        cpu.eval(&[0xA9, 0xF0, 0xC5, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xe0_cpx() {
        let mut cpu = create();
        cpu.eval(&[0xA2, 0xAA, 0xE0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xec_cpx() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xF0);
        cpu.eval(&[0xA2, 0xF0, 0xEC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xc0_cpy() {
        let mut cpu = create();
        cpu.eval(&[0xA0, 0xAA, 0xC0, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0xcc_cpy() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xF0);
        cpu.eval(&[0xA0, 0xF0, 0xCC, 0xAA, 0x00]);
        assert!(cpu.register.status.contains(CpuFlags::ZERO));
        assert!(cpu.register.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x90_bcc_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x90, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xb0_bcs_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xB0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(cpu.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0xf0_beq_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xF0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x30_bmi_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x30, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0xd0_bne_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0xD0, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x03);
        assert_eq!(cpu.mem_read(0x0201), 0x03);
    }

    #[test]
    fn test_0x10_bpl_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x10, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x02);
        assert_eq!(cpu.mem_read(0x0201), 0x02);
    }

    #[test]
    fn test_0x50_bvc_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xA9, 0xF0, 0x85, 0x44, 0xCA, 0x24, 0x44, 0xE0, 0x03, 0x50, 0xF9, 0x8E,
            0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x70_bvs_loop() {
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x08, 0xCA, 0x8E, 0x00, 0x02, 0xE0, 0x03, 0x70, 0xF8, 0x8E, 0x01, 0x02, 0x00,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x07);
        assert_eq!(cpu.mem_read(0x0201), 0x07);
    }

    #[test]
    fn test_0x4c_jmp_absolute() {
        let mut cpu = create();
        cpu.eval(&[
            0xA9, 0x03, 0x4C, 0x08, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(cpu.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect() {
        let mut cpu = create();
        cpu.mem_write_u16(0x0610, 0x0608);
        cpu.eval(&[
            0xA9, 0x03, 0x6C, 0x10, 0x06, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x03);
        assert_eq!(cpu.mem_read(0x0200), 0x03);
    }

    #[test]
    fn test_0x6c_jmp_indirect_6502_bug() {
        let mut cpu = create();
        cpu.mem_write(0x08FF, 0x08);
        cpu.mem_write(0x0800, 0x06);
        cpu.eval(&[
            0xA9, 0x03, 0x6C, 0xFF, 0x08, 0x00, 0x00, 0x00, 0x8D, 0x00, 0x02,
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
        let mut cpu = create();
        cpu.eval(&[
            0x20, 0x10, 0x06, 0x20, 0x0A, 0x06, 0x20, 0x09, 0x06, 0x00, 0xE8, 0xE0, 0x05, 0xD0,
            0xFB, 0x60, 0xA2, 0x00, 0x60,
        ]);
        assert_eq!(cpu.register.read(RegisterField::X), 0x05);
        // end: is a subroutine, so stack isn't completely reset
        assert_eq!(cpu.register.sp, STACK_RESET - 2);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut cpu = create();
        cpu.stack_push_u16(0xCAFE);
        cpu.stack_push_u16(0xAABB);
        cpu.stack_push_u16(0xCCDD);
        assert_eq!(cpu.stack_pop_u16(), 0xCCDD);
        assert_eq!(cpu.stack_pop_u16(), 0xAABB);
        assert_eq!(cpu.stack_pop_u16(), 0xCAFE);
    }

    #[test]
    fn test_0x48_pha() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x20, 0x48, 0x00]);
        assert_eq!(cpu.stack_pop(), 0x20);
    }

    #[test]
    fn test_0x08_php() {
        let mut cpu = create();
        cpu.eval(&[0x08, 0x00]);
        assert_eq!(cpu.stack_pop(), 0b110100);
    }

    #[test]
    fn test_0x68_pla() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x20, 0x48, 0xA9, 0x30, 0x68, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x20);
    }

    #[test]
    fn test_0x28_plp() {
        let mut cpu = create();
        /*
           SEC
           PHP
           SEI
           PLP
        */
        cpu.eval(&[0x38, 0x08, 0x78, 0x28, 0x00]);
        assert_eq!(cpu.register.status.bits(), 0b100101);
    }

    #[test]
    fn test_0x28_plp_sets_correct_flags() {
        let mut cpu = create();
        /*
           LDA #$FF
           PHA
           PLP
        */
        cpu.eval(&[0xA9, 0xFF, 0x48, 0x28, 0x00]);
        assert_eq!(cpu.register.status.bits(), 0xEF);
    }

    #[test]
    fn test_0xaf_lax() {
        let mut cpu = create();
        cpu.mem_write(0xAA, 0xBB);
        cpu.eval(&[0xAF, 0xAA, 0x00]);
        assert_eq!(cpu.register.read(RegisterField::A), 0xBB);
        assert_eq!(cpu.register.read(RegisterField::X), 0xBB);
        assert_eq!(cpu.register.read(RegisterField::Y), 0x00);
    }

    #[test]
    fn test_0x83_sax_should_not_affect_flags() {
        let mut cpu = create();
        cpu.eval(&[0xA9, 0x04, 0xA2, 0x02, 0x83, 0x49, 0x00]);

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
        let mut cpu = create();
        cpu.eval(&[
            0xA2, 0x00, 0xA0, 0x00, 0x8A, 0x99, 0x00, 0x02, 0x48, 0xE8, 0xC8, 0xC0, 0x10, 0xD0,
            0xF5, 0x68, 0x99, 0x00, 0x02, 0xC8, 0xC0, 0x20, 0xD0, 0xF7,
        ]);
        assert_eq!(cpu.register.read(RegisterField::A), 0x00);
        assert_eq!(cpu.register.read(RegisterField::X), 0x10);
        assert_eq!(cpu.register.read(RegisterField::Y), 0x20);
        assert_eq!(cpu.mem_read(0x0200), 0x00);
        assert_eq!(cpu.mem_read(0x0201), 0x01);
        assert_eq!(cpu.mem_read(0x0210), 0x0F);
    }

    #[test]
    fn test_all_official_operations_implemented() {
        let mut cpu = create();
        let ref opcodes = *opcodes::CPU_OPCODES;

        for op in opcodes {
            if op.unofficial_name == None {
                cpu.eval(&[op.code, 0x00, 0x00, 0x00, 0x00]);
            }
        }
    }

    #[test]
    fn test_immediate_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x200;
        let value = cpu.get_operand_address(&AddressingMode::Immediate);
        assert_eq!(cpu.register.pc, value);
    }

    #[test]
    fn test_zero_page_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write(0x10, 0x42);
        assert_eq!(cpu.get_operand_address(&AddressingMode::ZeroPage), 0x42);
    }

    #[test]
    fn test_absolute_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write_u16(0x10, 0x1234);
        assert_eq!(cpu.get_operand_address(&AddressingMode::Absolute), 0x1234);
    }

    #[test]
    fn test_zero_page_x_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write(0x10, 0x10);
        cpu.register.write(RegisterField::X, 0x32);
        assert_eq!(cpu.get_operand_address(&AddressingMode::ZeroPage_X), 0x42);
    }

    #[test]
    fn test_zero_page_y_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write(0x10, 0x10);
        cpu.register.write(RegisterField::Y, 0x22);
        assert_eq!(cpu.get_operand_address(&AddressingMode::ZeroPage_Y), 0x32);
    }

    #[test]
    fn test_absolute_x_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write_u16(0x10, 0x1234);
        cpu.register.write(RegisterField::X, 0x05);
        assert_eq!(cpu.get_operand_address(&AddressingMode::Absolute_X), 0x1239);
    }

    #[test]
    fn test_absolute_y_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write_u16(0x10, 0x1000);
        cpu.register.write(RegisterField::Y, 0x05);
        assert_eq!(cpu.get_operand_address(&AddressingMode::Absolute_Y), 0x1005);
    }

    #[test]
    fn test_indirect_x_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write(0x10, 0x80);
        cpu.register.write(RegisterField::X, 0x05);
        cpu.mem_write_u16(0x85, 0x2000);

        assert_eq!(cpu.get_operand_address(&AddressingMode::Indirect_X), 0x2000);
    }

    #[test]
    fn test_indirect_y_mode() {
        let mut cpu = create();
        cpu.register.pc = 0x10;
        cpu.mem_write(0x10, 0x50);
        cpu.mem_write_u16(0x50, 0x2000);
        cpu.register.write(RegisterField::Y, 0x05);

        assert_eq!(cpu.get_operand_address(&AddressingMode::Indirect_Y), 0x2005);
    }

    #[test]
    #[should_panic]
    fn test_get_operand_address_invalid_mode_should_panic() {
        create().get_operand_address(&AddressingMode::Accumulator);
    }
}
