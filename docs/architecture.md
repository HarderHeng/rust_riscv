# 本仓库代码架构

当前落地的是 helloworld + stage0 + sbi0。SBI / rCore 还没有完整形态。

## 目录

```text
rcore-bl808/
  Cargo.toml              workspace；HAL 的 git+rev 写在这里
  helloworld-m0/          M0/E907，RV32，crate rust-helloworld-m0
  helloworld-d0/          D0/C906，RV64，crate rust-helloworld-d0
  stage0/                 D0/C906，RV64，crate stage0；S 态直接写 FIFO 对照
  sbi0/                   D0/C906，RV64，crate sbi0；M 态 SBI + S 态 ecall
  scripts/build.sh        编译、截断、blri patch、校验 hash
  scripts/check-stage0.sh 检查 stage0 的 BFNP / s_main / mret
  scripts/check-sbi0.sh   检查 sbi0 的 BFNP / s_main / mret / trap
  docs/architecture.md    本仓库代码怎么分层、怎么启动
  docs/memory-map.md      地址
  docs/chapter-status.md  对照 rCore-Tutorial 章节
```

四份固件都是 `no_std` + `bouffalo_rt::entry`。链接脚本来自 `bouffalo-rt`（`build.rs` 里 `-Tbouffalo-rt.ld`）。

依赖不引用仓库外路径：

```toml
bouffalo-hal = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96…" }
bouffalo-rt  = { git = "https://github.com/rustsbi/bouffalo-hal", rev = "ea477a96…", default-features = false }
xuantie-riscv = { git = "https://github.com/rustsbi/xuantie", rev = "fe7ec712" }
```

M0 feature：`bl808-mcu`。D0 feature：`bl808-dsp`。`blri` 由 `build.sh` 按同一 `rev` `cargo install` 到 `target/host-tools/`。

## Flash 与核

```text
Flash 0x000000  rust-helloworld-m0.bin   4KiB BFNP 头 + M0 payload
Flash 0x100000  sbi0.bin                 4KiB BFNP 头 + D0 payload（S 态路径，默认）
                stage0.bin               烧对照时换成这份（S 态直接写 FIFO）
                rust-helloworld-d0.bin   烧对照时换成这份（双核 M 态）

BootROM → M0 @ 自己的 XIP
M0 把 D0 入口写成 0x58000000，SF_CTRL group1 offset = 0x101000
D0 从 0x58000000 取指（对应 Flash 0x100000 后面那份 payload）
```

打好的 Flash 镜像默认用 `dd` 拷到 Windows（不要 `cp`）：

| 用途 | 目录 | 文件 |
|------|------|------|
| 默认 D0 | `C:\bl808_sbi0\` | `rust-helloworld-m0.bin` @ `0x0`，`sbi0.bin` @ `0x100000` |
| helloworld 对照 | `C:\bl808_helloworld\` | m0 + `rust-helloworld-d0.bin` |
| stage0 对照 | `C:\bl808_stage0\` | m0 + `stage0.bin` |

USB-UART（板上 BL702）：

| 口 | 典型设备 | 接到 |
|----|----------|------|
| 烧录 / ISP | `/dev/ttyACM1` | BootROM；应用起来后不要当控制台开 |
| D0 控制台 | `/dev/ttyACM0` | UART3，GPIO16 TX / GPIO17 RX，2 Mbps |

M0 的 UART0（GPIO14/15）在 WSL 上通常看不到。D0 打出 `ipc synced` 才能认定 M0 已跑过拉核。

## 谁做什么

`bouffalo-rt` `_start`：设栈、清 BSS、拷 `.data`、trap、PMP，然后 `main`。  
**不做** C SDK 的 `SystemInit` / `board_init`（cache、UART 时钟树、mtimer）。

因此板级代码自己补 HAL / `xuantie-riscv` / `riscv` 没有或不能用的部分：

| 缺口 | 写在 |
|------|------|
| UART0 → XCLK 40 MHz | `helloworld-m0` `enable_uart0_clock` |
| UART3 → XCLK 40 MHz | 两核都有 `enable_uart3_clock`（HAL 没有 DSP UART API） |
| UART3 波特率时钟 | D0 的 `Uart3Xclk`（HAL 写死 160 MHz） |
| 拉 D0 + IPC + dcache clean | `helloworld-m0` `start_d0_core` |
| 等 IPC + dcache invalidate | `helloworld-d0` `wait_for_m0` |
| I-cache | `sbi0` 用 `xuantie-riscv`（`icache_iall` + `mhcr::set_ie`）；helloworld / stage0 仍是手搓 |
| D0 mtimer 1 MHz | `sbi0` `0x30000018`（HAL 没有 `CPU_RTC`） |
| `mtimecmp` | `sbi0` 三拍 32 位写（不用 `THeadClint::write_mtimecmp`） |
| 5 秒延时 | `rdcycle`，按 320 MHz |

GPIO、UART 引脚复用、`freerun`、M 态 UART3 FIFO（`uart::RegisterBlock`）、CSR（`riscv`）用生态。

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

## stage0

`stage0/src/main.rs`。M 态前缀和 helloworld-d0 一样：UART3 XCLK 40 MHz、IPC、I-cache，并打出同样的 `uart up` / `ipc synced` / `icache on`。没有 5 秒 LED 循环。

然后：

1. 打印 `[M] stage0 entering S-mode`。
2. PMP 全开：`pmpaddr0=-1`，`pmpcfg0=0x1F`（NAPOT + R/W/X）。覆盖 `bouffalo-rt` 的栈保护 TOR，否则 S 态访问 UART3 / XIP 会被拒。
3. `satp=0`，`mstatus.MPP=S`，`mret` 到 `s_main`。
4. S 态只写 UART3 FIFO `0x30002000+0x88`，不调 HAL。`UART_FIFO_CONFIG_1[5:0]`（`+0x84`）是 TX **剩余空间**（空=32）；`== 0` 表示满，写之前要等。

## sbi0

`sbi0/src/main.rs`。M 态前缀与 stage0 相同：UART3 XCLK 40 MHz、IPC、I-cache、PMP 全开。能走 HAL / `xuantie-riscv` / `riscv` 的已经接上。

陷阱与委托：

- `mtvec = trap_m`（direct），`medeleg=0`，`mideleg` 只委托 S 态时钟。`mcounteren.TM=1`，否则 S 态 `rdtime` 非法指令。`enter_s_mode` 置 `MPIE`，进 S 态后 `MIE=1`。
- 开 D0 mtimer：`0x30000018`，`div=319`（DSP 320 MHz → 1 MHz）。`mtimecmp` 在 `0xE4004000/4`，按 32 位写。
- S 态 `ecall`：`a7=0` set_timer，`a7=1` putchar，`a7=8` shutdown。不写 UART FIFO。
- M 态 FIFO 用 HAL `uart::RegisterBlock`。`mcause` 用入口读到的原值比较（不要 `Mcause::from_bits`）。
- M 态 `trap_handle`：S 态 `ecall` 才 `mepc+=4` 再分发；机器时钟置 `STIP`、关 `MTIE`，不改 `mepc`。
- S 态 `stvec=trap_s`，5 次 200 ms tick 后 shutdown。2026-08-29 已在 ACM0 打出。
- 其它 M 态 `mcause` 打 `[M] trap`；其它 `a7` 打 `[M] bad sbi`。

## 镜像打包

`scripts/build.sh`：

1. `cargo build` 四个 crate（m0、d0、stage0、sbi0）。
2. `rust-objcopy -O binary`。
3. 按头里 `0x84` group offset、`0x8C` `img_len` 截断。objcopy 常多约 72 字节；`blri` 会 hash 到 EOF，BootROM 只 hash `img_len`。
4. `blri patch` 写 header SHA-256。
5. 再算一遍 payload SHA-256，必须等于头里 `0x90`。

`scripts/check-stage0.sh` 检查 `stage0.bin` 头是 BFNP，ELF 里有 `s_main` 和 `mret`。  
`scripts/check-sbi0.sh` 检查 `sbi0.bin` 头与 trap / ecall 符号。

## 和目标形态的关系

```text
现在：  BootROM → helloworld-m0 → sbi0（M 态 SBI，S 态 ecall）
对照：  BootROM → helloworld-m0 → stage0（S 态直接写 FIFO）
对照：  BootROM → helloworld-m0 → helloworld-d0（两核都在 M 态循环）
以后：  BootROM → M0 拉核 → D0 M 态（完整 SBI）→ S 态 rCore
```

下一层代码应继续让 M0 只拉核；C906 上的 OS 入口、页表、UART 驱动另开 crate，不要在 helloworld 里堆内核。

地址表见 [memory-map.md](memory-map.md)。和 Tutorial 各章的对照见 [chapter-status.md](chapter-status.md)。不要和 Linux `whole_img` 混烧。
