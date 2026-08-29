# 内存图

地址来自 BL808 手册、本工作区 Linux `low_load`，以及 `bouffalo-rt` 的链接脚本。helloworld **还没有**启用 PSRAM；两核都在 XIP + 核内 SRAM 上跑。

## C906 能看到的窗口（Linux 参照）

| 区间 | 用途 |
|------|------|
| `0x20000000` | MCU 外设（部分） |
| `0x30000000` | MM 外设：UART3 `0x30002000`、MM_GLB `0x30007000` |
| `0x3eff0000` | Linux 把 OpenSBI 拷到这里（64K） |
| `0x3f000000` | VRAM（`bouffalo-rt` D0 栈在这附近） |
| `0x40000000` | IPC mailbox（`0x12345678` 握手） |
| `0x50000000` | **64MB PSRAM**，Linux 内核加载点；rCore 页帧池预备放这里 |
| `0x58000000` | Flash XIP。D0 镜像入口也是这个地址 |
| `0xe0000000` | PLIC |

Linux `low_load` 给 C906 配的 PMP 允许：MM 外设 1MB、OpenSBI 64K、PSRAM 64MB、XIP 64MB、PLIC。stage0 目前把 PMP 全开（`pmpaddr0=-1`，`pmpcfg0=0x1F`），避免 `bouffalo-rt` 栈保护 TOR 挡住 S 态 UART / XIP。细粒度 PMP 留给以后的 SBI。

## Flash 怎么摆（本仓库）

| Flash 偏移 | XIP | 内容 |
|------------|-----|------|
| `0x000000` | group0 | `rust-helloworld-m0.bin`（含 4K BFNP 头） |
| `0x100000` | group1，offset=`0x101000` | S 态路径：`sbi0.bin`（含 4K BFNP 头）。对照时换成 `stage0.bin` 或 `rust-helloworld-d0.bin` |

`0x58000000` 对应 Flash 里当前核的 image offset。M0 把 D0 的 offset 设成 `0x100000 + 0x1000`，所以 D0 的代码从自己那份头后面开始取指。

## 以后给 rCore 的建议布局

| 物理地址 | 用途 |
|----------|------|
| `0x50000000` | rCore 内核 + 页帧池（避开前 1MB 给 SBI/DTB 也可以） |
| `0x51ff8000` | DTB（若继续用 OpenSBI + FDT，Linux 用过这个地址） |
| `0x3eff0000` | SBI 固件（可沿用 Linux） |

QEMU virt 的 `0x80000000` DRAM **不存在**。移植 ch4 时改的是物理页帧起止，不是 Sv39 本身。
