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
fn kernel_main() {
    loop {}
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot() -> ! {
    unsafe {
        asm!(
            "mv sp, {stack_top}\n
            li t1, 6\n
            j {kernel_main}\n",
            stack_top = in(reg) &__stack_top,
            kernel_main = sym kernel_main,
        );
    }
    loop {}
}
