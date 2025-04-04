use crate::config::MAX_SYSCALL_NUM;

/// record task syscall times
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TaskSyscallCount(pub [(usize, usize); MAX_SYSCALL_NUM]);
// pub struct TaskSyscallCount(pub [usize; MAX_SYSCALL_NUM]);

impl TaskSyscallCount {
    pub fn zero_init() -> Self {
        TaskSyscallCount([(usize::MAX, 0); MAX_SYSCALL_NUM])
    }

    pub fn add_syscall_id_count(&mut self, syscall_id: usize) {
        // find, return
        for (id, value) in self.0.iter_mut() {
            if *id == syscall_id {
                *value += 1;
                return;
            }
        }
        // not find, add
        for (id, value) in self.0.iter_mut() {
            if *id == usize::MAX {
                *id = syscall_id;
                *value = 1;
                return;
            }
        }

        //
        panic!("should not be here, maybe too many kinds of syscall");
    }

    pub fn read_syscall_id_count(&self, syscall_id: usize) -> usize {
        // search
        // find, return
        for (id, value) in self.0.iter() {
            if *id == syscall_id {
                return *value;
            }
        }
        // not find
        0
    }
}
