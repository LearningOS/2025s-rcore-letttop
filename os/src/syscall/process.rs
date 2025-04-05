//! Process management syscalls

use crate::config::PAGE_SIZE;
use crate::mm::frame_allocator::FRAME_ALLOCATOR;
use crate::mm::{MapPermission, PageTableEntry};
use crate::{
    mm::{translated_byte_buffer, PageTable, PhysAddr, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TASK_MANAGER,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
/// should handle by fn translated_byte_buffer
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

    // for (i, &byte) in time_val_byte.iter().enumerate() {
    //     *ts_addr_byte_buffer[i] = byte;
    // }

    let mut byte_copied = 0;
    for buffer in ts_addr_byte_buffer.into_iter() {
        let byte_copied_this_time = buffer.len().min(time_val_byte_len - byte_copied);
        buffer[..byte_copied_this_time]
            .copy_from_slice(&time_val_byte[byte_copied..byte_copied + byte_copied_this_time]);
        byte_copied += byte_copied_this_time;
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            // let ptr: *mut u8 = id as *mut u8;
            // let ptr_byte = unsafe { *ptr };
            // ptr_byte as isize

            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            //
            let va = VirtAddr::from(id);
            let vpn = va.floor();
            let offset = va.page_offset();
            //
            let pte = match page_table.translate(vpn) {
                Some(pte) => pte,
                None => return -1,
            };
            if !pte.is_valid() || !pte.readable() {
                return -1;
            }
            let ptr = PhysAddr::from(pte.ppn()).0 + offset;
            unsafe { *(ptr as *const u8) as isize }
        }
        1 => {
            // let ptr: *mut u8 = id as *mut u8;
            // unsafe {
            //     *ptr = data as u8;
            // }

            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            //
            let va = VirtAddr::from(id);
            let vpn = va.floor();
            let offset = va.page_offset();
            //
            let pte = match page_table.translate(vpn) {
                Some(pte) => pte,
                None => return -1,
            };
            if !pte.is_valid() || !pte.writable() {
                return -1;
            }
            let ptr = PhysAddr::from(pte.ppn()).0 + offset;
            unsafe { *(ptr as *mut u8) = data as u8 }

            0
        }
        2 => {
            // read count
            TASK_MANAGER.get_current_task_syscall_count(id) as isize
        }
        _ => -1,
    }
}

/// Map a file to memory
///
/// Currently only allocates memory without actual file mapping.
///
/// # Arguments
/// * 'start' - A page-aligned virtual address
/// * 'len'   - Byte length of the file
/// * 'prot'  - Protection bits: [0..=2] for R W X, others are 0
///
/// # Returns
/// * '0'     - on success
/// * '-1'    - on error
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    // start should be page-aligned
    // prot[2..] should be 0
    // prot=0 is meansless
    if (start % PAGE_SIZE != 0) || (prot & !0x7 != 0) || (prot & 0x7 == 0) {
        return -1;
    }
    if len == 0 {
        return 0;
    };
    // check useable physical address
    let frame_allocator = FRAME_ALLOCATOR.exclusive_access();
    let available_pages = frame_allocator.unused_phy_pages();
    drop(frame_allocator);
    //
    let required_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE; // 向上取整
    if required_pages > available_pages {
        return -1;
    }
    // alloc memory
    // change from translated_byte_buffer
    let map_permission = MapPermission::from_bits((prot as u8) << 1).unwrap() | MapPermission::U;
    //
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task_id = inner.current_task;
    let current_task = &mut inner.tasks[current_task_id];
    let memory_set = &mut current_task.memory_set;

    //
    let mut start_va = start;
    let end_va = start + len;
    //debug
    println!("start va is {:x}", start_va);
    println!("end   va is {:x}", end_va);

    while start_va < end_va {
        let vpn = VirtAddr::from(start_va).floor();

        if memory_set.translate(vpn).is_some() {
            println!("page with vpn {:x?} is mapped", vpn);
            drop(inner);
            return -1;
        }

        println!(
            "map new space {:x} to {:x}",
            start_va,
            start_va + PAGE_SIZE - 1
        );
        // debug
        {
            if memory_set.translate((vpn.0 + 1).into()).is_some() {
                println!("next page is mapped before");
            } else {
                println!("page with vpn {:x?} is not mapped before", (vpn.0 + 1));
            }
            if memory_set.translate((vpn.0 + 2).into()).is_some() {
                println!("next page is mapped before");
            } else {
                println!("page with vpn {:x?} is not mapped before", (vpn.0 + 2));
            }
            if memory_set.translate((vpn.0 + 3).into()).is_some() {
                println!("next page is mapped before");
            } else {
                println!("page with vpn {:x?} is not mapped before", (vpn.0 + 3));
            }
        }

        // 创建新的映射区域
        memory_set.insert_framed_area(
            start_va.into(),                   // 页开头
            (start_va + PAGE_SIZE - 1).into(), // 页结束
            map_permission,                    // R W X U
        );
        // debug
        {
            if memory_set.translate((vpn.0 + 1).into()).is_some() {
                println!("page with vpn {:x?} is mapped after", (vpn.0 + 1));
            } else {
                println!("page with vpn {:x?} is not mapped after", (vpn.0 + 1));
            }
            if memory_set.translate((vpn.0 + 2).into()).is_some() {
                println!("page with vpn {:x?} is mapped after", (vpn.0 + 2));
            } else {
                println!("page with vpn {:x?} is not mapped after", (vpn.0 + 2));
            }
            if memory_set.translate((vpn.0 + 3).into()).is_some() {
                println!("page with vpn {:x?} is mapped after", (vpn.0 + 3));
            } else {
                println!("page with vpn {:x?} is not mapped after", (vpn.0 + 3));
            }
        }
        start_va += PAGE_SIZE;
    }

    // success
    drop(inner);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // start should be page-aligned
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    };
    //
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task_id = inner.current_task;
    let current_task = &mut inner.tasks[current_task_id];
    let memory_set = &mut current_task.memory_set;
    //
    let mut start_va = start;
    let end_va = start + len * 8;

    while start_va < end_va {
        let vpn = VirtAddr::from(start_va).floor();
        if let Some(pte) = memory_set.page_table.find_pte(vpn) {
            if !pte.is_valid() {
                return -1;
            }
            *pte = PageTableEntry::empty();
        } else {
            drop(inner);
            return -1;
        }

        start_va += PAGE_SIZE;
    }
    // success
    drop(inner);
    0
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
