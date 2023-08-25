//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 20;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;//3145728
pub const MEMORY_END: usize = 0x88000000; //2155872256
pub const PAGE_SIZE: usize = 0x1000; //4096
pub const PAGE_SIZE_BITS: usize = 0xc; //12
pub const MAX_SYSCALL_NUM: usize = 500;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;
pub const CLOCK_FREQ: usize = 12500000;

pub use crate::board::{MMIO};

// pub const MMIO: &[(usize, usize)] = &[
//     // (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
//     (0x1000_1000, 0x00_1000), // Virtio Block in virt machine
// ];

