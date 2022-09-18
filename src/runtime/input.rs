pub trait Input {
    fn get(&self, position: u32) -> Option<u8>;

    fn len(&self) -> u32;
}

impl Input for [u8] {
    fn get(&self, position: u32) -> Option<u8> {
        self.get(position as usize).copied()
    }

    fn len(&self) -> u32 {
        self.len() as u32
    }
}
