#![no_main]
#![no_std]
#![feature(panic_info_message)]

#![feature(alloc_error_handler)]

extern crate alloc;
use log::*;
#[path = "boards/qemu.rs"]
mod board;
#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;
mod loader;
mod sync;
mod syscall;
mod trap;
mod config;
mod task;
mod timer;
mod mm;
mod fs;
mod drivers;
mod net;
// mod build;

#[macro_use]
extern crate bitflags;


fn main() {
    // println!("Hello, world!");
}


core::arch::global_asm!(include_str!("entry.asm"));
// core::arch::global_asm!(include_str!("link_app.S"));

use lazy_static::*;
use sync::UPIntrFreeCell;
use crate::drivers::chardev::CharDevice;
use crate::drivers::{GPU_DEVICE, KEYBOARD_DEVICE, MOUSE_DEVICE, UART};

lazy_static! {
    pub static ref DEV_NON_BLOCKING_ACCESS: UPIntrFreeCell<bool> =
        unsafe { UPIntrFreeCell::new(false) };
}
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    // logging::init();
    mm::init();
    UART.init();
    println!("[kernel] Hello, myos!");
    println!("KERN: init gpu");
    let _gpu = GPU_DEVICE.clone();
    println!("KERN: init keyboard");
    let _keyboard = KEYBOARD_DEVICE.clone();
    println!("KERN: init mouse");
    let _mouse = MOUSE_DEVICE.clone();
    println!("KERN: init trap");
    // mm::remap_test();
    // task::add_initproc();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    board::device_init();
    fs::list_apps();
    task::add_initproc();
    *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}