#![no_main]
#![no_std]
#![feature(panic_info_message)]


use log::*;

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

fn main() {
    // println!("Hello, world!");
}


core::arch::global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello, myos!");
    panic!("Shutdown machine!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}