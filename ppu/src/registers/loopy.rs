#[repr(C)]
pub union LoopyRegister {
    // Credit to Loopy for working this out :D
    bits: u16,
    data: LoopyRegisterData,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LoopyRegisterData {
    coarse_x: u16,      // 5 bits
    coarse_y: u16,      // 5 bits
    nametable_x: u16,   // 1 bit
    nametable_y: u16,   // 1 bit
    fine_y: u16,        // 3 bits
    unused: u16,        // 1 bit
}

impl LoopyRegister {
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn set_bits(&mut self, value: u16) {
        self.bits = value;
    }

    pub fn get_bits(&self) -> u16 {
        unsafe {
            self.bits
        }
    }

    pub fn set_coarse_x(&mut self, value: u16) {
        self.data.coarse_x = value & 0x1F;
    }

    pub fn get_coarse_x(&self) -> u16 {
        unsafe {
            self.data.coarse_x
        }
    }

    pub fn set_coarse_y(&mut self, value: u16) {
        self.data.coarse_y = value & 0x1F;
    }

    pub fn get_coarse_y(&self) -> u16 {
        unsafe {
            self.data.coarse_y
        }
    }

    pub fn set_nametable_x(&mut self, value: u16) {
        self.data.nametable_x = value & 0x01;
    }

    pub fn get_nametable_x(&self) -> u16 {
        unsafe {
            self.data.nametable_x
        }
    }

    pub fn set_nametable_y(&mut self, value: u16) {
        self.data.nametable_y = value & 0x01;
    }

    pub fn get_nametable_y(&self) -> u16 {
        unsafe {
            self.data.nametable_y
        }
    }

    pub fn set_fine_y(&mut self, value: u16) {
        self.data.fine_y = value & 0x07;
    }

    pub fn get_fine_y(&self) -> u16 {
        unsafe {
            self.data.fine_y
        }
    }

    pub fn set_unused(&mut self, value: u16) {
        self.data.unused = value & 0x01;
    }

    pub fn get_unused(&self) -> u16 {
        unsafe {
            self.data.unused
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let register = LoopyRegister::new();
        assert_eq!(register.get_bits(), 0x0000);
    }

    #[test]
    fn test_set_bits() {
        let mut register = LoopyRegister::new();
        register.set_bits(0xABCD);
        assert_eq!(register.get_bits(), 0xABCD);
    }

    #[test]
    fn test_set_coarse_x() {
        let mut register = LoopyRegister::new();
        register.set_coarse_x(0x1F);
        assert_eq!(register.get_coarse_x(), 0x1F);
        register.set_coarse_x(0x20);
        assert_eq!(register.get_coarse_x(), 0x00);
    }

    #[test]
    fn test_set_coarse_y() {
        let mut register = LoopyRegister::new();
        register.set_coarse_y(0x1F);
        assert_eq!(register.get_coarse_y(), 0x1F);
        register.set_coarse_y(0x20);
        assert_eq!(register.get_coarse_y(), 0x00);
    }

    #[test]
    fn test_set_nametable_x() {
        let mut register = LoopyRegister::new();
        register.set_nametable_x(0x01);
        assert_eq!(register.get_nametable_x(), 0x01);
        register.set_nametable_x(0x02);
        assert_eq!(register.get_nametable_x(), 0x01);
    }

    #[test]
    fn test_set_nametable_y() {
        let mut register = LoopyRegister::new();
        register.set_nametable_y(0x01);
        assert_eq!(register.get_nametable_y(), 0x01);
        register.set_nametable_y(0x02);
        assert_eq!(register.get_nametable_y(), 0x01);
    }

    #[test]
    fn test_set_fine_y() {
        let mut register = LoopyRegister::new();
        register.set_fine_y(0x07);
        assert_eq!(register.get_fine_y(), 0x07);
        register.set_fine_y(0x08);
        assert_eq!(register.get_fine_y(), 0x00);
    }

    #[test]
    fn test_set_unused() {
        let mut register = LoopyRegister::new();
        register.set_unused(0x01);
        assert_eq!(register.get_unused(), 0x01);
        register.set_unused(0x02);
        assert_eq!(register.get_unused(), 0x01);
    }
}