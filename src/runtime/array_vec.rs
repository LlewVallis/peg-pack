use std::hint::unreachable_unchecked;
use std::mem;
use std::mem::MaybeUninit;

pub struct ArrayVec<T, const N: usize> {
    len: usize,
    values: [MaybeUninit<T>; N],
}

impl<T, const N: usize> ArrayVec<T, N> {
    pub fn new() -> Self {
        Self {
            len: 0,
            values: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    pub fn of<const M: usize>(arr: [T; M]) -> Self {
        assert!(M <= N);

        let mut result = Self::new();

        for value in arr {
            unsafe {
                result.push_unchecked(value);
            }
        }

        result
    }

    pub unsafe fn concat_unchecked(mut first: Self, second: Self) -> Self {
        first.extend_unchecked(second);
        first
    }

    pub fn len(&self) -> usize {
        if self.len > N {
            unsafe {
                unreachable_unchecked();
            }
        }

        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.values.get_unchecked(index).assume_init_ref()
    }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        let len = self.len();
        *self.values.get_unchecked_mut(len) = MaybeUninit::new(value);
        self.len += 1;
    }

    pub unsafe fn extend_unchecked(&mut self, other: Self) {
        for i in 0..other.len() {
            let value = other.values.get_unchecked(i).assume_init_read();
            self.push_unchecked(value);
        }

        mem::forget(other);
    }
}

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        for i in 0..self.len() {
            unsafe {
                self.values.get_unchecked_mut(i).assume_init_drop();
            }
        }
    }
}
