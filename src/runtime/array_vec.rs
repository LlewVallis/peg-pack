use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::mem;
use std::mem::MaybeUninit;

pub struct ArrayVec<T, const N: usize> {
    len: u8,
    values: [MaybeUninit<T>; N],
}

impl<T, const N: usize> ArrayVec<T, N> {
    pub fn new() -> Self {
        assert!(N > 0);
        assert!(N < 256);

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

    fn assert_invariants(&self) {
        if self.len as usize > N {
            unsafe {
                unreachable_unchecked();
            }
        }
    }

    pub fn len(&self) -> usize {
        self.assert_invariants();
        self.len as usize
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.assert_invariants();
        self.values.get_unchecked(index).assume_init_ref()
    }

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        self.assert_invariants();
        self.values.get_unchecked_mut(index).assume_init_mut()
    }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        self.assert_invariants();
        let len = self.len as usize;
        *self.values.get_unchecked_mut(len) = MaybeUninit::new(value);
        self.len = self.len.checked_add(1).unwrap_unchecked();
    }

    pub fn pop(&mut self) -> Option<T> {
        self.assert_invariants();

        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe {
            Some(self.values.get_unchecked(self.len as usize).assume_init_read())
        }
    }

    pub fn last(&self) -> Option<&T> {
        self.assert_invariants();

        if self.len == 0 {
            None
        } else {
            unsafe {
                Some(self.get_unchecked(self.len as usize - 1))
            }
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.assert_invariants();

        if self.len == 0 {
            None
        } else {
            unsafe {
                Some(self.get_unchecked_mut(self.len as usize - 1))
            }
        }
    }

    fn take_all_maybe_uninit(&mut self) -> [MaybeUninit<T>; N] {
        self.assert_invariants();
        self.len = 0;
        unsafe { mem::replace(&mut self.values, MaybeUninit::uninit().assume_init()) }
    }
}

impl<T, const N: usize> IntoIterator for ArrayVec<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter {
            start: 0,
            end: self.len,
            values: self.take_all_maybe_uninit(),
        }
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

pub struct IntoIter<T, const N: usize> {
    start: u8,
    end: u8,
    values: [MaybeUninit<T>; N],
}

impl<T, const N: usize> IntoIter<T, N> {
    unsafe fn assert_invariants(&self) {
        if self.start > self.end || self.end as usize > N {
            unreachable_unchecked();
        }
    }
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        unsafe {
            self.assert_invariants();
        }

        if self.start == self.end {
            return None;
        }

        unsafe {
            let value = self
                .values
                .get_unchecked_mut(self.start as usize)
                .assume_init_read();
            self.start = self.start.checked_add(1).unwrap_unchecked();
            Some(value)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            self.assert_invariants();
        }

        let len = (self.end - self.start) as usize;
        (len, Some(len))
    }
}

impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {}

impl<T, const N: usize> Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        unsafe {
            self.assert_invariants();
        }

        for i in self.start..self.end {
            unsafe {
                self.values.get_unchecked_mut(i as usize).assume_init_drop();
            }
        }
    }
}
