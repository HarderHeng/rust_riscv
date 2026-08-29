# 内存图

地址来自 BL808 手册、本工作区 Linux `low_load`，以及 `bouffalo-rt` 的链接脚本。helloworld / stage0 还没有启用 PSRAM。sbi0 在 M 态 `init_psram` 之后：冒烟 `0x50001000`，trampoline `0x50002000`，根/L1 页表 `0x50004000` / `0x50005000` / `0x50006000`。开 `satp` 后的 **S 取指**走 Linux `PAGE_OFFSET=0xffffffe000000000`（2MB 叶盖住 `0x50000000`），不是低 VA 恒等。两核代码仍从 XIP 取指（M 态 trap 仍是物理 `0x58000000`）。

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

QEMU virt 的 `0x80000000` DRAM **不存在**。移植页帧池时改物理起止。C906 上 **S 态 I-fetch** 还要走高 VA（`0xffffffe000000000`），不能假设 QEMU 那种低地址恒等取指在实机上能过。

## sbi0 Sv39 窗口（板上已打出）

| VA | PA | 叶 | 用途 |
|----|----|----|------|
| `0xffffffe000000000` | `0x50000000` | 2MB `PAGE_KERNEL_EXEC` | 开 `satp` 后的 S 取指 |
| `0x50000000` | `0x50000000` | 2MB `PAGE_KERNEL`（无 X） | S 态 `lui` 探测 PSRAM |
| （物理，satp=0） | `0x50002000` | — | trampoline：`csrw satp` 本身 |
| （物理） | `0x50004000+` | 4KiB 表 | 根 / L1 |

`csrw satp` 后下一条物理 PC `0x50002004` 会 IPF；M 改 `mepc` 到 `0xffffffe000002004`。
