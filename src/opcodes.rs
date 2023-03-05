use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
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
    Accumulator,
}

pub fn is_addressing_accumulator(mode: AddressingMode) -> bool {
    matches!(mode, AddressingMode::Accumulator)
}

#[derive(Debug)]
pub enum Instruction {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
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
        OpCode::new(0x69, Instruction::ADC, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, Instruction::ADC, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, Instruction::ADC, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x6D, Instruction::ADC, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7D, Instruction::ADC, 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x79, Instruction::ADC, 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0x61, Instruction::ADC, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x71, Instruction::ADC, 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0x29, Instruction::AND, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, Instruction::AND, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, Instruction::AND, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x2D, Instruction::AND, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3D, Instruction::AND, 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x39, Instruction::AND, 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0x21, Instruction::AND, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x31, Instruction::AND, 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0x0A, Instruction::ASL, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x06, Instruction::ASL, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x16, Instruction::ASL, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x0E, Instruction::ASL, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1E, Instruction::ASL, 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x90, Instruction::BCC, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0xB0, Instruction::BCS, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0xF0, Instruction::BEQ, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0x30, Instruction::BMI, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0xD0, Instruction::BNE, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0x10, Instruction::BPL, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0x50, Instruction::BVC, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),
        OpCode::new(0x70, Instruction::BVS, 2, 2 /* +1 if branch succeeds, +2 if to a new page */, AddressingMode::NoneAddressing),

        OpCode::new(0x24, Instruction::BIT, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2C, Instruction::BIT, 3, 4, AddressingMode::Absolute),

        OpCode::new(0x00, Instruction::BRK, 1, 7, AddressingMode::NoneAddressing),

        OpCode::new(0x18, Instruction::CLC, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xD8, Instruction::CLD, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x58, Instruction::CLI, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xB8, Instruction::CLV, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0xC9, Instruction::CMP, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xC5, Instruction::CMP, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xD5, Instruction::CMP, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xCD, Instruction::CMP, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xDD, Instruction::CMP, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0xD9, Instruction::CMP, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0xC1, Instruction::CMP, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xD1, Instruction::CMP, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0xE0, Instruction::CPX, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xE4, Instruction::CPX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xEC, Instruction::CPX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xC0, Instruction::CPY, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xC4, Instruction::CPY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xCC, Instruction::CPY, 3, 4, AddressingMode::Absolute),

        OpCode::new(0xC6, Instruction::DEC, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xD6, Instruction::DEC, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xCE, Instruction::DEC, 3, 6, AddressingMode::Absolute),
        OpCode::new(0xDE, Instruction::DEC, 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0xCA, Instruction::DEX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x88, Instruction::DEY, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0x49, Instruction::EOR, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x45, Instruction::EOR, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x55, Instruction::EOR, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x4D, Instruction::EOR, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x5D, Instruction::EOR, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0x59, Instruction::EOR, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0x41, Instruction::EOR, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x51, Instruction::EOR, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0xE6, Instruction::INC, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0xF6, Instruction::INC, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0xEE, Instruction::INC, 3, 6, AddressingMode::Absolute),
        OpCode::new(0xFE, Instruction::INC, 3, 7, AddressingMode::Absolute_X),
        OpCode::new(0xE8, Instruction::INX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xC8, Instruction::INY, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0x4C, Instruction::JMP, 3, 3, AddressingMode::Absolute),
        OpCode::new(0x6C, Instruction::JMP, 3, 5, AddressingMode::NoneAddressing), // Indirect
        OpCode::new(0x20, Instruction::JSR, 3, 6, AddressingMode::Absolute),

        OpCode::new(0xA9, Instruction::LDA, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA5, Instruction::LDA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB5, Instruction::LDA, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xAD, Instruction::LDA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBD, Instruction::LDA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0xB9, Instruction::LDA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0xA1, Instruction::LDA, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xB1, Instruction::LDA, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0xA2, Instruction::LDX, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA6, Instruction::LDX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB6, Instruction::LDX, 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0xAE, Instruction::LDX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBE, Instruction::LDX, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),

        OpCode::new(0xA0, Instruction::LDY, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA4, Instruction::LDY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB4, Instruction::LDY, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xAC, Instruction::LDY, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBC, Instruction::LDY, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),

        OpCode::new(0x4A, Instruction::LSR, 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x46, Instruction::LSR, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x56, Instruction::LSR, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x4E, Instruction::LSR, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x5E, Instruction::LSR, 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0xEA, Instruction::NOP, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0x09, Instruction::ORA, 2, 2, AddressingMode::Immediate),
        OpCode::new(0x05, Instruction::ORA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x15, Instruction::ORA, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x0D, Instruction::ORA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x1D, Instruction::ORA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0x19, Instruction::ORA, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0x01, Instruction::ORA, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x11, Instruction::ORA, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0x48, Instruction::PHA, 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x08, Instruction::PHP, 1, 3, AddressingMode::NoneAddressing),
        OpCode::new(0x68, Instruction::PLA, 1, 4, AddressingMode::NoneAddressing),
        OpCode::new(0x28, Instruction::PLP, 1, 4, AddressingMode::NoneAddressing),

        OpCode::new(0x2A, Instruction::ROL, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x26, Instruction::ROL, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x36, Instruction::ROL, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x2E, Instruction::ROL, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x3E, Instruction::ROL, 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x6A, Instruction::ROR, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x66, Instruction::ROR, 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x76, Instruction::ROR, 2, 6, AddressingMode::ZeroPage_X),
        OpCode::new(0x6E, Instruction::ROR, 3, 6, AddressingMode::Absolute),
        OpCode::new(0x7E, Instruction::ROR, 3, 7, AddressingMode::Absolute_X),

        OpCode::new(0x40, Instruction::RTI, 1, 6, AddressingMode::NoneAddressing),
        OpCode::new(0x60, Instruction::RTS, 1, 6, AddressingMode::NoneAddressing),

        OpCode::new(0xE9, Instruction::SBC, 2, 2, AddressingMode::Immediate),
        OpCode::new(0xE5, Instruction::SBC, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xF5, Instruction::SBC, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xED, Instruction::SBC, 3, 4, AddressingMode::Absolute),
        OpCode::new(0xFD, Instruction::SBC, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_X),
        OpCode::new(0xF9, Instruction::SBC, 3, 4 /* +1 on page cross */, AddressingMode::Absolute_Y),
        OpCode::new(0xE1, Instruction::SBC, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xF1, Instruction::SBC, 2, 5 /* +1 on page cross */, AddressingMode::Indirect_Y),

        OpCode::new(0x38, Instruction::SEC, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xF8, Instruction::SED, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x78, Instruction::SEI, 1, 2, AddressingMode::NoneAddressing),

        OpCode::new(0x85, Instruction::STA, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, Instruction::STA, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x8D, Instruction::STA, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9D, Instruction::STA, 3, 5, AddressingMode::Absolute_X),
        OpCode::new(0x99, Instruction::STA, 3, 5, AddressingMode::Absolute_Y),
        OpCode::new(0x81, Instruction::STA, 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x91, Instruction::STA, 2, 6, AddressingMode::Indirect_Y),

        OpCode::new(0x86, Instruction::STX, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x96, Instruction::STX, 2, 4, AddressingMode::ZeroPage_Y),
        OpCode::new(0x8E, Instruction::STX, 3, 4, AddressingMode::Absolute),
        OpCode::new(0x84, Instruction::STY, 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x94, Instruction::STY, 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x8C, Instruction::STY, 3, 4, AddressingMode::Absolute),

        OpCode::new(0xAA, Instruction::TAX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xA8, Instruction::TAY, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0xBA, Instruction::TSX, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x8A, Instruction::TXA, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x9A, Instruction::TXS, 1, 2, AddressingMode::NoneAddressing),
        OpCode::new(0x98, Instruction::TYA, 1, 2, AddressingMode::NoneAddressing),

    ];

    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for cpuop in &*CPU_OPCODES {
            if map.contains_key(&cpuop.code) {
                panic!("Duplicate opcode {:x}", cpuop.code)
            }

            map.insert(cpuop.code, cpuop);
        }
        map
    };
}
