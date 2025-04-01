use crate::config::MAX_SYSCALL_NUM;

/// record task syscall times
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TaskSyscallCount(pub [usize; MAX_SYSCALL_NUM]);

impl TaskSyscallCount {
    pub fn zero_init() -> Self {
        Self([0; MAX_SYSCALL_NUM])
    }
}
