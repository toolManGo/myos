use crate::drivers::GPU_DEVICE;
use crate::mm::{MapPermission, PhysAddr, PPNRange, VirtAddr};
use crate::mm::memory_set::{MapArea, MapType};
use crate::task::current_process;

const FB_VADDR: usize = 0x10000000;

pub fn sys_framebuffer() -> isize {
    let fb = GPU_DEVICE.get_framebuffer();
    let len = fb.len();
    // println!("[kernel] FrameBuffer: addr 0x{:X}, len {}", fb.as_ptr() as usize , len);
    let fb_start_pa = PhysAddr::from(fb.as_ptr() as usize);
    assert!(fb_start_pa.aligned());
    let fb_start_ppn = fb_start_pa.floor();

    let fb_end_ppn = PhysAddr::from(fb.as_ptr() as usize + len).ceil();
    let fb_start_vpn = VirtAddr::from(FB_VADDR).floor();
    let pn_offset = fb_start_ppn.0 as isize - fb_start_vpn.0 as isize;

    let current_process = current_process();
    let mut inner = current_process.inner_exclusive_access();
    inner.memory_set.push_noalloc(
        MapArea::new(
            (FB_VADDR as usize).into(),
            (FB_VADDR + len as usize).into(),
            MapType::Noalloc,
            MapPermission::R | MapPermission::W | MapPermission::U,
        ),
        PPNRange::new(fb_start_ppn, fb_end_ppn),
    );
    FB_VADDR as isize
}

pub fn sys_framebuffer_flush() -> isize {
    GPU_DEVICE.flush();
    0
}
