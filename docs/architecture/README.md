# 架构（Architecture）

面向架构师、核心开发者与深度重构时的 AI：回答「为什么这样设计、核心机制如何工作」。本目录存放对**当前项目**的全部解读。

## 文档

| 文档 | 内容 |
|---|---|
| [hal.md](hal.md) | HAL 分层设计：trait 体系、依赖方向、板级接入方式 |
| [shell.md](shell.md) | Shell 系统设计：I/O 抽象、命令注册表、状态机、扩展点 |
| [shell-reference.md](shell-reference.md) | Shell 使用参考：命令列表、行编辑、API、故障排查 |
| [shell-extension.md](shell-extension.md) | 新增 Shell 命令 / I/O 后端的实战指南 |
| [memory-map.md](memory-map.md) | 内存映射与 MMIO 地址（事实来源） |
| [interrupt-model.md](interrupt-model.md) | QEMU virt 中断模型详解：CLINT / PLIC / CSR / 回调注册 |
| [interrupt-comparison.md](interrupt-comparison.md) | QEMU 与 BL808 真实芯片中断机制对比 |
| [review-report.md](review-report.md) | 代码评审报告（2026-03-03，9.0/10） |
| [fixes-applied.md](fixes-applied.md) | 评审问题的修复记录 |

## 编写约定

- 架构文档解释**设计动机与机制**，同时记录硬性事实（内存地址、寄存器等）。
- 涉及「现状是什么」时标注对应代码位置，便于核对。
- 历史记录（评审、修复）只追加，不改写。
