//! Types related to task management

use super::TaskContext;
pub const MAX_SYSCALL_NUM: usize = 500;
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    pub first_time_run:usize,//第一次运行的时间
    pub syscall_times:[u32;MAX_SYSCALL_NUM]//第一次运行的时间,那么
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
