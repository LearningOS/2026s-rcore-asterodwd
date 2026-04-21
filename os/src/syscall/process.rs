//! Process management syscalls
use crate::{
    mm::{translated_byte_buffer, PageTable},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_count,
        suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    let time = get_time_us();
    let time_val = TimeVal {
        sec: time / 1_000_000,
        usec: time % 1_000_000,
    };

    let len = core::mem::size_of::<TimeVal>();
    let src = unsafe { core::slice::from_raw_parts(&time_val as *const TimeVal as *const u8, len) };

    let user_buffer = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );

    if user_buffer.is_empty() {
        return -1;
    }

    let mut current_offset = 0_usize;

    for slice in user_buffer {
        slice.copy_from_slice(&src[current_offset..(current_offset + slice.len())]);
        current_offset += slice.len();
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        0 => {
            let page_table = PageTable::from_token(current_user_token());
            let pa = page_table.lookup(id.into()).unwrap();

            *pa.get_mut::<u8>() as isize
            // unsafe { *(id as *const u8) as isize }
        }
        2 => get_syscall_count(id) as isize,
        // TODO: not implement yet
        1 => {
            let page_table = PageTable::from_token(current_user_token());
            let pa = page_table.lookup(id.into()).unwrap();

            *pa.get_mut::<u8>() = data as u8;

            // unsafe { *(id as *mut u8) = data as u8 };
            0
        }
        _ => unreachable!("not going to reaching here"),
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
