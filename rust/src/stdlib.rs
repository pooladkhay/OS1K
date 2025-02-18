use crate::mem::{Error, page_alloc};

/// Sets `size` bytes of memory starting at `buf` to the value `val`.
///
/// # Safety
///
/// This function is unsafe because it performs raw pointer dereferencing. The caller must ensure that:
///
/// - `buf` is valid for writes of `size` bytes.
/// - `buf` is properly aligned and non-null.
///
/// Failure to meet these requirements may result in undefined behavior.
///
/// Returns the original pointer `buf`.
pub unsafe fn memset(buf: *mut u8, val: u8, size: usize) -> *mut u8 {
    for i in 0..size {
        unsafe { *buf.add(i) = val }
    }

    buf
}

/// Copies `size` bytes from the memory region starting at `src` to the memory region starting at `dst`.
///
/// # Safety
///
/// This function is unsafe because it performs raw pointer dereferencing. The caller must ensure that:
///
/// - `src` must be valid for reads of `size` bytes.
/// - `dst` must be valid for writes of `size` bytes.
/// - The memory regions starting at `src` and `dst` do not overlap.
/// - Both `src` and `dst` must be properly aligned.
///
/// Failing to meet these requirements may result in undefined behavior.
///
/// Returns the original pointer `dst`.
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    for i in 0..size {
        unsafe { *dst.add(i) = *src.add(i) }
    }

    dst
}

/// Copies up to `size` bytes from the memory region starting at `src` to the memory region starting at `dst`,
/// and ensures that the destination buffer is null-terminated.
///
/// This function behaves similarly to C's `strncpy`, with the important difference that it always writes a null byte
/// at `dst[size - 1]`. This means that if the source string is longer than or equal to `size`, only `size - 1` bytes
/// are effectively copied, and the last byte in the destination is guaranteed to be `'\0'`.
///
/// # Safety
///
/// This function is unsafe because it performs raw pointer arithmetic and dereferencing. The caller must ensure that:
///
/// - `src` is valid for reads of at least `size` bytes.
/// - `dst` is valid for writes of at least `size` bytes.
/// - Both `src` and `dst` are properly aligned and non-null.
///
/// Failing to uphold these invariants may result in undefined behavior.
///
/// Returns the original pointer `dst`.
pub unsafe fn strncpy<'a>(dst: *mut u8, src: *const u8, size: usize) -> *mut u8 {
    unsafe {
        for i in 0..size {
            let s = *src.add(i);
            if s != b'\0' {
                *dst.add(i) = s
            } else {
                *dst.add(i) = b'\0'
            }
        }

        *dst.add(size - 1) = b'\0'
    }

    dst
}

/// Compares two null-terminated strings byte-by-byte.
///
/// Returns:
/// - 0 if both strings are equal,
/// - a negative value if the first differing byte in `s1` is less than the corresponding byte in `s2`,
/// - a positive value if the first differing byte in `s1` is greater than the corresponding byte in `s2`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and assumes that:
///
/// - Both `s1` and `s2` point to valid memory regions containing null-terminated strings.
/// - The memory regions are accessible for reading until a null byte (`b'\0'`) is encountered.
/// - Both pointers are non-null and properly aligned.
///
/// Violating these conditions may lead to undefined behavior.
pub unsafe fn strcmp(s1: *const u8, s2: *const u8) -> isize {
    let mut s1 = s1;
    let mut s2 = s2;
    unsafe {
        while *s1 != b'\0' && *s2 != b'\0' {
            if *s1 != *s2 {
                break;
            }
            s1 = s1.add(1);
            s2 = s2.add(1);
        }

        (*s1 - *s2) as isize
    }
}

/// Allocates `n` pages of memory.
///
/// Returns the beginning address of the allocated region if successful,
/// or an error of type `Error` if the allocation fails.
/// The returned address is guaranteed to be page-aligned.
pub fn palloc(n: usize) -> Result<usize, Error> {
    page_alloc(n)
}
