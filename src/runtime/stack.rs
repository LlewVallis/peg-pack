pub struct Stack<T> {
    values: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn of(value: T) -> Self {
        let mut result = Self::new();
        result.push(value);
        result
    }

    pub fn top(&self) -> Option<&T> {
        self.values.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut T> {
        self.values.last_mut()
    }

    pub fn push(&mut self, value: T) {
        self.values.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.values.pop()
    }
}
