use core::{
    num::NonZero,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::{self, NonNull},
    slice,
};

use crate::mem::{PhysAddr, buddy_alloc, buddy_free};

/// Allocates at least `n` bytes of contiguous physical memory.
///
/// Returns the beginning address of the allocated region if successful,
/// or an error of type `mem::Error` if the allocation fails.
/// The returned address is guaranteed to be page-aligned.
///
pub fn phalloc(n: usize) -> Result<PhysAddr, crate::mem::Error> {
    buddy_alloc(n)
}

/// Frees the provided physical memory region (`addr`).
///
/// # Panics
///
/// This function panics if, while freeing, the state of a given block
/// is not what it expects, which indicates a bug in the allocation logic.
pub fn phree(addr: PhysAddr) {
    buddy_free(addr);
}

// FIXME: Doesn't handle nested types properly. e.g FixedVec<FixedVec<usize>>
pub struct FixedVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    phys_addr: PhysAddr,
}

unsafe impl<T: Send> Send for FixedVec<T> {}
unsafe impl<T: Sync> Sync for FixedVec<T> {}

impl<T> FixedVec<T> {
    pub fn new(cap: usize) -> Self {
        assert!(size_of::<T>() != 0, "Zero-sized types are not allowed.");

        let size = cap * size_of::<T>();
        assert!(size <= isize::MAX as usize, "Allocation is too large.");

        let phys_addr = phalloc(size).unwrap();

        Self {
            ptr: NonNull::dangling().with_addr(NonZero::new(phys_addr.as_usize()).unwrap()),
            cap,
            phys_addr,
        }
    }

    pub fn cap(&self) -> usize {
        self.cap
    }
}

impl<T> Index<usize> for FixedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.cap, "Index out of bounds.");
        unsafe { &*self.ptr.as_ptr().add(index) }
    }
}

impl<T> IndexMut<usize> for FixedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.cap, "Index out of bounds.");
        unsafe { &mut *self.ptr.as_ptr().add(index) }
    }
}

impl<T> Deref for FixedVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.cap) }
    }
}

impl<T> DerefMut for FixedVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.cap) }
    }
}

impl<T> Drop for FixedVec<T> {
    fn drop(&mut self) {
        for i in 0..self.cap {
            unsafe {
                ptr::drop_in_place(self.ptr.as_ptr().add(i));
            }
        }
        phree(self.phys_addr);
    }
}
