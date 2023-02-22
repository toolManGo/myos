#![no_main]
#![no_std]
#![feature(panic_info_message)]

#![feature(alloc_error_handler)]


use log::*;

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
mod heap_alloc;

fn main() {
    // println!("Hello, world!");
}


core::arch::global_asm!(include_str!("entry.asm"));
core::arch::global_asm!(include_str!("link_app.S"));


#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    println!("[kernel] Hello, myos!");
    heap_alloc::init_heap();
    trap::init();
    loader::load_apps();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first_task();
    panic!("Unreachable in rust_main!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}