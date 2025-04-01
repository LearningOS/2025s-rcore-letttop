use alloc::collections::BTreeMap;

/// record task syscall times
#[derive(Clone, PartialEq, Eq)]
pub struct TaskSyscallCount(pub BTreeMap<usize, usize>);
// pub struct TaskSyscallCount(pub [usize; MAX_SYSCALL_NUM]);

impl TaskSyscallCount {
    pub fn zero_init() -> Self {
        Self(BTreeMap::new())
    }
}
