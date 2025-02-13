use core::arch::asm;

#[unsafe(no_mangle)]
pub unsafe fn sbi_call(
    arg0: isize,
    arg1: isize,
    arg2: isize,
    arg3: isize,
    arg4: isize,
    arg5: isize,
    fid: isize,
    eid: isize,
) -> Result<isize, isize> {
    let mut err = 0;
    let mut val = 0;

    // SBI calls return a pair of values in a0 and a1,
    // with a0 returning an error code.
    unsafe {
        asm!(
            "ecall",
            inout("a0") arg0 => err,
            inout("a1") arg1 => val,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid,
        )
    }

    if err == 0 { Ok(val) } else { Err(err) }
}
