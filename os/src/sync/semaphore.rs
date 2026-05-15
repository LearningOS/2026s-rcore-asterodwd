//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{
    block_current_and_run_next, current_process, current_task, wakeup_task, TaskControlBlock,
};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
    pub owner: Vec<usize>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    owner: Vec::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let detect_deadlock = {
            let process = current_process();
            let ret = process.inner_exclusive_access().detece_deadlock;
            ret
        };

        let mut inner = self.inner.exclusive_access();
        inner.count += 1;

        if detect_deadlock {
            let task = current_task().unwrap();
            let tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;
            if let Some(pos) = inner.owner.iter().position(|&id| id == tid) {
                inner.owner.remove(pos);
            }
        }

        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let detect_deadlock = {
            let process = current_process();
            let ret = process.inner_exclusive_access().detece_deadlock;
            ret
        };

        let mut inner = self.inner.exclusive_access();

        let task = current_task().unwrap();
        let tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;

        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(task);
            drop(inner);
            block_current_and_run_next();

            if detect_deadlock {
                let mut lock_inner = self.inner.exclusive_access();
                lock_inner.owner.push(tid);
            }
        } else if detect_deadlock {
            inner.owner.push(tid);
        }
    }

    /// get all owners of this semaphore
    pub fn get_owner(&self) -> Vec<usize> {
        self.inner.exclusive_access().owner.clone()
    }

    /// get all threads that wait on this semaphore
    pub fn get_wait_list(&self) -> Vec<usize> {
        self.inner
            .exclusive_access()
            .wait_queue
            .iter()
            .map(|task| task.inner_exclusive_access().res.as_ref().unwrap().tid)
            .collect::<Vec<_>>()
    }
}
