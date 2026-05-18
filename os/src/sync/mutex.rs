//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_process, TaskControlBlock};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// get owner
    fn get_owner(&self) -> Vec<usize>;
    /// get wait_list
    fn get_wait_list(&self) -> Vec<usize>;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }

    fn get_owner(&self) -> Vec<usize> {
        Vec::new()
    }

    fn get_wait_list(&self) -> Vec<usize> {
        Vec::new()
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,

    // saves tid in this vec
    owner: Vec<usize>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                    owner: Vec::new(),
                })
            },
        }
    }

    // fn set_owner(&self, tid: usize) {
    //     let mut inner = self.inner.exclusive_access();
    //     inner.owner.clear();
    //     inner.owner.push(tid);
    // }
    //
    // fn reset_owner(&self) {
    //     let mut inner = self.inner.exclusive_access();
    //     inner.owner.clear();
    // }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let task = current_task().unwrap();
        let tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;

        trace!("i'am here in lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(task);
            drop(mutex_inner);
            block_current_and_run_next();

            // // when lock is transferred form t1->t2, else block is not executed,
            // // so we have to set the owner when the thread wake up
            // if current_process().inner_exclusive_access().detece_deadlock {
            //     self.set_owner(tid);
            // }
        } else {
            mutex_inner.locked = true;

            trace!("before set owner");
            if current_process()
                .inner_exclusive_access()
                .detect_deadlock_enabled()
            {
                trace!("setting owner after access current_process");
                mutex_inner.owner.clear();
                mutex_inner.owner.push(tid);
            }
            trace!("end set owner");
        }

        // we can't move detect logic here because when thread is waken up, and execution is return
        // after block_current_and_run_next(), we have already dropped mutex_inner.
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let tid = waking_task
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .tid;

            mutex_inner.owner.clear();
            mutex_inner.owner.push(tid);
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
            mutex_inner.owner.clear();
        }
    }
    fn get_owner(&self) -> Vec<usize> {
        self.inner.exclusive_access().owner.clone()
    }

    fn get_wait_list(&self) -> Vec<usize> {
        self.inner
            .exclusive_access()
            .wait_queue
            .iter()
            .map(|task| task.inner_exclusive_access().res.as_ref().unwrap().tid)
            .collect::<Vec<_>>()
    }
}
