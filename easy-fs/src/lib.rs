#![no_std]

extern crate alloc;

mod bitmap;
mod block_cache;
mod block_device;
mod efs;
mod layout;
mod vfs;
/// Use a block size of 512 bytes
pub const BLOCK_SZ: usize = 512;
pub use block_device::BlockDevice;
pub use efs::EasyFileSystem;
pub use vfs::Inode;
use layout::*;
use bitmap::Bitmap;
use block_cache::{get_block_cache, block_cache_sync_all};
