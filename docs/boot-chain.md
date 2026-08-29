# 启动链

## 现在（双核 helloworld）

1. BootROM 读 Flash `0x0` 的 BFNP 头（hash 必须等于 `img_len` 那段 payload），跳到 M0。
2. `helloworld-m0`：
   - UART0 改到 XCLK 40 MHz，GPIO14/15，2 Mbps
   - 开 I-cache
   - 把 D0 放进 TZC group 1
   - 写 D0 复位地址 `0x58000000`
   - 写 SF_CTRL group1 offset `0x101000`（D0 镜像在 Flash `0x100000`，跳过 4K 头）
   - 先在 `0x40000000/0x40000004` 写 IPC `0x12345678`，再 `dcache.cpa`（HAL 自带的 clean 会跳过这段地址）
   - 开 DSP UART3 时钟（XCLK）和 DSP CPU 时钟，清 D0 复位
   - 每 5 秒打一行并翻转 GPIO8（按 `rdcycle`、MCU 320 MHz）
3. D0 从 XIP `0x58000000` 进 `bouffalo-rt` 的 `_start`。
4. `helloworld-d0` 先开 UART3（GPIO16/17，2 Mbps），再等 IPC、开 I-cache，然后同样 5 秒循环。

这和 C SDK helloworld 的拉核、IPC、UART 时钟一致。D0 的 C `system_bl808.c` 也是等同一对 IPC flag，然后才开 cache。

## 下一步（S 态 / SBI）

1. M0 仍只拉核。
2. D0 在 IPC 之后进 S 态（或先挂上 OpenSBI / RustSBI）。
3. 页帧池放在 PSRAM `0x50000000`。
4. SBI 进 S 态后跳到 rCore。

Linux `low_load` 的参照地址见 [memory-map.md](memory-map.md)。不要把 Linux 的 `whole_img` 布局和 `0x0` / `0x100000` 两段裸镜像混烧。
