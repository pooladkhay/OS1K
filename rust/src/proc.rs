use core::{arch::naked_asm, ptr};

use crate::{
    stdlib::FixedVec,
    sync::{Mutex, OnceCell},
};

const PROC_STACK_SIZE: usize = 8 * 1024 / size_of::<usize>();
const PROC_MAX: usize = 8;

static PROC_TABLE: OnceCell<Mutex<ProcTable>> = OnceCell::new();

#[derive(Debug, PartialEq)]
#[repr(u8)]
enum ProcState {
    Unused = 0,
    Runnable = 1,
}

#[derive(Debug)]
pub struct Process {
    pid: usize,
    state: ProcState,
    sp: usize,
    stack: [u8; PROC_STACK_SIZE],
}

impl Process {
    fn sp_as_mut_ptr(&mut self) -> *mut usize {
        &mut self.sp as *mut usize
    }
}

struct ProcTable {
    table: FixedVec<Process>,
    curr_proc_idx: usize,
}

impl ProcTable {
    fn new() -> Self {
        Self {
            table: FixedVec::new(PROC_MAX),
            curr_proc_idx: 0,
        }
    }

    fn next_unused(&self) -> Option<usize> {
        for i in 0..self.table.cap() {
            if self.table[i].state == ProcState::Unused {
                return Some(i);
            }
        }
        None
    }

    fn get_proc(&mut self, index: usize) -> &mut Process {
        &mut self.table[index]
    }

    fn create_process(&mut self, pc: usize) -> usize {
        let proc_index = self.next_unused().expect("no free process slots.");

        let proc = &mut self.table[proc_index];
        proc.pid = proc_index;

        proc.state = ProcState::Runnable;
        let mut sp = &mut proc.stack[PROC_STACK_SIZE - 4] as *mut u8 as *mut usize;

        unsafe {
            sp = sp.offset(-12);
            ptr::write(sp, pc);
        }
        for i in 1..13 {
            unsafe {
                ptr::write(sp.offset(i), 0);
            }
        }

        proc.sp = sp as usize;

        proc_index
    }
}

pub fn init() {
    PROC_TABLE.get_or_init(|| Mutex::new(ProcTable::new()));
}

pub fn new(pc: usize) {
    PROC_TABLE
        .get_or_init(|| Mutex::new(ProcTable::new()))
        .lock()
        .create_process(pc);
}

pub fn give_up() {
    let mut proc_guard = PROC_TABLE
        .get_or_init(|| Mutex::new(ProcTable::new()))
        .lock();

    let curr_proc_idx = proc_guard.curr_proc_idx;

    let mut next_runnable_idx = 0;
    for i in 1..proc_guard.table.cap() {
        if proc_guard.table[i].state == ProcState::Runnable && i != proc_guard.curr_proc_idx {
            next_runnable_idx = i;
            break;
        }
    }

    let prev_sp = proc_guard.get_proc(curr_proc_idx).sp_as_mut_ptr();
    let next_sp = proc_guard.get_proc(next_runnable_idx).sp_as_mut_ptr();

    proc_guard.curr_proc_idx = next_runnable_idx;

    drop(proc_guard);

    switch_context(prev_sp, next_sp);
}

#[naked]
pub extern "C" fn switch_context(prev_sp: *mut usize, next_sp: *mut usize) {
    unsafe {
        naked_asm!(
            // Save callee-saved registers onto the current process's stack.
            "addi sp, sp, -13 * 4", // Allocate stack space for 13 4-byte registers
            "sw ra,  0  * 4(sp)",   // Save callee-saved registers only
            "sw s0,  1  * 4(sp)",
            "sw s1,  2  * 4(sp)",
            "sw s2,  3  * 4(sp)",
            "sw s3,  4  * 4(sp)",
            "sw s4,  5  * 4(sp)",
            "sw s5,  6  * 4(sp)",
            "sw s6,  7  * 4(sp)",
            "sw s7,  8  * 4(sp)",
            "sw s8,  9  * 4(sp)",
            "sw s9,  10 * 4(sp)",
            "sw s10, 11 * 4(sp)",
            "sw s11, 12 * 4(sp)",
            // Switch the stack pointer.
            "sw sp, (a0)", // *prev_sp = sp;
            "lw sp, (a1)", // Switch stack pointer (sp) here
            // Restore callee-saved registers from the next process's stack.
            "lw ra,  0  * 4(sp)", // Restore callee-saved registers only
            "lw s0,  1  * 4(sp)",
            "lw s1,  2  * 4(sp)",
            "lw s2,  3  * 4(sp)",
            "lw s3,  4  * 4(sp)",
            "lw s4,  5  * 4(sp)",
            "lw s5,  6  * 4(sp)",
            "lw s6,  7  * 4(sp)",
            "lw s7,  8  * 4(sp)",
            "lw s8,  9  * 4(sp)",
            "lw s9,  10 * 4(sp)",
            "lw s10, 11 * 4(sp)",
            "lw s11, 12 * 4(sp)",
            "addi sp, sp, 13 * 4", // We've popped 13 4-byte registers from the stack
            "ret",
        )
    }
}
