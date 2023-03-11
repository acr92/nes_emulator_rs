use crate::cpu::CPU;
use crate::opcodes;
use crate::opcodes::{AddressingMode, Instruction, OpCode};
use crate::register::RegisterField;
use core::mem::Mem;

pub fn trace(cpu: &mut CPU) -> String {
    let ref opscodes = *opcodes::OPCODES_MAP;

    let code = cpu.mem_read(cpu.register.pc);
    let ops = opscodes
        .get(&code)
        .expect(format!("Unknown OP 0x{:02X}", code).as_str());

    let begin = cpu.register.pc;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match ops.mode {
        AddressingMode::Immediate
        | AddressingMode::Accumulator
        | AddressingMode::NoneAddressing => (0, 0),
        _ => {
            let addr = cpu.get_absolute_address(&ops.mode, begin + 1);
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match ops.len {
        1 => match ops.mode {
            AddressingMode::Accumulator => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.mem_read(begin + 1);
            hex_dump.push(address);

            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPage_X => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPage_Y => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Indirect_X => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.register.read(RegisterField::X))),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::Indirect_Y => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.register.read(RegisterField::Y) as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.mem_read(begin + 1);
            let address_hi = cpu.mem_read(begin + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.mem_read_u16(begin + 1);

            match ops.mode {
                AddressingMode::NoneAddressing => {
                    if ops.code == 0x6c {
                        //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_u16(address)
                        };

                        // let jmp_addr = cpu.mem_read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute if !is_jmp_instruction(ops) => {
                    format!("${:04x} = {:02x}", mem_addr, stored_value)
                }
                AddressingMode::Absolute => format!("${:04x}", mem_addr),
                AddressingMode::Absolute_X => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Absolute_Y => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");

    let instruction = if let Some(mnemonic) = ops.unofficial_name.clone() {
        mnemonic
    } else {
        format!(" {:#?}", ops.instruction)
    };

    let asm_str = format!("{:04x}  {:8} {:} {}", begin, hex_str, instruction, tmp)
        .trim()
        .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x} PPU:{:3},{:3} CYC:{}",
        asm_str,
        cpu.register.read(RegisterField::A),
        cpu.register.read(RegisterField::X),
        cpu.register.read(RegisterField::Y),
        cpu.register.status.bits(),
        cpu.register.sp,
        cpu.bus.ppu.scanline,
        cpu.bus.ppu.cycles,
        cpu.bus.cycles
    )
    .to_ascii_uppercase()
}

fn is_jmp_instruction(ops: &&OpCode) -> bool {
    match ops.instruction {
        Instruction::JMP | Instruction::JSR => true,
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;
    use crate::cpu::CPU;
    use crate::register::RegisterField;
    use core::mem::Mem;
    use ppu::PPU;

    #[test]
    fn test_format_trace() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        bus.mem_write(100, 0xa2);
        bus.mem_write(101, 0x01);
        bus.mem_write(102, 0xca);
        bus.mem_write(103, 0x88);
        bus.mem_write(104, 0x00);
        let mut cpu = CPU::new(bus);

        cpu.register.pc = 0x64;
        cpu.register.write(RegisterField::A, 1);
        cpu.register.write(RegisterField::X, 2);
        cpu.register.write(RegisterField::Y, 3);
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD PPU:  0,  0 CYC:0",
            result[0]
        );
        assert_eq!(
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD PPU:  0,  6 CYC:2",
            result[1]
        );
        assert_eq!(
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD PPU:  0, 12 CYC:4",
            result[2]
        );
    }

    #[test]
    fn test_format_mem_access() {
        let mut bus = Bus::new(PPU::new_empty_rom());
        // ORA ($33), Y
        bus.mem_write(100, 0x11);
        bus.mem_write(101, 0x33);

        //data
        bus.mem_write(0x33, 00);
        bus.mem_write(0x34, 04);

        //target cell
        bus.mem_write(0x400, 0xAA);

        let mut cpu = CPU::new(bus);
        cpu.register.pc = 0x64;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD PPU:  0,  0 CYC:0",
            result[0]
        );
    }
}
