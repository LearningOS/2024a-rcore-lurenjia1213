//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.

use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::mm::{MapPermission,VirtPageNum,VirtAddr};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
use crate::timer::get_time_ms;
use crate::config::MAX_SYSCALL_NUM;
/// Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,

    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            //判断一下这是第几次？
            if task_inner.first_time_run==0{
                task_inner.first_time_run=get_time_ms();
            }

            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
#[allow(unused)]
///insert_framed_area
pub fn insert_framed_area(start_va:VirtAddr,end_va:VirtAddr,perm:MapPermission){
    current_task().unwrap().inner_exclusive_access().memory_set.insert_framed_area(start_va, end_va, perm);
}
#[allow(unused)]
///原先是unmap  xxxx，改改~
pub fn remove_framed_area(
    
    start_va: VirtAddr,
    end_va: VirtAddr,
    //permission: MapPermission,
) {
    
    let start_vpn :VirtPageNum=start_va.floor();
    let end_vpn :VirtPageNum=end_va.ceil().0.into();
    for vpn in start_vpn.0..end_vpn.0{
        current_task().unwrap().inner_exclusive_access().memory_set.page_table.unmap(vpn.into());
    }
}
#[allow(unused)]
///as you see
pub fn syscall_counter(syscall_id:usize){
    current_task().unwrap().inner_exclusive_access().syscall_times[syscall_id]+=1;
    //drop(inner);
    //ret
}
#[allow(unused)]
///ch3
pub fn get_first_time_run()->usize{
    /*let inner = self.inner.exclusive_access();
    let current = inner.current_task;
    let ret=inner.tasks[current].first_time_run;
    drop(inner);
    ret*/
    current_task().unwrap().inner_exclusive_access().first_time_run
}
#[allow(unused)]
///获取系统调用计数
pub fn get_syscall_times()->[u32;MAX_SYSCALL_NUM]{
    current_task().unwrap().inner_exclusive_access().syscall_times
}