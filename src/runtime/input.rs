pub trait Input {
    fn get(&self, position: usize) -> Option<u8>;

    fn len(&self) -> usize;
}

impl Input for [u8] {
    fn get(&self, position: usize) -> Option<u8> {
        self.get(position).copied()
    }

    fn len(&self) -> usize {
        self.len()
    }
}
