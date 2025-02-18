#![feature(naked_functions)]
#![no_std]
#![no_main]

mod macros;
mod mem;
mod sbi;
mod stdlib;
mod sync;
mod trap;

use core::{arch::asm, panic::PanicInfo};
use stdlib::memset;
use trap::trap_entry;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    panic!("{info}")
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

unsafe fn kernel_main() -> ! {
    write_csr!("stvec", trap_entry as *const ());

    unsafe {
        let bss_start = &__bss as *const u8 as *mut u8;
        let bss_end = &__bss_end as *const u8;
        _ = memset(bss_start, 0, bss_end.offset_from(bss_start) as usize);

        let ram_start = &__free_ram as *const u8 as *mut u8;
        let ram_end = &__free_ram_end as *const u8;
        _ = memset(ram_start, 0, ram_end.offset_from(ram_start) as usize);
    }

    println!("Hello, World!");

    // trigger an exception
    unsafe { asm!("unimp") }

    loop {
        unsafe { asm!("wfi") }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot() -> ! {
    unsafe {
        asm!(
            "mv sp, {stack_top}",
            "j {kernel_main}",
            stack_top = in(reg) &__stack_top,
            kernel_main = sym kernel_main,
            options(noreturn)
        );
    }
}
