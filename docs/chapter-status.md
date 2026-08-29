# 对照 rCore-Tutorial-v3

上游：[rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3) 和 [书](https://rcore-os.cn/rCore-Tutorial-Book-v3/)。

| 章 | 上游内容 | 本仓库 |
|----|----------|--------|
| 0 环境 | QEMU + RustSBI-QEMU | **双核 HAL helloworld 已在 M1s 上跑通**；QEMU 仍建议用来对照算法 |
| 1 裸机 | S 态打印 | **stage0 已在 M1s 上 `mret` 进 S 态，ACM0 打出 `[S] hello from supervisor on C906`** |
| 2 批处理 | SBI 控制台 / 关机 | **sbi0 已在 M1s 上经 `ecall` 打出 `[S] hello via sbi` 和 `[M] shutdown`** |
| 3 时钟中断 | `sbi_set_timer` | 缺 SBI 时钟 |
| 4 Sv39 | 页表、恒等映射 | 核支持；物理区间要改成 PSRAM |
| 5 进程 | 地址空间 | 未开始 |
| 6 文件系统 | easy-fs | 未开始；块设备可能是 XIP 或 SD |
| 7 进程间通信 / 更后 | — | 未开始 |

原则：QEMU 上先把某一章跑通，再把同一章的 board 假设（UART、时钟、内存）换成 BL808。不要在实机上同时改内核算法和板级地址。
