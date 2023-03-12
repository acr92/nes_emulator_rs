use crate::mem::Mem;

#[derive(Copy, Clone)]
pub enum BusPeripheral {
    Cpu,
    Ppu,
    PpuScanlines,
    Apu,
}

pub trait Bus<'a>: Mem {
    fn tick(&mut self, cycles: u8);
    fn poll_nmi_status(&mut self) -> Option<u8>;
    fn get_clock_cycles_for_peripheral(&self, peripheral: BusPeripheral) -> usize;
}
