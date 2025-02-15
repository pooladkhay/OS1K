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
            unsafe { asm!("wfi") }
        }
    });
}
