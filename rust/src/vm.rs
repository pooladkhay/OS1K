use crate::{mem::PAGE_SIZE, panic, stdlib::FixedVec};

// SATP: Supervisor Address Translation and Protection
pub const SATP_SV32: usize = 1 << 31;
pub const PAGE_V: usize = 1 << 0;
pub const PAGE_R: usize = 1 << 1;
pub const PAGE_W: usize = 1 << 2;
pub const PAGE_X: usize = 1 << 3;
pub const PAGE_U: usize = 1 << 4;

#[derive(Debug)]
pub struct PageTable {
    root_pt: FixedVec<usize>,
    second_pts: FixedVec<FixedVec<usize>>,
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            // each page table level has 2^10 entries.
            // each entry is 32 bits wide, hense
            // each level fits into one page.
            root_pt: FixedVec::new(1024),
            second_pts: FixedVec::new(1024),
        }
    }

    pub fn root_pt_addr(&self) -> usize {
        self.root_pt.as_ptr() as usize
    }

    pub fn map_page(&mut self, vaddr: usize, paddr: usize, flags: usize) {
        if vaddr % PAGE_SIZE != 0 {
            panic!("unaligned vaddr {vaddr:x}");
        }
        if paddr % PAGE_SIZE != 0 {
            panic!("unaligned paddr {paddr:x}");
        }

        let vpn1 = vaddr >> 22 & 0x3ff;

        if (self.root_pt[vpn1] & PAGE_V) == 0 {
            // PTE is not valid,
            // lets create the non-existing 2nd level page table
            let second_pt: FixedVec<usize> = FixedVec::new(1024);
            let second_pt_phys_addr = second_pt.as_ptr() as usize;
            self.second_pts[vpn1] = second_pt;
            self.root_pt[vpn1] = ((second_pt_phys_addr / PAGE_SIZE) << 10) | PAGE_V;
        }

        let vpn0 = vaddr >> 12 & 0x3ff;
        let second_pt = &mut self.second_pts[vpn1];
        second_pt[vpn0] = ((paddr / PAGE_SIZE) << 10) | flags | PAGE_V;
    }
}
