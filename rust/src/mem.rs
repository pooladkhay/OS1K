use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{__free_ram, __free_ram_end, sync::OnceCell};

const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub enum Error {
    OutOfMemory,
}

struct Memory {
    // start: usize,
    end: usize,
    next: AtomicUsize,
}

impl Memory {
    /// Creates a new `Memory` instance.
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` blocks to convert external symbols (`__free_ram` and `__free_ram_end`)
    /// into usable addresses. It assumes that:
    /// - These symbols are provided by the linker and point to valid memory locations.
    /// - The addresses obtained from these symbols are correctly aligned and within the expected memory range.
    /// - The memory region from `__free_ram` to `__free_ram_end` is valid and free for use.
    ///
    /// The caller must ensure that these assumptions hold, as violating them may lead to undefined behavior.
    fn new() -> Self {
        Self {
            //
            end: unsafe { &__free_ram_end } as *const _ as usize,
            next: AtomicUsize::new(unsafe { &__free_ram } as *const _ as usize),
        }
    }

    /// Allocates a contiguous memory region of `size` bytes.
    ///
    /// Returns a `Result` containing the beginning address of the allocated region if the allocation is successful,
    /// or an `Error` (currently `Error::OutOfMemory`) if there is insufficient memory.
    fn allocate(&self, size: usize) -> Result<usize, Error> {
        self.next
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                if n + size <= self.end {
                    Some(n + size)
                } else {
                    None
                }
            })
            .map_err(|_| Error::OutOfMemory)
    }
}

// Global static instance of Memory, safely wrapped in a OnceCell.
static MEMORY: OnceCell<Memory> = OnceCell::new();

/// Allocates `n` pages of memory.
///
/// Returns the beginning address of the allocated region if successful,
/// or an error of type `Error` if the allocation fails.
/// The returned address is guaranteed to be page-aligned.
pub fn page_alloc(n: usize) -> Result<usize, Error> {
    let mem = MEMORY.get_or_init(Memory::new);
    mem.allocate(n * PAGE_SIZE)
}
