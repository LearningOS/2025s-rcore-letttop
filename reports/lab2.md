为什么PNN要分为三段？

为什么编译器要求PageTable有一个default


# 实现功能
1. 扩展了TaskManager，用一个[(usize,usize);MAX_SYSCALL_NUM]来记录syscall的调用，提供add和count两个函数用来计数和调取计数
# 问答题
1. RustSBI-QEMU Version 0.2.0-alpha.3. 
 - bad_register: IllegalInstruction in application, kernel killed it.
 - bad_instruction: IllegalInstruction in application, kernel killed it.
 - bad_address: PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
 - 最后都是Panicked at src/task/mod.rs:139 All applications completed!
2. 
 - 刚进入时，sp代表栈顶，其上先是上下文，再是内核栈；初始从内核态进入用户态和trap后返回用户态。
 - CSR需要用寄存器的值写入；sstatus是之前的特权级；sepc是之前的pc；sscratch是之前的栈顶sp
 - x2是当前的sp还在上下文之下，需要调整后再写入；x4是目前不需要
 - __restore在内核态，sp是内核栈顶，交换后sp是用户栈顶
 - sret设置pc，设置特权级相关寄存器
 - 与问题4相反
 - trap-handler里的syscall触发ecall

# 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    [《The RISC-V Instruction Set Manual Volume I》](https://github.com/riscv/riscv-isa-manual)
    [RISC-V Supervisor Binary Interface Specification](https://github.com/riscv-non-isa/riscv-sbi-doc)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 建议意见