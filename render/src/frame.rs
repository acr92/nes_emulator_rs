pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;
    pub const RGB_SIZE: usize = 3;

    pub fn new() -> Self {
        Frame {
            data: vec![0; (Frame::WIDTH) * (Frame::HEIGHT) * Frame::RGB_SIZE],
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * Frame::RGB_SIZE * Frame::WIDTH + x * Frame::RGB_SIZE;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame::new()
    }
}
