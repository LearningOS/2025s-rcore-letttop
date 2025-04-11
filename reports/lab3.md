# 实现功能
1. 迁移上一章的sys_get_time sys_mmap sys_munmap，主要是memory_set的获取
2. sys_spawn，创建进程，设置父子关系，加入队列
3. stride 调度算法，pass = BigStride / priority，TCB的mutable部分加入stride和prio

# 问答题
1. Stride
 - 否，8bit 最大2^8-1=255，250+10=4 < 255
 - when prio >= 2, max pass = BigStride / 2; MAX(STRIDE_MAX – STRIDE_MIN) = 存储类型的上下界；如果存储类型的上下界 > BigStride / 2, 那么一开始0加上BigStride / 2，再加就会越界，实际的MAX(STRIDE_MAX – STRIDE_MIN)还是等于BigStride / 2
  ```rust
  use core::cmp::Ordering;
  struct Stride(u64);
  impl PartialOrd for Stride {
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if other.0.wrapping_sub(self.0) < 1 << 63 {
        Some(Ordering::Less)
        } else {
        Some(Ordering::Greater)
        }
      }
  }
  impl PartialEq for Stride {
      fn eq(&self, other: &Self) -> bool {
          false
      }
  }
  ```

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
