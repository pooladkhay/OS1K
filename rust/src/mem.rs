use core::{
    fmt::LowerHex,
    ops::{Add, Sub},
    slice,
    str::Utf8Error,
};

use crate::{
    panic,
    sync::{Mutex, OnceCell},
};

pub const PAGE_SIZE: usize = 4096;

// MARK - INTERFACE TO THE MEMORY MANAGEMENT SUB-SYSTEM

// Global static instance of Memory, safely wrapped in a OnceCell.
static MEMORY: OnceCell<Mutex<Memory>> = OnceCell::new();

/// Initializes the global static instance of Memory
///
/// Must be called early in the boot process before any call to buddy_alloc().
pub fn init(ram_start: usize, ram_end: usize, alloc_mem_start: usize, alloc_mem_end: usize) {
    MEMORY.get_or_init(|| {
        Mutex::new(Memory::new(
            Some(ram_start),
            Some(ram_end),
            Some(alloc_mem_start),
            Some(alloc_mem_end),
        ))
    });
}

pub fn buddy_alloc(n: usize) -> Result<PhysAddr, Error> {
    // It's safe to call Memory::new() with None values since
    // init_mem() has already initialized the OnceCell and Mutex.
    let mem = MEMORY.get_or_init(|| Mutex::new(Memory::new(None, None, None, None)));
    // FIXME: Giant lock on all available memory
    mem.lock().buddy_alloc(n)
}

pub fn buddy_free(addr: PhysAddr) {
    // It's safe to call Memory::new() with None values since
    // init_mem() has already initialized the OnceCell and Mutex.
    let mem = MEMORY.get_or_init(|| Mutex::new(Memory::new(None, None, None, None)));
    // FIXME: Giant lock on all available memory
    mem.lock().buddy_free(addr);
}

// MARK - END

#[derive(Debug)]
pub enum Error {
    OutOfMemory,
    ZeroSize,
}

// MARK - INITIAL ALLOCATOR

struct InitialAlloc {
    start: usize,
    end: usize,
    next: usize,
}

impl InitialAlloc {
    /// Returns a new instance of InitialAlloc, that controls a zeroed memory region of `end - start` size.
    ///
    /// # Safety
    ///
    /// - `start` and `end` point to valid memory locations and free for use.
    /// - The addresses are correctly aligned.
    ///
    /// The caller must ensure that these assumptions hold, as violating them may lead to undefined behavior.
    fn new(start: usize, end: usize) -> Self {
        let _start = start as *const u8 as *mut u8;
        let _end = end as *const u8 as *mut u8;
        unsafe { _start.write_bytes(0, _end.offset_from(_start) as usize) };

        Self {
            start,
            end,
            next: start,
        }
    }

    /// Allocates `n` pages of memory.
    ///
    /// The sole purpose of this function is to allocate a few pages of memory
    /// to initialize a more sophisticated memory allocator.
    ///
    /// Returns the beginning address of the allocated region if successful.
    /// The returned address is guaranteed to be page-aligned.
    ///
    /// # Panics
    ///
    /// This function panics if there is not enough available memory.
    fn page_alloc(&mut self, n: usize) -> PhysAddr {
        let size = n * PAGE_SIZE;

        let addr = self.next;

        if addr + size <= self.end {
            self.next += size;
        } else {
            panic!("{:?}", Error::OutOfMemory);
        }

        PhysAddr::new(addr, Some(size))
    }
}

// MARK - END

// MARK - BUDDY ALLOCATOR

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum BlockState {
    Free = 1,
    Allocated = 2,
    Split = 3,
}

/// Returns the level where a given memory block would sit
/// in the binary tree that buddy allocator internally uses.
fn find_order(n: usize) -> usize {
    if n == usize::MAX {
        size_of::<usize>() * 8
    } else {
        usize::ilog2(n + 1) as usize
    }
}

/// Returns the next powers of two that comes after `n`,
/// or `None` if `n` is grater than `(usize::MAX / 2) + 1`
fn next_power_of_two(n: usize) -> Option<usize> {
    if n == 0 {
        return Some(1);
    }

    if n > (usize::MAX / 2 + 1) {
        return None; // Cannot represent next power of two within usize
    }

    let mut x = n - 1;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;

    #[cfg(target_pointer_width = "64")]
    {
        x |= x >> 32;
    }

    Some(x + 1)
}

#[repr(C)]
struct Memory<'a> {
    start: PhysAddr,
    end: PhysAddr,
    mem_size: usize,
    buddy_node_count: usize,
    buddy_high_order: usize,
    buddy_low_order: usize,
    buddy_stack_size: usize,
    buddy_stack: &'a mut [usize], // FIXME: change to function-local
    buddy_meta: &'a mut [BlockState],
}

impl<'a> Memory<'a> {
    /// Creates a new `Memory` instance.
    ///
    /// # Safety
    ///
    /// - `ram_start`, `ram_end`, `alloc_mem_start`, and `alloc_mem_end` must be valid addresses.
    /// - This function must not be called for a second time on the same memory regions.
    ///
    /// The caller must ensure that these assumptions hold, as violating them may lead to undefined behavior.
    ///
    /// # Panics
    ///
    /// This function panics if either of arguments are `None`.
    fn new(
        ram_start: Option<usize>,
        ram_end: Option<usize>,
        alloc_mem_start: Option<usize>,
        alloc_mem_end: Option<usize>,
    ) -> Self {
        // Initializing the first allocator that will be used
        // to allocate memory to initialize the buddy allocator.
        // This allocator uses a special reserved memory region
        // defined in the linker script.
        let alloc_mem_start = alloc_mem_start.expect(
            "expected the start address of the reserved allocator memory region, found None.",
        );
        let alloc_mem_end = alloc_mem_end.expect(
            "expected the end address of the reserved allocator memory region, found None.",
        );
        let mut sc_alloc = InitialAlloc::new(alloc_mem_start, alloc_mem_end);

        let start = ram_start.expect("expected the start address of RAM, found None.");
        let end = ram_end.expect("expected the end address of RAM, found None.");

        // FIXME: This should be the size that buddy can handle,
        // i.e. previous power of two of the actual size.
        let mem_size = end - start;

        // Initialize metadata memory
        let buddy_node_count = 2 * (mem_size / PAGE_SIZE) - 1;
        let buddy_meta_size = buddy_node_count * size_of::<BlockState>();
        let buddy_meta = unsafe {
            let addr = sc_alloc
                .page_alloc(buddy_meta_size.div_ceil(PAGE_SIZE))
                .as_mut_slice_leak::<BlockState>(buddy_node_count);

            addr.as_mut_ptr()
                .write_bytes(BlockState::Free as u8, buddy_node_count);
            addr
        };

        // Initialize stack memory for DFS on metadata
        let buddy_high_order = find_order(mem_size);
        let buddy_low_order = find_order(PAGE_SIZE);
        let buddy_stack_len = buddy_high_order - buddy_low_order + 1;
        // FIXME: rethink the stack size
        let buddy_stack_size = buddy_meta_size * size_of::<usize>();
        let buddy_stack = unsafe {
            let addr = sc_alloc
                .page_alloc(buddy_stack_size.div_ceil(PAGE_SIZE))
                .as_mut_slice_leak::<usize>(buddy_stack_len);

            addr.as_mut_ptr().write_bytes(0, buddy_meta_size);
            addr
        };

        Self {
            start: PhysAddr::new(start, None),
            end: PhysAddr::new(end, None),
            mem_size,
            buddy_node_count,
            buddy_high_order,
            buddy_low_order,
            buddy_meta,
            buddy_stack,
            buddy_stack_size,
        }
    }

    /// Allocates at least `n` bytes of contiguous memory.
    ///
    /// Returns the beginning address of the allocated region if successful,
    /// or an error of type `mem::Error` if the allocation fails.
    /// The returned address is guaranteed to be page-aligned.
    ///
    /// This function uses a binary tree represented as an array of `BlockState`s.
    fn buddy_alloc(&mut self, n: usize) -> Result<PhysAddr, Error> {
        if n > self.mem_size {
            return Err(Error::OutOfMemory);
        }

        if n == 0 {
            return Err(Error::ZeroSize);
        }

        let n: usize = if n < PAGE_SIZE { PAGE_SIZE } else { n };
        let n = next_power_of_two(n).expect("can you really handle that size??");

        let req_order = self.buddy_high_order - find_order(n);

        let mut sp = 0_isize;
        self.buddy_stack[sp as usize] = 0; // index of the first node

        while sp >= 0 {
            let i = self.buddy_stack[sp as usize];
            sp -= 1;
            let level = find_order(i);

            if req_order == level {
                if self.buddy_meta[i] == BlockState::Free {
                    self.buddy_meta[i] = BlockState::Allocated;

                    let addr = unsafe {
                        (self.start.as_usize() as *const u8).add(
                            ((1 + i) - 2_usize.pow(level as u32))
                                * 2_usize.pow((self.buddy_high_order - level) as u32),
                        )
                    };
                    return Ok(PhysAddr::new(addr as usize, Some(n)));
                }
            } else {
                match self.buddy_meta[i] {
                    BlockState::Free => {
                        self.buddy_meta[i] = BlockState::Split;
                        sp += 1;
                        self.buddy_stack[sp as usize] = 2 * i + 2;
                        sp += 1;
                        self.buddy_stack[sp as usize] = 2 * i + 1;
                    }
                    BlockState::Allocated => continue,
                    BlockState::Split => {
                        sp += 1;
                        self.buddy_stack[sp as usize] = 2 * i + 2;
                        sp += 1;
                        self.buddy_stack[sp as usize] = 2 * i + 1;
                    }
                }
            };
        }

        return Err(Error::OutOfMemory);
    }

    fn buddy_free(&mut self, addr: PhysAddr) {
        if let None = addr.size {
            // If address doesn't have a size,
            // then it was not allocated by this allocator.
            return;
        }

        let size = addr.size.expect("buddy_free(): size is None.");
        let offset = addr.as_usize() - self.start.as_usize();

        let level = self.buddy_high_order - size.trailing_zeros() as usize; // Size is power of 2
        let position = offset / size;
        let i = (1 << level) - 1 + position;
        let i_at_level = (1 + i) - 2_usize.pow(level as u32);

        if self.buddy_meta[i] == BlockState::Allocated {
            self.buddy_meta[i] = BlockState::Free;
        } else {
            panic!("buddy_free(): Memory at index {i} was not allocated, something is wrong.")
        }

        // Merge with buddy logic

        let buddy_i_at_level = i_at_level ^ 1;
        let mut buddy_i = buddy_i_at_level + 2_usize.pow(level as u32) - 1;
        let mut level = level;
        let mut i = i;
        while self.buddy_meta[buddy_i] == BlockState::Free {
            // in each iteration, i is the parent of the i in previous iterations.
            i = (i - 1) / 2;

            if self.buddy_meta[i] == BlockState::Split {
                self.buddy_meta[i] = BlockState::Free;
            } else {
                panic!("buddy_free(): Memory at index {i} was not split, something is wrong.")
            }

            if i == 0 {
                break;
            }

            level = level - 1;
            let i_at_level = (1 + i) - 2_usize.pow(level as u32);
            let buddy_i_at_level = i_at_level ^ 1;
            buddy_i = buddy_i_at_level + 2_usize.pow(level as u32) - 1;
        }
    }
}

// MARK - END

// MARK - PHYSICAL-ADDRESS TYPE DEFINITION

/// `PhysAddr` represents a physical memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct PhysAddr {
    addr: usize,
    size: Option<usize>,
}

impl PhysAddr {
    pub fn new(addr: usize, size: Option<usize>) -> Self {
        // FIXME: Should not allow 0
        Self { addr, size }
    }

    pub fn size(&self) -> Option<usize> {
        self.size
    }

    pub const fn as_usize(self) -> usize {
        self.addr
    }

    /// Checks if the internal value is aligned to the specified `alignment`.
    ///
    /// Returns `true` if the internal `usize` value is evenly divisible by `alignment`, indicating
    /// that it is aligned to that boundary. For meaningful alignment checks, `alignment` should
    /// typically be a power of two (e.g., 1, 2, 4, 8).
    ///
    /// # Panics
    /// If `alignment` is zero, this function will panic due to division by zero.
    pub fn is_aligned(&self, alignment: usize) -> bool {
        self.addr % alignment == 0
    }

    /// Returns a `*const u8` pointer derived from the internal `usize` value.
    ///
    /// This function casts the internal `usize` to a constant raw pointer. The resulting pointer
    /// is not dereferenced by this function, so it is safe to call. The caller is responsible
    /// for ensuring the pointer is valid and properly aligned if they choose to dereference it.
    pub fn as_ptr(&self) -> *const u8 {
        self.addr as *const u8
    }

    /// Returns a `*mut u8` pointer derived from the internal `usize` value.
    ///
    /// This function casts the internal `usize` to a mutable raw pointer. It does not dereference
    /// the pointer, so it is safe to call. The caller must ensure that the pointer is valid and
    /// that dereferencing or writing to it does not violate Rust's aliasing rules (e.g., no
    /// concurrent mutable access without proper synchronization).
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.addr as *const u8 as *mut u8
    }

    /// # Safety
    ///
    /// - `self.addr as *const T` must be a valid, non-null pointer to a readable memory region.
    /// - The memory region must contain at least `len` initialized elements of type `T`.
    /// - The pointer must be properly aligned for type `T`.
    /// - The memory must remain allocated and immutable for the entire duration of the program.
    pub unsafe fn as_slice<T>(&self, len: usize) -> &[T] {
        unsafe { slice::from_raw_parts(self.addr as *const T, len) }
    }

    /// # Safety
    ///
    /// - `self.addr as *mut T` must be a valid, non-null pointer to a readable and writable memory region.
    /// - The memory region must contain at least `len` elements of type `T`.
    /// - The pointer must be properly aligned for type `T`.
    /// - The memory must remain allocated for the entire duration of the program.
    /// - No other references (mutable or immutable) to the memory should exist while the mutable slice is in use.
    pub unsafe fn as_mut_slice<T>(&mut self, len: usize) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.addr as *mut T, len) }
    }

    pub unsafe fn as_mut_slice_leak<T>(self, len: usize) -> &'static mut [T] {
        unsafe { slice::from_raw_parts_mut(self.addr as *mut T, len) }
    }

    /// # Safety
    ///
    /// - `self.addr as *const T` must be a valid, non-null pointer to a readable memory region containing an initialized value of type `T`.
    /// - The memory region must be at least `size_of::<T>()` bytes.
    /// - The pointer must be properly aligned for type `T`.
    /// - The memory must remain allocated and immutable for the entire duration of the program.
    pub unsafe fn as_struct<T>(&self) -> &T {
        unsafe { &*(self.addr as *const T) }
    }

    /// # Safety
    ///
    /// - `self.addr as *mut T` must be a valid, non-null pointer to a readable and writable memory region containing a value of type `T`.
    /// - The memory region must be at least `size_of::<T>()` bytes.
    /// - The pointer must be properly aligned for type `T`.
    /// - The memory must remain allocated for the entire duration of the program.
    /// - If the reference is used to read, the memory must be initialized.
    /// - No other references (mutable or immutable) to the memory should exist while the mutable reference is in use.
    pub unsafe fn as_mut_struct<T>(&mut self) -> &mut T {
        unsafe { &mut *(self.addr as *const T as *mut T) }
    }

    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `self.addr as *const u8` is a valid pointer to a readable, initialized memory region of at least `len` bytes.
    /// - The memory region remains allocated and is not deallocated for the entire duration of the program.
    /// - The memory region is not mutated for the entire duration of the program, as the returned `&str` references it immutably.
    pub unsafe fn as_str(&self, len: usize) -> Result<&str, Utf8Error> {
        let byte_slice = unsafe { slice::from_raw_parts(self.addr as *const u8, len) };
        core::str::from_utf8(byte_slice)
    }
}

// impl Add<usize> for PhysAddr {
//     type Output = Self;

//     fn add(self, rhs: usize) -> Self::Output {
//         Self(self.addr + rhs)
//     }
// }

// impl Add for PhysAddr {
//     type Output = Self;

//     fn add(self, rhs: Self) -> Self::Output {
//         Self(self.addr + rhs.0)
//     }
// }

// impl Sub<usize> for PhysAddr {
//     type Output = Self;

//     fn sub(self, rhs: usize) -> Self::Output {
//         Self(self.addr - rhs)
//     }
// }

// impl Sub for PhysAddr {
//     type Output = Self;

//     fn sub(self, rhs: Self) -> Self::Output {
//         Self(self.addr - rhs.0)
//     }
// }

impl LowerHex for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.addr, f)
    }
}

// MARK - END

// MARK - VIRTUAL-ADDRESS TYPE DEFINITION

/// `VirtAddr` represents a virtual memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub fn is_aligned(&self, alignment: usize) -> bool {
        self.0 % alignment == 0
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add for VirtAddr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<usize> for VirtAddr {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub for VirtAddr {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl LowerHex for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

// MARK - END
