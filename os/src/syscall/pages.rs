//! memory and page system related syscalls

/// map a file to memory
/// alloc memory only for now
///
/// start: usize a page algin virtual address
/// len: len of file
/// port:usize  [0..=2] is R W X  , other is 0
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    todo!()
}
