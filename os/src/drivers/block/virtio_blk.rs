use alloc::collections::BTreeMap;
use super::BlockDevice;
use crate::mm::address::StepByOne;
use crate::mm::page_table::PageTable;
use crate::mm::{
    frame_alloc, frame_dealloc, FrameTracker, PhysPageNum, VirtAddr, PhysAddr,
};
use crate::sync::{Condvar, UPIntrFreeCell};
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{BlkResp, Hal, RespStatus, VirtIOBlk, VirtIOHeader};
use crate::DEV_NON_BLOCKING_ACCESS;
use crate::drivers::virtio::VirtioHal;
use crate::mm::memory_set::kernel_token;
use crate::task::schedule;

// #[allow(unused)]
// const VIRTIO0: usize = 0x10001000;
#[allow(unused)]
const VIRTIO0: usize = 0x10008000;
pub struct VirtIOBlock {
    virtio_blk: UPIntrFreeCell<VirtIOBlk<'static, VirtioHal>>,
    condvars: BTreeMap<u16, Condvar>,
}
lazy_static! {
    static ref QUEUE_FRAMES: UPIntrFreeCell<Vec<FrameTracker>> = unsafe { UPIntrFreeCell::new(Vec::new()) };
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut resp = BlkResp::default();
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe { blk.read_block_nb(block_id, buf, &mut resp).unwrap() };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });
            schedule(task_cx_ptr);
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when reading VirtIOBlk"
            );
        } else {
            self.virtio_blk
                .exclusive_access()
                .read_block(block_id, buf)
                .expect("Error when reading VirtIOBlk");
        }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut resp = BlkResp::default();
            let task_cx_ptr = self.virtio_blk.exclusive_session(|blk| {
                let token = unsafe { blk.write_block_nb(block_id, buf, &mut resp).unwrap() };
                self.condvars.get(&token).unwrap().wait_no_sched()
            });
            schedule(task_cx_ptr);
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when writing VirtIOBlk"
            );
        } else {
            self.virtio_blk
                .exclusive_access()
                .write_block(block_id, buf)
                .expect("Error when writing VirtIOBlk");
        }
    }
    fn handle_irq(&self) {
        self.virtio_blk.exclusive_session(|blk| {
            while let Ok(token) = blk.pop_used() {
                self.condvars.get(&token).unwrap().signal();
            }
        });
    }
}


impl VirtIOBlock {
    pub fn new() -> Self {
        let virtio_blk = unsafe {
            UPIntrFreeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            )
        };
        let mut condvars = BTreeMap::new();
        let channels = virtio_blk.exclusive_access().virt_queue_size();
        for i in 0..channels {
            let condvar = Condvar::new();
            condvars.insert(i, condvar);
        }
        Self {
            virtio_blk,
            condvars,
        }
    }
}
//
// pub struct VirtioHal;
//
// impl Hal for VirtioHal {
//     fn dma_alloc(pages: usize) -> usize {
//         let mut ppn_base = PhysPageNum(0);
//         for i in 0..pages {
//             let frame = frame_alloc().unwrap();
//             if i == 0 {
//                 ppn_base = frame.ppn;
//             }
//             assert_eq!(frame.ppn.0, ppn_base.0 + i);
//             QUEUE_FRAMES.exclusive_access().push(frame);
//         }
//         let pa: PhysAddr = ppn_base.into();
//         pa.0
//     }
//
//     fn dma_dealloc(pa: usize, pages: usize) -> i32 {
//         let pa = PhysAddr::from(pa);
//         let mut ppn_base: PhysPageNum = pa.into();
//         for _ in 0..pages {
//             frame_dealloc(ppn_base);
//             ppn_base.step();
//         }
//         0
//     }
//
//     fn phys_to_virt(addr: usize) -> usize {
//         addr
//     }
//
//     fn virt_to_phys(vaddr: usize) -> usize {
//         PageTable::from_token(kernel_token())
//             .translate_va(VirtAddr::from(vaddr))
//             .unwrap()
//             .0
//     }
// }
