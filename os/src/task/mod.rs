mod switch;

#[allow(clippy::module_inception)]
mod task;
mod context;
mod pid;
mod manager;
mod processor;

use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
use manager::fetch_task;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};


lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}



pub fn suspend_current_and_run_next(){
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ stop exclusively accessing parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** stop exclusively accessing current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}
//
// /// The task manager, where all the tasks are managed.
// ///
// /// Functions implemented on `TaskManager` deals with all task state transitions
// /// and task context switching.
// ///
// /// Most of `TaskManager` are hidden behind the field `inner`, to defer
// /// borrowing checks to runtime.
// pub struct TaskManager {
//     /// total number of tasks
//     num_app: usize,
//     /// use inner value to get mutable access
//     inner: UPSafeCell<TaskManagerInner>,
// }
//
// /// The task manager inner in 'UPSafeCell'
// struct TaskManagerInner {
//     /// task list
//     tasks: Vec<TaskControlBlock>,
//     /// id of current `Running` task
//     current_task: usize,
// }
//
// lazy_static! {
//     /// a `TaskManager` instance through lazy_static!
//     pub static ref TASK_MANAGER: TaskManager = {
//         info!("init TASK_MANAGER");
//         let num_app = get_num_app();
//         info!("num_app = {}", num_app);
//         let mut tasks: Vec<TaskControlBlock> = Vec::new();
//         for i in 0..num_app {
//             tasks.push(TaskControlBlock::new(get_app_data(i), i));
//         }
//         TaskManager {
//             num_app,
//             inner: unsafe {
//                 UPSafeCell::new(TaskManagerInner {
//                     tasks,
//                     current_task: 0,
//                 })
//             },
//         }
//     };
// }
//
// impl TaskManager {
//     fn run_first_task(&self) -> ! {
//         let mut inner = self.inner.exclusive_access();
//         let task0 = &mut inner.tasks[0];
//         task0.task_status = TaskStatus::Running;
//         let nex_task_cx_ptr = &task0.task_cx as *const TaskContext;
//         drop(inner);
//         let mut _unused = TaskContext::zero_init();
//         unsafe {
//             __switch(&mut _unused as *mut TaskContext, nex_task_cx_ptr);
//         }
//         panic!("unreachable in run_first_task!");
//     }
//
//     /// Change the status of current `Running` task into `Ready`.
//     fn mark_current_suspended(&self) {
//         let mut inner = self.inner.exclusive_access();
//         let current = inner.current_task;
//         inner.tasks[current].task_status = TaskStatus::Ready;
//     }
//
//     /// Change the status of current `Running` task into `Exited`.
//     fn mark_current_exited(&self) {
//         let mut inner = self.inner.exclusive_access();
//         let current = inner.current_task;
//         inner.tasks[current].task_status = TaskStatus::Exited;
//     }
//
//     /// Find next task to run and return task id.
//     ///
//     /// In this case, we only return the first `Ready` task in task list.
//     fn find_next_task(&self) -> Option<usize> {
//         let inner = self.inner.exclusive_access();
//         let current = inner.current_task;
//         (current + 1..current + self.num_app + 1)
//             .map(|id| id % self.num_app)
//             .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
//     }
//     /// Get the current 'Running' task's token.
//     fn get_current_token(&self) -> usize {
//         let inner = self.inner.exclusive_access();
//         inner.tasks[inner.current_task].get_user_token()
//     }
//
//     #[allow(clippy::mut_from_ref)]
//     /// Get the current 'Running' task's trap contexts.
//     fn get_current_trap_cx(&self) -> &mut TrapContext {
//         let inner = self.inner.exclusive_access();
//         inner.tasks[inner.current_task].get_trap_cx()
//     }
//
//     /// Switch current `Running` task to the task we have found,
//     /// or there is no `Ready` task and we can exit with all applications completed
//     fn run_next_task(&self) {
//         if let Some(next) = self.find_next_task() {
//             let mut inner = self.inner.exclusive_access();
//             let current = inner.current_task;
//             inner.tasks[next].task_status = TaskStatus::Running;
//             inner.current_task = next;
//             let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
//             let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
//             drop(inner);
//             // before this, we should drop local variables that must be dropped manually
//             unsafe {
//                 __switch(current_task_cx_ptr, next_task_cx_ptr);
//             }
//             // go back to user mode
//         } else {
//             panic!("All applications completed!");
//         }
//     }
//     fn task_map(&self, start: usize, len: usize, port: usize)->isize{
//         if start&(PAGE_SIZE-1)!=0 {
//             info!("sys_mmap: start address is not page aligned");
//             return -1;
//         }
//         if port>7usize||port==0 {
//             info!("sys_mmap: port number is invalid");
//             return -1;
//         }
//
//         let mut inner = self.inner.exclusive_access();
//         let task_id = inner.current_task;
//         let current_task = &mut inner.tasks[task_id];
//         let memory_set = &mut current_task.memory_set;
//         let start_vpn = VirtPageNum::from(VirtAddr(start));
//         let end_vpn = VirtPageNum::from(VirtAddr(start + len).ceil());
//         for vpn in start_vpn.0..end_vpn.0 {
//             if let Some(pte) = memory_set.translate(VirtPageNum(vpn)) {
//                 if pte.is_valid() {
//                     info!("vpn {} has been occupied!", vpn);
//                     return -1;
//                 }
//             }
//         }
//
//         let permission = MapPermission::from_bits((port as u8) << 1).unwrap() | MapPermission::U;
//         memory_set.insert_framed_area(VirtAddr(start), VirtAddr(start+len), permission);
//         0
//     }
//
// }
//
//
// /// Run the first task in task list.
// pub fn run_first_task() {
//     TASK_MANAGER.run_first_task();
// }
//
// /// Switch current `Running` task to the task we have found,
// /// or there is no `Ready` task and we can exit with all applications completed
// fn run_next_task() {
//     TASK_MANAGER.run_next_task();
// }
//
// /// Change the status of current `Running` task into `Ready`.
// fn mark_current_suspended() {
//     TASK_MANAGER.mark_current_suspended();
// }
//
// /// Change the status of current `Running` task into `Exited`.
// fn mark_current_exited() {
//     TASK_MANAGER.mark_current_exited();
// }
//
// /// Suspend the current 'Running' task and run the next task in task list.
// pub fn suspend_current_and_run_next() {
//     mark_current_suspended();
//     run_next_task();
// }
//
// /// Exit the current 'Running' task and run the next task in task list.
// pub fn exit_current_and_run_next() {
//     mark_current_exited();
//     run_next_task();
// }
//
// /// Get the current 'Running' task's token.
// pub fn current_user_token() -> usize {
//     TASK_MANAGER.get_current_token()
// }
//
// /// Get the current 'Running' task's trap contexts.
// pub fn current_trap_cx() -> &'static mut TrapContext {
//     TASK_MANAGER.get_current_trap_cx()
// }
//
//
// #[allow(dead_code, unused_variables, unused)]
// pub fn mmap(start: usize, len: usize, port: usize) -> isize{
//     // current
//     todo!()
// }