use core::bus::Bus;
use core::mem::Mem;

pub(crate) struct MockBus {
    memory: [u8; 0xFFFF],
}

impl MockBus {
    pub fn new() -> Self {
        MockBus {
            memory: [0; 0xFFFF],
        }
    }
}

impl Mem for MockBus {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value
    }
}

impl Bus<'static> for MockBus {
    fn tick(&mut self) {
        // Do nothing
    }

    fn poll_nmi_status(&mut self) -> Option<u8> {
        None
    }

    fn get_clock_cycles_for_peripheral(&self, _: core::bus::BusPeripheral) -> usize {
        123456
    }
}
