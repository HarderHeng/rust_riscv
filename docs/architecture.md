# 架构

```text
                Flash
  0x000000  rust-helloworld-m0.bin   M0/E907  打印、拉起 D0、5 秒闪灯
  0x100000  rust-helloworld-d0.bin   D0/C906  等 IPC、打印、5 秒闪灯

  USB-UART (BL702)
       │
       ├─ UART0 GPIO14/15  ← M0 日志（烧录口 ACM1 上通常读不到）
       └─ UART3 GPIO16/17  ← C906 控制台（ACM0，rCore 也用这个）
```

## 原则

1. **OS 只跑在 C906。** E907 没有 Sv39，不当 rCore 宿主。
2. **不 fork 内核。** 本仓库是 board support：启动、SBI、UART、内存、文档。
3. **先双核能说话，再 S 态，再 SBI，再 Sv39。** 跳步会把 T-Head 的 cache/MMU 坑和板级坑混在一起。
4. **M 态固件库不是内核。** `bouffalo-hal` 只用来把核带进一个能说话的环境。HAL 的 `_start` 不会开 I-cache、也不会配齐 C SDK 的 `board_init`，这些要在板级补。

## 分层（目标形态）

```text
用户程序                         rCore-Tutorial user
────────────────────────────────────────────────────
S 态内核                         rCore-Tutorial os（尽量不改）
────────────────────────────────────────────────────
SBI                              OpenSBI 或 RustSBI（本仓库平台代码）
────────────────────────────────────────────────────
M 态加载                         M0 拉核 + D0 早期初始化
────────────────────────────────────────────────────
BootROM + BFNP 头                博流启动头（截断到 img_len + blri patch）
```

现在只做到「双核 M 态 helloworld」。S 态、SBI 和 rCore 内核还没挂上。

## 为什么第一步是 helloworld

`bouffalo-rt` 进 `main` 之前不会做 C SDK 的 `SystemInit` / `board_init`。镜像 hash、UART 时钟、D-cache clean 地址、I-cache、CPU 频率，都要在板上对过一遍，后面进 S 态才有意义。
