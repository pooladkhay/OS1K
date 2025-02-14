pub unsafe fn memset(buf: *mut u8, val: u8, size: isize) -> *mut u8 {
    for i in 0..size {
        unsafe { *buf.offset(i) = val }
    }

    buf
}
