#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
unsafe fn memset(buf: *mut u8, val: u8, size: isize) -> *mut u8 {
    for i in 0..size {
        unsafe { *buf.offset(i) = val }
    }

    buf
}

#[unsafe(no_mangle)]
unsafe fn kernel_main() -> ! {
    unsafe {
        let bss_start = &__bss as *const u8 as *mut u8;
        let bss_end = &__bss_end as *const u8;
        _ = memset(bss_start, 0, bss_end.offset_from(bss_start));
    }

    loop {}
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot() -> ! {
    unsafe {
        asm!(
            "mv sp, {stack_top}\n
            j {kernel_main}\n",
            stack_top = in(reg) &__stack_top,
            kernel_main = sym kernel_main,
        );
    }
    loop {}
}
