# 文档中心

本目录是项目的文档库。`architecture/` 存放对**当前项目**的全部解读；`spec/` 与 `plan/` 是**面向未来工作流**的目录，默认应为空（引入新功能时：先写 spec 提案 → 再写 plan 计划 → 实现完成后沉淀进 `architecture/`）。

```text
docs/
├── README.md              # 本索引
├── architecture/          # 对当前项目的解读（架构师 / 核心开发者 / 深度参与者）
├── spec/                  # （空）新功能提案，先于实现编写
└── plan/                  # （空）针对已通过 spec 的实现计划
```

## 目录职责

| 目录 | 状态 | 内容 |
|---|---|---|
| [architecture/](architecture/) | 当前解读 | 设计原理、机制细节、内存映射、历史评审与修复记录 |
| spec/ | 默认空 | 新功能规格提案（引入功能的第一步） |
| plan/ | 默认空 | 新功能的实现计划（spec 通过后编写） |

## architecture/ 文档清单

| 文档 | 内容 |
|---|---|
| [hal.md](architecture/hal.md) | HAL 分层设计：trait 体系、依赖方向、板级接入 |
| [shell.md](architecture/shell.md) | Shell 系统解读：I/O 抽象、命令系统、状态机、扩展点 |
| [shell-reference.md](architecture/shell-reference.md) | Shell 使用参考：命令、行编辑、API、故障排查 |
| [shell-extension.md](architecture/shell-extension.md) | 新增 Shell 命令 / I/O 后端的实战指南 |
| [memory-map.md](architecture/memory-map.md) | 内存映射与 MMIO 地址（事实来源） |
| [interrupt-model.md](architecture/interrupt-model.md) | QEMU virt 中断模型：CLINT / PLIC / CSR / 回调注册 |
| [interrupt-comparison.md](architecture/interrupt-comparison.md) | QEMU 与 BL808 中断机制对比 |
| [review-report.md](architecture/review-report.md) | 代码评审报告（2026-03-03，9.0/10） |
| [fixes-applied.md](architecture/fixes-applied.md) | 评审问题的修复记录 |

## 工作流约定

1. 引入新功能：先在 `spec/` 写规格提案 → 通过后在 `plan/` 写实现计划 → 实现完成后，将结论沉淀进 `architecture/` 并清理 spec/plan 中的过程文档。
2. 修改代码时如有对应解读文档，必须同步更新（如内存布局改动 → `architecture/memory-map.md`）。
3. 历史记录（评审、修复）只追加，不改写。
