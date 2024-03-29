use alloc::collections::BTreeMap;
use crate::sync::UPIntrFreeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::info;
use riscv::register::satp;
use crate::config::{MEMORY_END, MMIO, PAGE_SIZE, TRAMPOLINE};
use crate::mm::address::{PhysAddr, PhysPageNum, PPNRange, StepByOne, VirtAddr, VirtPageNum, VPNRange};
use crate::mm::frame_allocator::{frame_alloc, FrameTracker};
use crate::mm::page_table::{PageTable, PTEFlags};
use spin::Mutex;
use crate::mm::PageTableEntry;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref KERNEL_SPACE: Arc<UPIntrFreeCell<MemorySet>> =
        Arc::new(unsafe { UPIntrFreeCell::new(MemorySet::new_kernel()) });
}


// 一小块区域的虚拟地址映射物理地址关系
pub struct MapArea {
    //一段虚拟页号的连续区间，表示该逻辑段在地址区间中的位置和长度。它是一个迭代器
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_perm: MapPermission) -> Self {
        let start = start_va.floor();
        let end = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start, end),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        #[allow(clippy::single_match)]
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }


    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum::from(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
            MapType::Linear(pn_offset) => {
                // check for sv39
                assert!(vpn.0 < (1usize << 27));
                ppn = PhysPageNum((vpn.0 as isize + pn_offset) as usize);
            }
            MapType::Noalloc => {
                panic!("Noalloc should not be mapped");
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags)
    }

    pub fn map_noalloc(&mut self, page_table: &mut PageTable,ppn_range:PPNRange) {
        for (vpn,ppn) in core::iter::zip(self.vpn_range,ppn_range) {
            self.data_frames.insert(vpn, FrameTracker::new_noalloc(ppn));
            let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
            page_table.map(vpn, ppn, pte_flags);
        }
    }

    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            // 映射为实际物理地址
            let dst = &mut page_table.translate(current_vpn).unwrap().ppn().get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(
                another.vpn_range.get_start(),
                another.vpn_range.get_end(),
            ),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
    /// offset of page num
    Linear(isize),
    Noalloc,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// Get the token of the kernel memory space
pub fn kernel_token() -> usize {
    KERNEL_SPACE.exclusive_access().token()
}
/// 地址空间 是一系列有关联的不一定连续的逻辑段，
/// 这种关联一般是指这些逻辑段组成的虚拟内存空间与一个运行的程序（目前把一个运行的程序称为任务，后续会称为进程）绑定，
/// 即这个运行的程序对代码和数据的直接访问范围限制在它关联的虚拟地址空间之内。
pub struct MemorySet {
    //页表
    page_table: PageTable,
    //映射区域
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn push_noalloc(&mut self, mut map_area: MapArea, ppn_range: PPNRange) {
        map_area.map_noalloc(&mut self.page_table, ppn_range);
        self.areas.push(map_area);
    }
    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }
    pub fn from_existed_user(user_apace: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        for area in user_apace.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area,None);

            for vpn in area.vpn_range {
                let src_ppn = user_apace.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn.get_bytes_array().copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        memory_set
    }

    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        self.push(MapArea::new(start_va, end_va, MapType::Framed, permission), None);
    }

    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }

    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        info!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        info!("mapping .text section");
        memory_set.push(MapArea::new(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::X),
                        None);

        info!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        info!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        info!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        info!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        info!("mapping memory-mapped registers");
        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None);
        }
        memory_set
    }
    fn map_trampoline(&mut self) {
        self.page_table.map(VirtAddr::from(TRAMPOLINE).into(),
                            PhysAddr::from(strampoline as usize).into(),
                            PTEFlags::R | PTEFlags::X)
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();

        // map trampoline
        memory_set.map_trampoline();

        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(map_area, Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]));
            }
        }

        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        // let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        // memory_set.push(MapArea::new(user_stack_bottom.into(),
        //                              user_stack_top.into(),
        //                              MapType::Framed,
        //                              MapPermission::R | MapPermission::W | MapPermission::U),
        //                 None,
        // );
        // // map TrapContext
        // memory_set.push(
        //     MapArea::new(
        //         TRAP_CONTEXT_BASE.into(),
        //         TRAMPOLINE.into(),
        //         MapType::Framed,
        //         MapPermission::R | MapPermission::W,
        //     ),
        //     None,
        // );
        (
            memory_set,
            user_stack_bottom,
            elf.header.pt2.entry_point() as usize,
        )
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            core::arch::asm!("sfence.vma");
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}


// #[allow(unused)]
// pub fn remap_test() {
//     let mut kernel_space = KERNEL_SPACE.exclusive_access();
//     let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
//     let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
//     let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_text.floor())
//         .unwrap()
//         .writable());
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_rodata.floor())
//         .unwrap()
//         .writable());
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_data.floor())
//         .unwrap()
//         .executable());
//     info!("remap_test passed!");
// }