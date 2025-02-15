#![no_std]
#![no_main]

mod println;
mod sbi;
mod stdlib;

use core::{arch::asm, panic::PanicInfo};
use stdlib::memset;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

unsafe fn kernel_main() -> ! {
    unsafe {
        let bss_start = &__bss as *const u8 as *mut u8;
        let bss_end = &__bss_end as *const u8;
        _ = memset(bss_start, 0, bss_end.offset_from(bss_start) as usize);
    }

    println!("Hello, World!");

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
