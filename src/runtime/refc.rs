use std::cell::Cell;
use std::ops::Deref;
use std::ptr::NonNull;

pub struct Refc<T> {
    inner: NonNull<RefcBox<T>>,
}

struct RefcBox<T> {
    referents: Cell<u32>,
    value: T,
}

impl<T> Refc<T> {
    pub fn new(value: T) -> Self {
        let boxed = RefcBox {
            referents: Cell::new(1),
            value,
        };

        let inner = NonNull::from(Box::leak(Box::new(boxed)));

        Self { inner }
    }

    fn referents(&self) -> &Cell<u32> {
        unsafe { &self.inner.as_ref().referents }
    }
}

impl<T> Drop for Refc<T> {
    fn drop(&mut self) {
        let referents = self.referents();
        let new_count = unsafe { referents.get().checked_sub(1).unwrap_unchecked() };

        if new_count == 0 {
            unsafe {
                Box::from_raw(self.inner.as_ptr());
            }
        } else {
            referents.set(new_count)
        }
    }
}

impl<T> Deref for Refc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &self.inner.as_ref().value }
    }
}

impl<T> Clone for Refc<T> {
    fn clone(&self) -> Self {
        let referents = self.referents();
        let new_count = unsafe { referents.get().checked_add(1).unwrap_unchecked() };
        referents.set(new_count);

        Self { inner: self.inner }
    }
}
