use emulator::bus::Bus;
use emulator::cartridge::Rom;
use emulator::cpu::CPU;
use emulator::trace::trace;
use k9::assert_equal;
use ppu::PPU;

mod common;

#[test]
fn test_nestest() {
    let expected: Vec<_> = std::fs::read_to_string(test_file!("nestest.log"))
        .unwrap()
        .split("\r\n")
        .map(|a| String::from(a))
        .collect();

    let program = std::fs::read(test_file!("nestest.nes")).unwrap();
    let rom = Rom::new(&program).unwrap();

    let ppu = PPU::new_empty_rom();
    let mut bus = Bus::new(ppu);
    bus.rom = Some(Box::from(rom));
    let mut cpu = CPU::new(bus);
    cpu.reset();

    cpu.bus.cycles = 7;
    cpu.bus.ppu.cycles = 21;
    cpu.bus.ppu.scanline = 0;
    cpu.register.pc = 0xC000;

    let mut index = 0;
    cpu.run_with_callback(|cpu| {
        let actual = trace(cpu);

        // Skip the last instruction, it's just another RTS. We didn't start the program from the
        // same instruction that the nestest.log is from.
        if index == expected.len() - 1 {
            return;
        }

        if expected[index] != actual {
            dbg!(&cpu.register);
        }

        assert_equal!(expected[index], actual);
        index = index + 1;
    });
}
