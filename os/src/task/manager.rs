//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let min_stride_task_index = self.ready_queue
            .iter()
            .enumerate()
            .fold(None, |min_index, (index, task)| {
                match min_index {
                    None => Some(index),
                    Some(min_idx) => {
                        if task.inner_exclusive_access().stride < self.ready_queue[min_idx].inner_exclusive_access().stride {
                            Some(index)
                        } else {
                            min_index
                        }
                    }
                }
            });
    
        // 如果没有找到任务，返回 None
        let index = match min_stride_task_index {
            Some(i) => i,
            None => return None,
        };
        let task = &self.ready_queue[index];
        let pass = task.inner_exclusive_access().pass;
        task.inner_exclusive_access().stride += pass;
        let removed_task = self.ready_queue.remove(index);
        removed_task
    }    
    //在这过程中，gpt参与修改了部分代码
    
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
