use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T>{
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}


impl<T> UPSafeCell<T> {
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
