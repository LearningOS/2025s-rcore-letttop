# 实现功能
1. 在os/sync/deadlock_detector.rs中创建检查器
2. process的inner中加入死锁检测器，fork时创立新进程的空白检测器
3. 在锁的syscall中调用检测器

# 问答题
1. 
   1. 主线程退出要回收所有线程的资源，包括代表任务的控制块TCB，TCB中所有引用的全局资源（锁、页、栈帧）
   2. 其他线程可能被引用的位置，即所有标注$Arc<TaskControlBlock>>$的位置，都需要回收
2. 
   1. mutex1的lock在loop中，会不断尝试获取，mutex2只尝试一次
   2. mutex1总是释放锁，mutex2在运行完队列内所有才释放   
3. 二者都没有考虑线程的调度，mutex1会有竞争，mutex2不考虑优先级


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
