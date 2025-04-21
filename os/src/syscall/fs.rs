//! File and filesystem-related syscalls

use crate::fs::{open_file, OpenFlags, Stat, ROOT_INODE};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

/// 根据fd读取进程fd_table中的file
/// 将buf+len包装成一个UserBuffer
/// 将buf写入file
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// 根据fd读取进程fd_table中的file
/// 将buf+len包装成一个UserBuffer
/// 将file写入buf
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = inner.fd_table[fd].clone() {
        drop(inner);
        trace!("try get stat");
        let stat = file.get_stat();
        trace!("got stat");
        // unsafe { *st = stat };

        // 使用translated_refmut直接获取用户空间的可变引用
        let user_stat = translated_refmut(token, st);
        *user_stat = stat;

        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name_ptr: *const u8, new_name_ptr: *const u8) -> isize {
    // resolve the name
    let token = current_user_token();

    let old_path = translated_str(token, old_name_ptr);
    let new_path = translated_str(token, new_name_ptr);
    if old_path == new_path {
        trace!("kernel: sys_linkat failed: new file has the same name");
        return -1;
    }
    trace!(
        "kernel: pid[{}] sys_linkat {:?} at {:?}",
        current_task().unwrap().pid.0,
        old_path,
        new_path
    );
    // find old_name inode
    if ROOT_INODE.find(&old_path).is_none() {
        trace!("kernel: sys_linkat failed: old file not found");
        return -1;
    };
    //
    ROOT_INODE.create_hard_link(&new_path, &old_path);
    //
    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name_ptr: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, name_ptr);
    trace!(
        "kernel:pid[{}] sys_unlinkat {:?}",
        current_task().unwrap().pid.0,
        path
    );
    // check file inode exit
    if ROOT_INODE.find(&path).is_none() {
        trace!("kernel: sys_linkat failed: old file not found");
        return -1;
    };
    //
    ROOT_INODE.remove_hard_link(&path);

    0
}
