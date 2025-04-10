//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE,
    loader::get_app_data_by_name,
    mm::{
        translated_byte_buffer, translated_refmut, translated_str, unused_phy_pages, MapPermission,
        VirtAddr,
    },
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskControlBlock,
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    let time_val_byte_len = core::mem::size_of::<TimeVal>();
    let ts_addr_byte_buffer =
        translated_byte_buffer(current_user_token(), ts as *mut u8, time_val_byte_len);

    let us = get_time_us();

    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let time_val_byte = unsafe {
        core::slice::from_raw_parts(&time_val as *const TimeVal as *const u8, time_val_byte_len)
    };

    let mut byte_copied = 0;
    for buffer in ts_addr_byte_buffer.into_iter() {
        let byte_copied_this_time = buffer.len().min(time_val_byte_len - byte_copied);
        buffer[..byte_copied_this_time]
            .copy_from_slice(&time_val_byte[byte_copied..byte_copied + byte_copied_this_time]);
        byte_copied += byte_copied_this_time;
    }

    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    // check input valid
    // * start should be page-aligned
    // * prot[2..] should be 0
    // * prot=0 is meansless
    if (start % PAGE_SIZE != 0) || (prot & !0x7 != 0) || (prot & 0x7 == 0) {
        return -1;
    }
    // TODO, should alloc even len is 0
    if len == 0 {
        return 0;
    };
    // check useable physical address
    let available_pages = unused_phy_pages();
    let required_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE; // 向上取整
    if required_pages > available_pages {
        return -1;
    }
    // alloc memory
    // * check pages
    // ** vpn
    let start_va: VirtAddr = start.into();
    let start_vpn = start_va.floor();
    let end_va: VirtAddr = (start + len).into();
    let end_vpn = end_va.ceil();
    // ** memory_set exclusive_access
    let current_tcb = match current_task() {
        Some(v) => v,
        None => return -1,
    };
    let mut inner = current_tcb.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;
    // ** loop check pages valid
    let mut cur_vpn = start_vpn;
    while cur_vpn < end_vpn {
        if let Some(pte) = memory_set.translate(cur_vpn) {
            if pte.is_valid() {
                trace!("kernel: sys_mmap: page with vpn {:x?} is mapped", cur_vpn);
                return -1;
            }
        }
        cur_vpn.0 += 1
    }
    // * alloc
    let map_permission = MapPermission::from_bits_truncate((prot as u8) << 1) | MapPermission::U;
    memory_set.insert_framed_area(start_va, end_va, map_permission);
    trace!("kernel: sysmmap: alloc area");
    // success
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // check input valid
    // * start should be page-aligned
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    };
    // dealloc
    // ** vpn
    let start_va: VirtAddr = start.into();
    let start_vpn = start_va.floor();
    let end_va: VirtAddr = (start + len).into();
    let end_vpn = end_va.ceil();
    // ** memory_set exclusive_access
    let current_tcb = match current_task() {
        Some(v) => v,
        None => return -1,
    };
    let mut inner = current_tcb.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;

    let mut cur_vpn = start_vpn;
    while cur_vpn < end_vpn {
        if let Some(pte) = memory_set.translate(cur_vpn) {
            if !pte.is_valid() {
                trace!("sys_munmap: page with vpn {:x?} is not mapped", cur_vpn);
                drop(inner);
                return -1;
            }
        }

        memory_set.remove_area_with_start_vpn(cur_vpn);
        cur_vpn.0 += 1
    }

    // success
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    // new BLK from elf
    let token = current_user_token();
    let path = translated_str(token, path);
    let elf_data = match get_app_data_by_name(path.as_str()) {
        Some(v) => v,
        None => return -1,
    };

    let new_task = TaskControlBlock::new(elf_data);
    let new_pid = new_task.pid.0;

    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;

    let binding = current_task().unwrap();
    let mut parent_inner = binding.inner_exclusive_access();

    // add parent
    new_task.inner_exclusive_access().parent = Some(Arc::downgrade(&binding));
    // add child
    let arc_new_task = Arc::new(new_task);
    parent_inner.children.push(arc_new_task.clone());

    // add to line
    add_task(arc_new_task);

    new_pid as isize
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    if prio < 2 {
        return -1;
    }
    let current_tcb = match current_task() {
        Some(v) => v,
        None => return -1,
    };
    let mut inner = current_tcb.inner_exclusive_access();
    inner.prio = prio as usize;
    prio
}
