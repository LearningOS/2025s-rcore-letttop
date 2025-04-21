use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::trace;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    ///0
    pub block_id: usize,
    ///0
    pub block_offset: usize,
    /// used in inode stat
    pub fs: Arc<Mutex<EasyFileSystem>>,
    ///0
    pub block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    pub fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            let read_sz =
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device);
            if read_sz != DIRENT_SZ {
                trace!("[find_inode_id][ERROR] read_sz != DIRENT_SZ, i = {}, read_sz = {}, expected = {}", i, read_sz, DIRENT_SZ);
            }
            if dirent.name() == name {
                return Some(dirent.inode_id());
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                let read_sz =
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device);
                if read_sz != DIRENT_SZ {
                    trace!(
                        "[ls][ERROR] read_sz != DIRENT_SZ, i = {}, read_sz = {}, expected = {}",
                        i,
                        read_sz,
                        DIRENT_SZ
                    );
                }
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        // 首先获取需要释放的数据块，但在单独的作用域中使用锁
        let data_blocks_dealloc = {
            let mut fs = self.fs.lock();
            let blocks = self.modify_disk_inode(|disk_inode| {
                let size = disk_inode.size;
                let blocks = disk_inode.clear_size(&self.block_device);
                trace!("here?");
                assert!(blocks.len() == DiskInode::total_blocks(size) as usize);
                trace!("no");
                blocks
            });
            drop(fs); // 显式释放锁
            blocks
        };

        // 然后在另一个作用域中释放数据块
        if !data_blocks_dealloc.is_empty() {
            let mut fs = self.fs.lock();
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
            drop(fs); // 显式释放锁
        }

        block_cache_sync_all();
    }

    /// Create hard link dir_entry under current inode by name
    pub fn create_hard_link(&self, new_name: &str, old_name: &str) {
        let mut fs = self.fs.lock();

        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // find old inode id
            let old_inode_id = self.find_inode_id(old_name, root_inode).unwrap();

            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(new_name, old_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        block_cache_sync_all();
        // release efs lock automatically by compiler
    }

    /// remove hard link dir_entry under current inode by name
    pub fn remove_hard_link(&self, name: &str) {
        let mut inode_id = None;

        // 第一部分：查找并移除目录项
        {
            let mut fs = self.fs.lock();
            self.modify_disk_inode(|root_inode| {
                let file_count = (root_inode.size as usize) / DIRENT_SZ;

                // 读取所有 dirent
                let mut dirents = Vec::new();
                for i in 0..file_count {
                    let mut dirent = DirEntry::empty();
                    root_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device);
                    dirents.push(dirent);
                }

                // 找到要删除的项
                let pos_opt = dirents.iter().position(|e| e.name() == name);
                trace!(
                    "[remove_hard_link] search pos for name = {:?}, result = {:?}, dirents = {:?}",
                    name,
                    pos_opt,
                    dirents.iter().map(|e| e.name()).collect::<Vec<_>>()
                );
                if pos_opt.is_none() {
                    trace!(
                        "[remove_hard_link] ERROR: name not found in dirents, name = {:?}",
                        name
                    );
                    return;
                }
                let pos = pos_opt.unwrap();
                inode_id = Some(dirents[pos].inode_id());
                trace!(
                    "[remove_hard_link] before, file_count = {}, dirents.len() = {}, pos = {}",
                    file_count,
                    dirents.len(),
                    pos
                );
                // 用最后一项覆盖要删除的项
                if pos != file_count - 1 {
                    root_inode.write_at(
                        pos * DIRENT_SZ,
                        dirents[file_count - 1].as_bytes(),
                        &self.block_device,
                    );
                }
                // 清空最后一项，防止幽灵目录项
                let zero_dirent = DirEntry::empty();
                root_inode.write_at(
                    (file_count - 1) * DIRENT_SZ,
                    zero_dirent.as_bytes(),
                    &self.block_device,
                );
                // 缩小 size
                let new_file_count = file_count - 1;
                trace!(
                    "[remove_hard_link] after, new_file_count = {}",
                    new_file_count
                );
                root_inode.size = (new_file_count as u32) * (DIRENT_SZ as u32);
                trace!(
                    "[remove_hard_link] after shrink, root_inode.size = {}",
                    root_inode.size
                );
            });
            block_cache_sync_all();
        } // 在这里释放锁！这是关键修改

        // 检查inode硬链接是否为0
        if inode_id.is_none() {
            trace!(
                "[remove_hard_link] ERROR: inode_id is None after dirent search, name = {:?}",
                name
            );
            return;
        }
        let inode_id = inode_id.unwrap();
        trace!(
            "[remove_hard_link] after dirent search, inode_id = {}",
            inode_id
        );

        // 现在在没有持有锁的情况下计算链接数
        let nlink = self.count_links(inode_id);
        trace!(
            "[remove_hard_link] after count_links, inode_id = {}, nlink = {}",
            inode_id,
            nlink
        );

        // 详细输出所有与该 inode_id 相关的目录项
        {
            let fs = self.fs.lock();
            self.read_disk_inode(|disk_inode| {
                let file_count = (disk_inode.size as usize) / DIRENT_SZ;
                for i in 0..file_count {
                    let mut dirent = DirEntry::empty();
                    let read_sz = disk_inode.read_at(
                        i * DIRENT_SZ,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    );
                    if dirent.inode_id() == inode_id {
                        trace!(
                            "[remove_hard_link][debug] dirent match: i = {}, name = {:?}, inode_id = {}, read_sz = {}",
                            i,
                            dirent.name(),
                            dirent.inode_id(),
                            read_sz
                        );
                    }
                }
            });
        }

        if nlink == 0 {
            // 创建一个新的作用域，在此作用域内处理inode清理
            {
                // 找到该 inode
                let mut fs = self.fs.lock();
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                drop(fs); // 立即释放锁

                // 创建一个新的inode实例，但不持有fs锁
                let inode = Inode::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                );
                trace!(
                    "[remove_hard_link] about to clear inode, inode_id = {}",
                    inode_id
                );
                // 调用clear
                inode.clear();
                trace!(
                    "[remove_hard_link] finished clear inode, inode_id = {}",
                    inode_id
                );
            }
            // 同步块缓存
            block_cache_sync_all();
        }

        trace!("[remove_hard_link] end of function for name = {:?}", name);
        // release efs lock automatically by compiler
    }

    /// return hard link of file by inode_id
    pub fn count_links(&self, inode_id: u32) -> usize {
        let mut count = 0;
        {
            // 锁定目录所在的文件系统，遍历当前目录中的所有 dirent
            let _fs = self.fs.lock();
            self.read_disk_inode(|disk_inode| {
                let file_count = (disk_inode.size as usize) / DIRENT_SZ;
                trace!(
                    "[count_links] inode_id = {}, file_count = {}",
                    inode_id,
                    file_count
                );
                for i in 0..file_count {
                    let mut dirent = DirEntry::empty();
                    let read_sz = disk_inode.read_at(
                        i * DIRENT_SZ,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    );
                    trace!("[count_links] i = {}, read_sz = {}", i, read_sz);
                    if read_sz != DIRENT_SZ {
                        trace!("[count_links][ERROR] read_sz != DIRENT_SZ, i = {}, read_sz = {}, expected = {}", i, read_sz, DIRENT_SZ);
                    }
                    let dir_inode_id = dirent.inode_id();
                    trace!("[count_links] dir_inode_id = {}", dir_inode_id);
                    if dir_inode_id == inode_id {
                        count += 1;
                    }
                }
            });
        }
        // 对于目录 inode，其内含“.”条目不会被计入硬链接因此需要减 1
        let fs = self.fs.lock();
        let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
        let is_dir = get_block_cache(block_id as usize, self.block_device.clone())
            .lock()
            .read(block_offset, |disk_inode: &DiskInode| disk_inode.is_dir());
        if is_dir && count > 0 {
            count - 1
        } else {
            count
        }
    }
}
