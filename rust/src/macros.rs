use crate::sbi::putchar;

pub struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            putchar(ch);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(crate::macros::Writer, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        use crate::print;
        print!("{}\n", format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! panic {
    ($($arg:tt)*) => ({
        use crate::print;
        print!("PANIC: {}:{}: {}", file!(), line!(), format_args!($($arg)*));
        loop {
            unsafe { core::arch::asm!("wfi") }
        }
    });
}

#[macro_export]
macro_rules! read_csr {
    ($reg:literal) => {{
        let value: usize;
        unsafe {
            core::arch::asm!(
                concat!("csrr {0}, ", $reg),
                out(reg) value,
            );
        }
        value
    }};
}

#[macro_export]
macro_rules! write_csr {
    ($reg:literal, $value:expr) => {
        unsafe {
            core::arch::asm!(
                concat!("csrw ", $reg, ", {0}"),
                in(reg) $value,
            );
        }
    };
}
