 fatal: repository 'https://rsproxy.cn/crates.io-index/' not found
error: failed to update replaced source registry `crates-io`

Caused by:
  failed to fetch `https://rsproxy.cn/crates.io-index`

Caused by:
  process didn't exit successfully: `git fetch --force --update-head-ok 'https://rsproxy.cn/crates.io-index' '+HEAD:refs/remotes/origin/HEAD'` (exit status: 128)
make: *** [Makefile:41: env] Error 101
Error: Process completed with exit code 2.
再次commit尝试

# 实现功能
1. 扩展了sys_trace: new: 物理地址直接交互
2. 重写sys_get_time 
3. sys_mmap，使用memoryset的insert_framed_area方法创建maparea
4. sys_munmap，从memoryset的页表中删除map_area
# 问答题
1. PTE
 - 63-54: 未使用
 - 53-10: 44位PPN，用于寻找下级页表或物理地址
 - 9-8  : 未使用
 - 7 - D: 自从页表项上的这一位被清零之后，页表项的对应虚拟页表是否被修改过。
 - 6 - A: 自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被访问过；
 - 5 - G: 全局，切进程不刷新
 - 4 - U: 用户可见
 - 3 - X: 可执行
 - 2 - W: 可写
 - 1 - R: 可读
 - 0 - V: valid，有效
2. 缺页
 - 缺页就是VA对应的PTE不符合要求，比如对V=0的操作，读R=0的页，写W=0的页，执行X=0，U-mode访问U=0
 - 重要的寄存器就是异常处理对应的寄存器，sstatus，sepc，scause，stval，stvec
 - lazy可以减少物理页占用，减少不必要的操作
 - 10GB/4KB=2.5M个页帧，三级页表需要2.5M PTE * 8Byte = 20MB，二级页表需要20MB/512=39KB，一级页表需要39KB/512=77B
 - 先分配虚拟地址，使用时再创建PTE映射物理地址；缺页时把一个有效的PTE映射写入硬盘，再map新的申请
 - V=0，触发中断，OS处理过程中查找是否在swap中
3. 单双页表
 - 单页表在切换进程时切换整个页表
 - 通过U标记
 - 异常处理是无须切换页表，缓存命中率更高
 - 双页表切换：切换进程（跟单页表一样）、切换特权级（单页表不用）

# 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    [《The RISC-V Instruction Set Manual Volume I》](https://github.com/riscv/riscv-isa-manual)
    [RISC-V Supervisor Binary Interface Specification](https://github.com/riscv-non-isa/riscv-sbi-doc)
    [rCore-Tutorial-Book-v3 3.6.0-alpha.1](https://rcore-os.cn/rCore-Tutorial-Book-v3)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 建议意见
 - guide文档似乎是从book里抽取的，有些东西讲的不够清楚，有些代码与文档不对应