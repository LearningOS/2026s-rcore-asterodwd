use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task, ProcessControlBlock};
use crate::timer::{add_timer, get_time_ms};
use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    // trace!(
    //     "kernel:pid[{}] tid[{}] sys_mutex_lock",
    //     current_task().unwrap().process.upgrade().unwrap().getpid(),
    //     current_task()
    //         .unwrap()
    //         .inner_exclusive_access()
    //         .res
    //         .as_ref()
    //         .unwrap()
    //         .tid
    // );
    let process = current_process();
    let (mutex, detect_deadlock) = {
        let process_inner = process.inner_exclusive_access();
        let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
        (mutex, process_inner.detece_deadlock)
    };

    trace!("detece_deadlock = {}", detect_deadlock);
    if detect_deadlock {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;

        trace!("tid: {}", tid);
        trace!("before check deadlock");
        if check_mutex_deadlock(tid, &process, mutex_id) {
            return -0xdead;
        }
        trace!("end check deadlock");
    }

    drop(process);
    trace!("before mutext lock");
    mutex.lock();
    trace!("end lock");
    0
}

fn check_mutex_deadlock(tid: usize, process: &Arc<ProcessControlBlock>, lock_id: usize) -> bool {
    let process_inner = process.inner_exclusive_access();
    let lock_table = process_inner.mutex_list.clone();
    drop(process_inner);

    let lock_table: Vec<_> = lock_table.iter().flatten().collect();

    let mut owner_list = lock_table[lock_id].get_owner();

    debug!("i'am here");
    // in fact, there is at most one owner!
    while let Some(&owner_tid) = owner_list.first() {
        println!("i'am here2.0");
        if owner_tid == tid {
            return true;
        }

        println!("i'am here2");
        if let Some(&next_lock) = lock_table
            .iter()
            .find(|item| item.get_wait_list().contains(&owner_tid))
        {
            owner_list = next_lock.get_owner();
        } else {
            break;
        }
    }

    false
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let (sem, detect_deadlock, lock_table) = {
        let process_inner = process.inner_exclusive_access();
        (
            Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap()),
            process_inner.detece_deadlock,
            process_inner.semaphore_list.clone(),
        )
    };

    let task = current_task().unwrap();
    if detect_deadlock {
        let mut task_inner = task.inner_exclusive_access();

        let tid = task_inner.res.as_ref().unwrap().tid;

        task_inner.sema_id_waiting_on = Some(sem_id);
        drop(task_inner);

        if check_semaphore_deadlock(tid, sem_id, &lock_table) {
            let mut task_inner = task.inner_exclusive_access();
            task_inner.sema_id_waiting_on = None;
            println!("detected!!!!!!!!!");
            return -0xdead;
        }
    }

    sem.down();

    if detect_deadlock {
        task.inner_exclusive_access().sema_id_waiting_on = None;
    }
    0
}

fn check_semaphore_deadlock(
    tid: usize,
    sem_id: usize,
    lock_table: &[Option<Arc<Semaphore>>],
) -> bool {
    let mut visited_threads: BTreeSet<usize> = BTreeSet::new();
    let mut stack = Vec::new();
    stack.push(sem_id);

    while let Some(sem_id) = stack.pop() {
        // I think there must be a semaphore with the sem_id
        let sem_obj = lock_table[sem_id].as_ref().unwrap();
        let own_list = sem_obj.get_owner();

        for owner_tid in own_list {
            if tid == owner_tid {
                return true;
            }

            if !visited_threads.contains(&owner_tid) {
                visited_threads.insert(owner_tid);

                if let Some(next_lock) = find_what_sem_is_thread_waiting_for(owner_tid) {
                    stack.push(next_lock);
                }
            }
        }
    }
    false
}

fn find_what_sem_is_thread_waiting_for(tid: usize) -> Option<usize> {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let task = &process_inner.tasks[tid];

    let waiting = task
        .as_ref()
        .unwrap()
        .inner_exclusive_access()
        .sema_id_waiting_on;

    waiting
}

fn _check_semaphore_deadlock2(
    tid: usize,
    process: &Arc<ProcessControlBlock>,
    sem_id: usize,
) -> bool {
    let mut visited_threads: BTreeSet<usize> = BTreeSet::new();

    let process_inner = process.inner_exclusive_access();
    let lock_table = process_inner.semaphore_list.clone();
    drop(process_inner);

    let lock_table: Vec<_> = lock_table.clone().into_iter().flatten().collect();

    _dfs(tid, sem_id, &lock_table, &mut visited_threads)
}

fn _dfs(
    tid: usize,
    sem_id: usize,
    lock_table: &[Arc<Semaphore>],
    visited: &mut BTreeSet<usize>,
) -> bool {
    let target_lock = &lock_table[sem_id];
    if target_lock.inner.exclusive_access().count > 0 {
        return false;
    }
    let mut results = Vec::new();
    let mut owner_list = lock_table[sem_id].get_owner();

    while let Some(owner_tid) = owner_list.pop() {
        if owner_tid == tid {
            results.push(true);
        }

        if visited.contains(&tid) {
            continue;
        }

        if let Some(next_lock) = lock_table
            .iter()
            .find(|&item| item.get_wait_list().contains(&owner_tid))
        {
            owner_list = next_lock.get_owner();
            // dfs(owner_id, next_lock, lock_table, visited);
        } else {
            break;
        }
    }

    results.iter().all(|item| *item)
}

/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    process_inner.detece_deadlock = true;
    0
}
