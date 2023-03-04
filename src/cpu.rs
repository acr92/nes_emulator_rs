pub struct Register {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub status: u8,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

pub struct CPU {
    pub register: Register,
    memory: [u8; 0xFFFF],
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register: Register {
                a: 0,
                x: 0,
                y: 0,
                pc: 0,
                status: 0,
            },
            memory: [0; 0xFFFF],
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_read_u16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr.wrapping_add(1)) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }

    fn mem_write_u16(&mut self, addr: u16, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr.wrapping_add(1), hi);
    }

    pub fn reset(&mut self) {
        self.register = Register {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            status: 0,
        };

        self.register.pc = self.mem_read_u16(0xFFFC);
    }

    pub fn load_and_run(&mut self, program: &[u8]) {
        self.load(program);
        self.reset();
        self.run()
    }
    fn load(&mut self, program: &[u8]) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(program);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    fn run(&mut self) {
        loop {
            let opcode = self.mem_read(self.register.pc);
            self.register.pc += 1;

            match opcode {
                // LDA
                0xA9 => {
                    self.lda(AddressingMode::Immediate);
                    self.register.pc += 1;
                },
                0xA5 => {
                    self.lda(AddressingMode::ZeroPage);
                    self.register.pc += 1;
                }
                0xAD => {
                    self.lda(AddressingMode::Absolute);
                    self.register.pc += 2;
                }
                // TAX
                0xAA => {
                    self.tax();
                },
                // INX
                0xE8 => {
                    self.inx();
                }
                // BRK
                0x00 => {
                    return;
                }
                _ => { todo!("Unknown opcode {}", opcode) }
            }
        }
    }

    fn inx(&mut self) {
        self.register.x = self.register.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register.x);
    }

    fn tax(&mut self) {
        self.register.x = self.register.a;
        self.update_zero_and_negative_flags(self.register.x);
    }

    fn lda(&mut self, mode: AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register.a = value;
        self.update_zero_and_negative_flags(self.register.a);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // set Zero Flag if A = 0
        if result == 0 {
            self.register.status |= 0b0000_0010;
        } else {
            self.register.status &= 0b1111_1101;
        }

        // set Negative flag if bit 7 of A is set
        if result & 0b1000_0000 != 0 {
            self.register.status |= 0b1000_0000;
        } else {
            self.register.status &= 0b0111_1111;
        }
    }

    fn get_operand_address(&self, mode: AddressingMode) -> u16 {
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
    use crate::cpu::CPU;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register.a, 0x05);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x00, 0x00]);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xa5_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x55);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa5_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x00);
        cpu.load_and_run(&[0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x00);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xad_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x55);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x55);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b00);
        assert_eq!(cpu.register.status & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xad_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.mem_write_u16(0x1020, 0x00);
        cpu.load_and_run(&[0xad, 0x20, 0x10, 0x00]);
        assert_eq!(cpu.register.a, 0x00);
        assert_eq!(cpu.register.status & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(&[0xa9, 0x10, 0xaa, 0x00]);
        assert_eq!(cpu.register.x, cpu.register.a);
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
}