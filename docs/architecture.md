# 本仓库代码架构

当前落地的只有双核 M 态 helloworld。rCore / SBI / S 态还没有代码。

## 目录

```text
rcore-bl808/
  Cargo.toml              workspace；HAL 的 git+rev 写在这里
  helloworld-m0/          M0/E907，RV32，crate rust-helloworld-m0
  helloworld-d0/          D0/C906，RV64，crate rust-helloworld-d0
  scripts/build.sh        编译、截断、blri patch、校验 hash
  docs/architecture.md    本仓库代码怎么分层、怎么启动
  docs/memory-map.md      地址
  docs/chapter-status.md  对照 rCore-Tutorial 章节
```

两份固件都是 `no_std` + `bouffalo_rt::entry`。链接脚本来自 `bouffalo-rt`（`build.rs` 里 `-Tbouffalo-rt.ld`）。

依赖不引用仓库外路径：

```toml
bouffalo-hal = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96…" }
bouffalo-rt  = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96…", default-features = false }
```

M0 feature：`bl808-mcu`。D0 feature：`bl808-dsp`。`blri` 由 `build.sh` 按同一 `rev` `cargo install` 到 `target/host-tools/`。

## Flash 与核

```text
Flash 0x000000  rust-helloworld-m0.bin   4KiB BFNP 头 + M0 payload
Flash 0x100000  rust-helloworld-d0.bin   4KiB BFNP 头 + D0 payload

BootROM → M0 @ 自己的 XIP
M0 把 D0 入口写成 0x58000000，SF_CTRL group1 offset = 0x101000
D0 从 0x58000000 取指（对应 Flash 0x100000 后面那份 payload）
```

USB-UART（板上 BL702）：

| 口 | 典型设备 | 接到 |
|----|----------|------|
| 烧录 / ISP | `/dev/ttyACM1` | BootROM；应用起来后不要当控制台开 |
| D0 控制台 | `/dev/ttyACM0` | UART3，GPIO16 TX / GPIO17 RX，2 Mbps |

M0 的 UART0（GPIO14/15）在 WSL 上通常看不到。D0 打出 `ipc synced` 才能认定 M0 已跑过拉核。

## 谁做什么

`bouffalo-rt` `_start`：设栈、清 BSS、拷 `.data`、trap、PMP，然后 `main`。  
**不做** C SDK 的 `SystemInit` / `board_init`（cache、UART 时钟树、mtimer）。

因此板级代码自己补：

| 缺口 | 写在 |
|------|------|
| UART0 → XCLK 40 MHz | `helloworld-m0` `enable_uart0_clock` |
| UART3 → XCLK 40 MHz | 两核都有 `enable_uart3_clock`（HAL 没有 DSP UART API） |
| UART3 波特率时钟 | D0 的 `Uart3Xclk`（HAL 写死 160 MHz） |
| 拉 D0 + IPC + dcache clean | `helloworld-m0` `start_d0_core` |
| 等 IPC + dcache invalidate | `helloworld-d0` `wait_for_m0` |
| I-cache | 两核 `enable_icache`（MHCR `0x7C1`，`icache.iall`） |
| 5 秒延时 | `rdcycle`，按 320 MHz |

GPIO、UART 引脚复用、`freerun` 用 HAL。

## M0 启动顺序

`helloworld-m0/src/main.rs`：

1. `enable_uart0_clock`：打开 UART0 门控，HBN 选 XCLK，div=0，并改 `Clocks`，这样 `freerun` 按 40 MHz 算 2 Mbps。
2. GPIO8 拉高（UART 错了也能看灯）。
3. UART0：mux sig2/sig3 → GPIO14/15。
4. `enable_icache`。
5. `enable_uart3_clock`：`MM_CLK_CTRL_CPU` / `MM_CLK_CTRL_PERI`，对照 `GLB_Set_DSP_UART0_CLK(XCLK, 0)`。
6. `start_d0_core`（见下）。
7. 循环：翻 GPIO8、打印、`delay_ms(5000)`。`rdcycle` 按 `MCU_HZ = 320e6`（头里 `mcu_clk = 0x04`）。

### `start_d0_core`

对照 C `start_d0_core`，顺序不能乱：

1. TZC MM BMX：D0 进 group 1 并 lock。
2. `MM_MISC_CPU0_BOOT = 0x58000000`。
3. `SF_CTRL` ID1 offset = `0x100000 + 0x1000`。
4. 在 `0x40000000`、`0x40000004` 写 `0x12345678`。
5. `dcache.cpa`（`.insn … 0x29`）。HAL `l1c_dcache_clean_range` 只认 `0x2000_0000` / `0x6000_0000`，对 IPC 是空操作。
6. 开 MMCPU0 时钟，转大约 1 µs，清 `MMCPU0_RESET`。

先写 IPC 再放复位。D-cache 若脏，D0 会一直读到 0。

## D0 启动顺序

`helloworld-d0/src/main.rs`：

1. 再配一遍 DSP UART 时钟（防止只靠 M0）。
2. GPIO16/17 `into_mm_uart`，`uart3.freerun(..., Uart3Xclk)`，时钟按 40 MHz。
3. 先打印 `uart up`（此时还没等 IPC，用来区分“核活了”和“卡在握手”）。
4. `wait_for_m0`：读两个 IPC 字，循环里 `dcache.ipa`；到手后清 0。
5. `enable_icache`（对照 C D0 `SystemInit`：等 IPC 之后才开 cache）。
6. 循环：翻 GPIO8、打印、`delay_ms(5000)`。`DSP_HZ = 320e6`（头里 `dsp_clk = 0x03`）。没有调用 C 的 `GLB_Set_DSP_System_CLK(400M)`。

两核都在拧 GPIO8，和 C helloworld 一样，灯周期可能看起来不整齐。

## 镜像打包

`scripts/build.sh`：

1. `cargo build` 两个 crate。
2. `rust-objcopy -O binary`。
3. 按头里 `0x84` group offset、`0x8C` `img_len` 截断。objcopy 常多约 72 字节；`blri` 会 hash 到 EOF，BootROM 只 hash `img_len`。
4. `blri patch` 写 header SHA-256。
5. 再算一遍 payload SHA-256，必须等于头里 `0x90`。

## 和目标形态的关系

```text
现在：  BootROM → helloworld-m0 → helloworld-d0（两核都在 M 态）
以后：  BootROM → M0 拉核 → D0 M 态（SBI）→ S 态 rCore
```

下一层代码应继续让 M0 只拉核；C906 上的 OS 入口、页表、UART 驱动另开 crate，不要在 helloworld 里堆内核。

地址表见 [memory-map.md](memory-map.md)。和 Tutorial 各章的对照见 [chapter-status.md](chapter-status.md)。不要和 Linux `whole_img` 混烧。
