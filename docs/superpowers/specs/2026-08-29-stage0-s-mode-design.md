# stage0：D0 S 态打印

日期：2026-08-29  
对照：rCore-Tutorial-v3 第 1 章（裸机 S 态打印）  
板子：Sipeed M1s Dock / BL808 D0（C906）

## 目标

证明 C906 能离开 M 态、在 S 态用已经初始化好的 UART3 说话。这是接 SBI / rCore 的前置，本身不是内核。

ACM0（`2000000 8N1`）上必须按这个顺序出现：

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
[M] stage0 entering S-mode
[S] hello from supervisor on C906
```

`[S]` 那一行只能由 `mret` 之后的函数写出。看不到 `[S]` 就算失败（卡在 PMP、没真正 mret、或 FIFO 地址错）。

## 非目标

- 不加 rCore-Tutorial submodule，不改内核。
- 不上 OpenSBI / RustSBI。
- 不启页表（`satp` 保持 0）。
- 不配 S 态中断、不启 PSRAM。
- 不改 `helloworld-m0` / `helloworld-d0` 的行为；它们继续当对照。

## 结构

| 产物 | 角色 | Flash |
|------|------|--------|
| `helloworld-m0.bin` | 已验证的 M0 拉核（不改） | `0x0` |
| `stage0.bin` | 新 crate：M 态初始化 + `mret` + S 态打印 | `0x100000` |
| `helloworld-d0.bin` | 对照，仍编译，默认不烧这条路径 | `0x100000`（仅对照时） |

新 crate：`stage0/`，`bl808-dsp`，依赖与 `helloworld-d0` 相同（workspace git HAL）。  
`scripts/build.sh` 额外产出 `target/riscv64imac-unknown-none-elf/release/stage0.bin`，并打印烧 `helloworld-m0` + `stage0` 的命令。

## D0 行为

M 态前半与 `helloworld-d0` 相同，不要发明第二套拉核/时钟：

1. `enable_uart3_clock`（XCLK 40 MHz）。
2. GPIO16/17 + `uart3.freerun(..., Uart3Xclk)`，2 Mbps。
3. 打印 `uart up`。
4. `wait_for_m0`（IPC `0x40000000/4` = `0x12345678`，循环里 `dcache.ipa`）。
5. `enable_icache`。
6. 打印 `ipc synced`、`icache on`。
7. 打印 `[M] stage0 entering S-mode` 并 `flush`。
8. 放开 PMP（见下）。
9. `satp = 0`。
10. `mstatus.MPP = S`（bits `[12:11] = 01`），`mepc = s_main`，`mret`。

`s_main`（S 态，`-> !`）：

- 复用当前 `sp`（M 态 `bouffalo-rt` 栈），不换栈。
- 不调用 HAL。
- 按字节写 UART3 TX FIFO：`0x3000_2000 + 0x88`（`UART_FIFO_WDATA`）。
- 字符串：`"[S] hello from supervisor on C906\r\n"`。
- 写完后空转（`loop { spin_loop() }`）。不要 5 秒翻灯，避免和“是否还在 M 态循环”搞混。

## PMP

`bouffalo-rt` `_start` 已经写了 `pmpcfg0` / `pmpaddr0..1`，把 `0x3F000000` 一段禁掉给 S/U。C906 上只要配过 PMP，未命中区间对 S 态通常是拒绝，UART3 和 XIP 都会炸。

这一期：**整段地址空间 NAPOT + R/W/X**，覆盖 `pmpcfg0` 最低一档即可（`A=NAPOT`，`XWR=111`）。细粒度 PMP 留给以后的 SBI。

## 失败怎么看

| 现象 | 含义 |
|------|------|
| 完全没字 | 镜像 hash / 没进下载模式 / 烧错地址（先烧对照 helloworld） |
| 只有 `uart up` | IPC，M0 没起来或 cache 没 clean |
| 有 `icache on` 和 `[M]` 没有 `[S]` | `mret` 或 PMP 或 FIFO |
| `[S]` 乱码 | 时钟/波特率；M 态那几行若正常则更像 S 态写错寄存器 |

## 文档

做完后改：

- `docs/architecture.md`：加上 `stage0` 流程。
- `docs/chapter-status.md`：第 1 章改为“stage0 已在板上打出 `[S]`”。
- `docs/memory-map.md`：Flash 表增加 stage0 一行。
- `AGENTS.md`：S 态禁止用 HAL UART；PMP 这一期是全开。
- `README.md`：烧 stage0 的命令和期望输出。
