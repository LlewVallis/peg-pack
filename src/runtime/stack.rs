use std::mem;

pub struct Stack<T> {
    top: T,
    elements: Vec<T>,
}

impl<T> Stack<T> {
    pub fn of(value: T) -> Self {
        Self {
            top: value,
            elements: Vec::new(),
        }
    }

    pub unsafe fn top(&self) -> &T {
        &self.top
    }

    pub unsafe fn top_mut(&mut self) -> &mut T {
        &mut self.top
    }

    pub fn push(&mut self, value: T) {
        unsafe {
            let old_top = mem::replace(self.top_mut(), value);
            self.elements.push(old_top);
        }
    }

    pub unsafe fn pop(&mut self) -> T {
        let next = self.elements.pop().unwrap_unchecked();
        mem::replace(self.top_mut(), next)
    }
}
