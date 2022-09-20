use std::hint::unreachable_unchecked;
use std::mem;

use super::array_vec::ArrayVec;

pub struct SmallVec<T, const N: usize> {
    data: Data<T, N>,
}

enum Data<T, const N: usize> {
    Stack(ArrayVec<T, N>),
    Heap(Vec<T>),
}

impl<T, const N: usize> SmallVec<T, N> {
    pub fn new() -> Self {
        Self {
            data: Data::Stack(ArrayVec::new())
        }
    }

    pub fn push(&mut self, value: T) {
        match &mut self.data {
            Data::Stack(stack) => unsafe {
                if stack.len() < N {
                    stack.push_unchecked(value);
                } else {
                    self.promote();
                    self.push_heap(value);
                }
            }
            Data::Heap(_) => unsafe {
                self.push_heap(value);
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        match &mut self.data {
            Data::Stack(stack) => stack.pop(),
            Data::Heap(heap) => heap.pop(),
        }
    }

    pub fn last(&self) -> Option<&T> {
        match &self.data {
            Data::Stack(stack) => stack.last(),
            Data::Heap(heap) => heap.last(),
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        match &mut self.data {
            Data::Stack(stack) => stack.last_mut(),
            Data::Heap(heap) => heap.last_mut(),
        }
    }

    unsafe fn push_heap(&mut self, value: T) {
        match &mut self.data {
            Data::Heap(heap) => heap.push(value),
            Data::Stack(_) => unreachable_unchecked(),
        }
    }

    unsafe fn promote(&mut self) {
        match &mut self.data {
            Data::Stack(stack) => {
                let mut vector = Vec::with_capacity(N * 2);

                for value in mem::replace(stack, ArrayVec::new()) {
                    vector.push(value);
                }

                self.data = Data::Heap(vector);
            },
            Data::Heap(_) => unreachable_unchecked(),
        }
    }
}
