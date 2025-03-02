#![feature(naked_functions)]
#![no_std]
#![no_main]

mod macros;
mod mem;
mod sbi;
mod stdlib;
mod sync;
mod trap;

use core::{arch::asm, hint::spin_loop, panic::PanicInfo};
use mem::init_mem;
use stdlib::{memset, phalloc, phree};
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
    static __allocator_mem: u8;
    static __allocator_mem_end: u8;
}

unsafe fn kernel_init(hart_id: usize, _dtb_addr: usize) {
    write_csr!("stvec", trap_entry as *const ());

    if hart_id != 0 {
        // FIXME: when running in debug mode, value is not zero
        println!("hart_id:{}", hart_id);
        loop {
            spin_loop();
        }
    }

    let bss_start = unsafe { &__bss } as *const u8 as *mut u8;
    let bss_end = unsafe { &__bss_end } as *const u8;
    _ = unsafe { memset(bss_start, 0, bss_end.offset_from(bss_start) as usize) };

    let alloc_mem_start = unsafe { &__allocator_mem } as *const u8 as *mut u8;
    let alloc_mem_end = unsafe { &__allocator_mem_end } as *const u8;
    _ = unsafe {
        memset(
            alloc_mem_start,
            0,
            alloc_mem_end.offset_from(alloc_mem_start) as usize,
        )
    };

    // FIXME: Either this or zeroing during the allocation
    // FIXME: Should be replaced with the actual memory addresses acquired by parsing dtb
    let ram_start = unsafe { &__free_ram } as *const u8 as *mut u8;
    let ram_end = unsafe { &__free_ram_end } as *const u8;
    _ = unsafe { memset(ram_start, 0, ram_end.offset_from(ram_start) as usize) };

    init_mem(
        ram_start as usize,
        ram_end as usize,
        alloc_mem_start as usize,
        alloc_mem_end as usize,
    );
}

unsafe fn kernel_main(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        kernel_init(hart_id, dtb_addr);
    }

    println!("Hello, World!");

    let m1 = phalloc(13 * 1024 * 1024).unwrap();
    println!("0x{:x}", m1);

    let m2 = phalloc(12 * 1024 * 1024).unwrap();
    println!("0x{:x}", m2);

    let m3 = phalloc(15 * 1024 * 1024).unwrap();
    println!("0x{:x}", m3);

    let m4 = phalloc(16 * 1024 * 1024).unwrap();
    println!("0x{:x}", m4);

    phree(m1);

    phree(m2);

    phree(m3);

    phree(m4);

    loop {
        unsafe { asm!("wfi") }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        asm!(
            "mv a0, {0}",
            "mv a1, {1}",
            "mv sp, {2}",
            "j {3}",
            in(reg) hart_id,
            in(reg) dtb_addr,
            in(reg) &__stack_top,
            sym kernel_main,
            options(noreturn)
        );
    }
}
