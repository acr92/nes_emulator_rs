use lazy_static::lazy_static;
use std::collections::HashMap;

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

#[derive(Debug)]
pub enum Instruction {
    BRK,
    TAX,
    INX,
    LDA,
    STA,
}

pub struct OpCode {
    pub code: u8,
    pub instruction: Instruction,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    fn new(code: u8, instruction: Instruction, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code,
            instruction,
            len,
            cycles,
            mode,
        }
    }
}

lazy_static! {
    pub static ref CPU_OPCODES: Vec<OpCode> = vec![
        OpCode::new(0x00, Instruction::BRK, 1, 7, AddressingMode::NoneAddressing),
        OpCode::new(0xAA, Instruction::TAX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xE8, Instruction::INX, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xA9, Instruction::LDA, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA5, Instruction::LDA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB5, Instruction::LDA, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xAD, Instruction::LDA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBD, Instruction::LDA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0xB9, Instruction::LDA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0xA1, Instruction::LDA, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xB1, Instruction::LDA, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0x85, Instruction::STA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, Instruction::STA, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x8D, Instruction::STA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9D, Instruction::STA, 3, 5, AddressingMode::Absolute_X),
        OpCode::new(0x99, Instruction::STA, 3, 5, AddressingMode::Absolute_Y),
        OpCode::new(0x81, Instruction::STA, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x91, Instruction::STA, 2, 6, AddressingMode::Indirect_Y),
    ];

    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for cpuop in &*CPU_OPCODES {
            map.insert(cpuop.code, cpuop);
        }
        map
    };
}
