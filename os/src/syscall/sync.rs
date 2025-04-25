use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;

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
        "kernel: pid[{}] tid[{}] sys_mutex_create",
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
    trace!("borrow process inner");
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        // 覆盖
        if process_inner.deadlock_detect_enable() {
            trace!("deadlock detect enabled");
            // 添加锁
            process_inner.remove_mutex_by_id(id);
            trace!(
                "kernel: pid[{}] tid[{}] sys_mutex_create: remove mutex {}",
                current_task().unwrap().process.upgrade().unwrap().getpid(),
                current_task()
                    .unwrap()
                    .inner_exclusive_access()
                    .res
                    .as_ref()
                    .unwrap()
                    .tid,
                id
            );
            process_inner.create_mutex(Some(id));
            trace!(
                "kernel: pid[{}] tid[{}] sys_mutex_create: cover mutex {}",
                current_task().unwrap().process.upgrade().unwrap().getpid(),
                current_task()
                    .unwrap()
                    .inner_exclusive_access()
                    .res
                    .as_ref()
                    .unwrap()
                    .tid,
                id
            );
        }
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        if process_inner.deadlock_detect_enable() {
            // 添加锁
            process_inner.create_mutex(None);
            trace!(
                "kernel: pid[{}] tid[{}] sys_mutex_create: create new mutex",
                current_task().unwrap().process.upgrade().unwrap().getpid(),
                current_task()
                    .unwrap()
                    .inner_exclusive_access()
                    .res
                    .as_ref()
                    .unwrap()
                    .tid,
            );
        }
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    // 获取当前线程ID
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_mutex_lock", pid, tid);
    let process = current_process();

    // 检查是否启用了死锁检测
    if process.deadlock_detect_enable() {
        trace!(
            "kernel:pid[{}] tid[{}] sys_mutex_lock deadlock detect",
            pid,
            tid
        );
        let process_inner = process.inner_exclusive_access();
        // 获取死锁检测器
        let mutex_detector = process_inner.deadlock_detector.clone().unwrap()[0].clone();
        let mutex_num = mutex_detector.inner.exclusive_access().available.len();

        // 创建请求向量，对于mutex锁，只需要请求1个资源
        let mut request = vec![0; mutex_num];
        request[mutex_id] = 1;

        // 检查是否安全
        if !mutex_detector.is_safe_state(tid, request) {
            trace!(
                "kernel:pid[{}] tid[{}] sys_mutex_lock deadlock detect failed!",
                pid,
                tid
            );
            // 不安全，可能导致死锁，返回错误
            return -0xdead;
        }
    }

    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock deadlock detect pass!",
        pid,
        tid
    );
    // lock
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    // 获取当前线程ID
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_mutex_lock", pid, tid);

    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    // 检查是否启用了死锁检测
    if process_inner.deadlock_detect_enable() {
        // 获取死锁检测器
        let mutex_detector = process_inner.deadlock_detector.clone().unwrap()[0].clone();
        let mutex_num = mutex_detector.inner.exclusive_access().available.len();

        // 创建请求向量，对于mutex锁，只需要请求1个资源
        let mut request = vec![0; mutex_num];
        request[mutex_id] = 1;

        // 释放资源
        mutex_detector.detector_unlock(tid, request);
    }

    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    // 获取当前线程ID
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_create", pid, tid);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        // 覆盖
        if process_inner.deadlock_detect_enable() {
            trace!("deadlock detect enabled");
            // 添加锁
            process_inner.remove_semaphore_by_id(id);
            trace!(
                "kernel: pid[{}] tid[{}] sys_semaphore_create: remove semaphore {}",
                pid,
                tid,
                id
            );
            process_inner.create_semaphore(Some(id), res_count);
            trace!(
                "kernel: pid[{}] tid[{}] sys_semaphore_create: cover semaphore {}",
                pid,
                tid,
                id
            );
        }

        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        if process_inner.deadlock_detect_enable() {
            // 添加锁
            process_inner.create_semaphore(None, res_count);
            trace!(
                "kernel: pid[{}] tid[{}] sys_semaphore_create: create new semaphore",
                pid,
                tid,
            );
        }

        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_up", pid, tid);
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    // 检查是否启用了死锁检测
    if process_inner.deadlock_detect_enable() {
        // 获取死锁检测器
        let semaphore_detector = process_inner.deadlock_detector.clone().unwrap()[0].clone();
        let semaphore_num = semaphore_detector.inner.exclusive_access().available.len();

        // 创建请求向量
        let mut request = vec![0; semaphore_num];
        request[sem_id] = 1;

        // 释放资源
        semaphore_detector.detector_unlock(tid, request);
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_down", pid, tid);

    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    // 检查是否启用了死锁检测
    if process_inner.deadlock_detect_enable() {
        trace!(
            "kernel:pid[{}] tid[{}] sys_semaphore_down deadlock detect",
            pid,
            tid
        );
        // 获取死锁检测器
        let semaphore_detector = process_inner.deadlock_detector.clone().unwrap()[0].clone();
        let semaphore_num = semaphore_detector.inner.exclusive_access().available.len();

        // 创建请求向量
        let mut request = vec![0; semaphore_num];
        request[sem_id] = 1;

        // 检查是否安全
        if !semaphore_detector.is_safe_state(tid, request) {
            trace!(
                "kernel:pid[{}] tid[{}] sys_semaphore_down deadlock detect failed!",
                pid,
                tid
            );
            // 不安全，可能导致死锁，返回错误
            return -0xdead;
        }
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    sem.down();
    0
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
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if enabled == 1 {
        process_inner.enable_deadlock_detect();
        0
    } else if enabled == 0 {
        process_inner.disable_deadlock_detect();
        0
    } else {
        -1
    }
}
