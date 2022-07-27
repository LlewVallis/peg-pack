use std::hint::unreachable_unchecked;
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

    pub fn len(&self) -> usize {
        if self.len > N {
            unsafe {
                unreachable_unchecked();
            }
        }

        self.len
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.values.get_unchecked(index).assume_init_ref()
    }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        let len = self.len();
        *self.values.get_unchecked_mut(len) = MaybeUninit::new(value);
        self.len += 1;
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
