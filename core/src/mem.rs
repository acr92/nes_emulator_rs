pub trait Mem {
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

#[cfg(test)]
mod test {
    use crate::mem::Mem;

    struct Test {
        memory: [u8; 32],
    }

    impl Test {
        fn new() -> Self {
            Test { memory: [0; 32] }
        }
    }

    impl Mem for Test {
        fn mem_read(&self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        fn mem_write(&mut self, addr: u16, value: u8) {
            self.memory[addr as usize] = value
        }
    }

    #[test]
    fn test_mem_write_u16_and_mem_read_u16() {
        let mut test = Test::new();
        test.mem_write_u16(0x10, 0x1122);
        assert_eq!(test.mem_read_u16(0x10), 0x1122);
    }
}
