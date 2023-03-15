use log::info;
use crate::config::PAGE_SIZE;
use crate::mm::page_table::PageTable;
use crate::mm::{MemorySet, PhysAddr, VirtAddr};

use crate::task::{current_user_token, exit_current_and_run_next, mmap, suspend_current_and_run_next, TASK_MANAGER, TaskManager, TaskStatus};

use crate::timer::get_time_us;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}


#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// get time with second and microsecond
// pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
//     let us = get_time_us();
//     unsafe {
//         *ts = TimeVal {
//             sec: us / 1_000_000,
//             usec: us % 1_000_000,
//         };
//     }
//     0
// }


// YOUR JOB: 引入虚地址后重写 sys_get_time
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let virt_addr = VirtAddr(_ts as usize);
    let phys_addr = translate_va(virt_addr);
    if let Some(phys_addr) = phys_addr {
        let us = get_time_us();
        let kernel_ts = phys_addr.0 as *mut TimeVal;
        unsafe {
            *kernel_ts = TimeVal {
                sec: us / 1_000_000,
                usec: us % 1_000_000,
            };
        }
        0
    } else {
        -1
    }
}

fn translate_va(virt_addr: VirtAddr) -> Option<PhysAddr> {
    PageTable::from_token(current_user_token()).translate_va(virt_addr)
}

// CLUE: 从 ch4 开始不再对调度算法进行测试~
pub fn sys_set_priority(_prio: isize) -> isize {
    -1
}

// YOUR JOB: 扩展内核以实现 sys_mmap 和 sys_munmap
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    return -1;
    // if (_start % PAGE_SIZE) != 0 { return -1; }
    // if _port & !0x7 != 0 || _port & 0x7 == 0 { return -1; }
    //
    // // if _len % PAGE_SIZE != 0 {
    // //     _len = ( _len / PAGE_SIZE + 1 ) * PAGE_SIZE;
    // // }
    //
    // mmap(_start, _len, _port)

}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    -1
}

// YOUR JOB: 引入虚地址后重写 sys_task_info
// pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
//     -1
// }