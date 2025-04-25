use super::UPSafeCell;
use alloc::vec;
use alloc::vec::Vec;

/// 进程的死锁检测器
/// 检测器对每个类型的资源都有一份,mutex一份,semaphore一份
pub struct DeadlockDetector {
    /// 成员
    pub inner: UPSafeCell<DeadlockDetectorInner>,
}

pub struct DeadlockDetectorInner {
    /// 可利用资源向量
    /// Available[j] = k，表示第 j 类资源的可用数量为 k
    pub available: Vec<i32>,
    /// 分配矩阵
    /// Allocation[i,j] = g，则表示线程 i 当前己分得第 j 类资源的数量为 g
    pub allocation: Vec<Vec<i32>>,
    /// 需求矩阵
    /// Need[i,j] = d，表示线程 i 还需要第 j 类资源的数量为 d
    pub need: Vec<Vec<i32>>,
}

impl DeadlockDetector {
    /// 创建一个空的
    pub fn new() -> Self {
        DeadlockDetector {
            inner: unsafe { UPSafeCell::new(DeadlockDetectorInner::new()) },
        }
    }

    /// 检查分配某个资源是否安全
    pub fn is_safe_state(&self, tid: usize, request: Vec<i32>) -> bool {
        let mut inner = self.inner.exclusive_access();
        let t_num = inner.allocation.len();
        let r_num = inner.available.len();
        trace!(
            "[kernel] DeadlockDetector: task id: {}, resource count: {}",
            t_num,
            r_num
        );
        // 检查输入合法性
        if tid > t_num {
            return false;
        }
        for (j, rqst) in request.iter().enumerate() {
            if rqst > &inner.available[j] {
                trace!(
                    "[kernel] DeadlockDetector: request resource id {} for {} failed, available {}",
                    j,
                    rqst,
                    &inner.available[j]
                );
                return false;
            }
        }

        // 假设 分配资源
        for (j, rqst) in request.iter().enumerate() {
            inner.available[j] -= rqst;
            inner.allocation[tid][j] += rqst;
            inner.need[tid][j] -= rqst;
        }

        // 两个工作向量
        let mut work_list = inner.available.clone();
        let mut finish_list = vec![false; inner.allocation.len()];

        // 多次遍历检查线程是否可完成
        loop {
            let mut found = false;
            for i in 0..t_num {
                if finish_list[i] {
                    continue;
                }
                // 检查是否有足够的资源
                let mut can_finish = true;
                for j in 0..r_num {
                    if inner.need[i][j] > work_list[j] {
                        can_finish = false;
                        break;
                    }
                }

                if !can_finish {
                    continue;
                }

                found = true;
                finish_list[i] = true;
                for j in 0..r_num {
                    work_list[j] += inner.allocation[i][j];
                }
            }
            if !found {
                break;
            }
        }

        // 检查是否全部可完成
        if finish_list.iter().all(|finish| *finish) {
            true
        } else {
            // 还原 分配资源
            for (j, rqst) in request.iter().enumerate() {
                inner.available[j] += rqst;
                inner.allocation[tid][j] -= rqst;
                inner.need[tid][j] += rqst;
            }
            false
        }
    }

    /// 释放资源
    pub fn detector_unlock(&self, tid: usize, request: Vec<i32>) {
        let mut inner = self.inner.exclusive_access();
        let t_num = inner.allocation.len();
        let r_num = inner.available.len();
        trace!(
            "[kernel] DeadlockDetector: task id: {}, resource count: {}",
            t_num,
            r_num
        );

        // 恢复分配资源
        for (j, rqst) in request.iter().enumerate() {
            inner.available[j] += rqst;
            inner.allocation[tid][j] -= rqst;
            // 不再需要了
            // inner.need[tid][j] += rqst;
        }
    }
}

impl DeadlockDetectorInner {
    pub fn new() -> Self {
        DeadlockDetectorInner {
            available: Vec::new(),
            allocation: Vec::new(),
            need: Vec::new(),
        }
    }
}
