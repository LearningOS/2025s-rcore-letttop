//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
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
        let mut min_stride_index = 0;
        let mut min_stride = self
            .ready_queue
            .front()
            .unwrap()
            .inner_exclusive_access()
            .stride;
        for (index, task) in self.ready_queue.iter().enumerate() {
            let cur_stride = task.inner_exclusive_access().stride;
            if cur_stride < min_stride {
                min_stride = cur_stride;
                min_stride_index = index;
            }
        }

        if let Some(next_task) = self.ready_queue.remove(min_stride_index) {
            let next_task_prio = next_task.inner_exclusive_access().prio;
            next_task.inner_exclusive_access().stride += BIG_STRIDE / next_task_prio;
            Some(next_task)
        } else {
            None
        }
    }
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

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
