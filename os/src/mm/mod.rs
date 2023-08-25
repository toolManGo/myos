pub mod heap_allocator;
pub mod address;
pub mod page_table;
pub mod frame_allocator;
pub mod memory_set;

pub use address::{VPNRange, PPNRange};
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
// pub use memory_set::remap_test;
pub use memory_set::{kernel_token, MapPermission, MemorySet, MapArea, MapType, KERNEL_SPACE};
use page_table::PTEFlags;
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}