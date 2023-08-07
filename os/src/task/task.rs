use alloc::string::String;
use crate::config::{TRAP_CONTEXT_BASE};
use crate::mm::{KERNEL_SPACE, MemorySet, PhysPageNum, VirtAddr};
use crate::task::context::TaskContext;
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};

use alloc::vec::Vec;
use core::cell::RefMut;
use crate::fs::{File, Stdin, Stdout};
use crate::mm::page_table::translated_refmut;
use crate::sync::UPSafeCell;
use crate::task::action::SignalActions;
use crate::task::id::{KernelStack, kstack_alloc, pid_alloc, PidHandle, TaskUserRes};
use crate::task::process::ProcessControlBlock;
use crate::task::SignalFlags;

/// task control block structure
pub struct TaskControlBlock {
    // immutable
    /// Process identifier
    pub process: Weak<ProcessControlBlock>,
    /// Kernel stack corresponding to PID
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub exit_code: Option<i32>,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    #[allow(unused)]
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
}

impl TaskControlBlock {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), ustack_base, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kernel_stack = kstack_alloc();
        let kstack_top = kernel_stack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: TaskContext::goto_trap_return(kstack_top),
                    task_status: TaskStatus::Ready,
                    exit_code: None,
                })
            },
        }
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn get_user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.inner_exclusive_access();
        inner.memory_set.token()
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Blocked,
}
