pub struct Rectangle {
    pub x1: usize,
    pub y1: usize,
    pub x2: usize,
    pub y2: usize,
}

impl Rectangle {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rectangle { x1, x2, y1, y2 }
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Rectangle::new(0, 0, 0, 0)
    }
}
