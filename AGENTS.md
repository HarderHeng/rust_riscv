# Agent 规则

给在本仓库里改代码的 AI / 人类。细节以 [docs/architecture.md](docs/architecture.md) 为准。

## 这是什么

板级仓库：把 rCore-Tutorial 接到 Sipeed M1s Dock（BL808）的 **D0/C906**。  
**不要 fork 内核，不要把 rCore 拷进本仓库。** 现在也不要加 rCore submodule。

E907 没有 MMU，不能跑 rCore。M0 只负责拉起 D0。

## 依赖

- `bouffalo-hal` / `bouffalo-rt` / `blri` 只用 `Cargo.toml` 里的 **git + rev**，或 `scripts/build.sh` 里同一组 `HAL_GIT` / `HAL_REV`。
- **禁止** `path = "../bouffalo-hal"` 这类仓库外本地路径。
- 不要改旁边 checkout 的 `bouffalo-hal` 来“顺手修 HAL”。板级缺口写在本仓库。
- 升级 HAL 时同时改 workspace `rev` 和 `build.sh` 的 `HAL_REV`。

## 硬件与烧录

- 板子：M1s Dock。下载模式：BOOT+RST，先松 RST，再松 BOOT。
- M0 镜像 @ Flash `0x0`，D0 @ `0x100000`。不要和 Linux `whole_img` 混烧。
- **不要随便 `--whole_chip`。**
- 烧录口：编号最大的 `/dev/ttyACM*`（常见 `ttyACM1`）。看 D0 日志用 `ttyACM0`，`2000000 8N1`。
- **不要把 ACM1 当控制台。** 打开它会拨 DTR，干扰运行中的核。
- 不要用 Ox64 / 其它板的地址套到 M1s 上。

## 镜像

`rust-objcopy` 会在 `img_len` 后面多垫字节。必须先截到 `group_image_offset + img_len`，再 `blri patch`，再核 SHA-256。`scripts/build.sh` 已经做了，不要绕过。

BootROM 认 `flag` 里的 hash。header 对不上就整片沉默，不像“程序跑了但波特率错”。

## 写固件时

- HAL `_start` **不会**开 I-cache、不会做 C `board_init`。XIP 上空转延时必须先开 I-cache，或用 `rdcycle` / mtimer，不要假设 `riscv::asm::delay` 是墙钟。
- UART：C 用 XCLK 40 MHz。HAL 默认 UART0=80 MHz bclk，UART3 写死 160 MHz。波特率按真实时钟算。
- 拉 D0：对照 C `start_d0_core`。IPC 写在 `0x40000000/0x40000004`，**先写后 `dcache.cpa`，再放复位**。不要调用 HAL 的 `l1c_dcache_clean_range`（它会跳过这段地址）。
- D0 等 IPC 时 `dcache.ipa`。开 I-cache 放在 IPC 之后，和 C `SystemInit` 一样。
- 时钟：镜像头 MCU/DSP 都是 WiFi PLL **320 MHz**。我们没有跑 C 的 `GLB_Set_DSP_System_CLK(400M)`。
- 板级 MMIO 用常量 + `read_volatile`/`write_volatile`，并在注释里写清对照的 C 函数。
- UART 行尾必须是 `\r\n`。不要用只加 `\n` 的 `writeln!`，终端否则不会回列首。
- **sbi0（默认 D0 路径）**：S 态输出必须走旧版 SBI `ecall`（`a7=0` set_timer，`a7=1` putchar，`a7=8` shutdown）；禁止 S 态写 `0x30002000+0x88`。只有 M 态陷阱路径写 UART3 FIFO。`mepc+=4` 只用于 `ecall`，中断不要加。D0 mtimer：`0x30000018` div=319；`mtimecmp` `0xE4004000`。
- **stage0 对照**：仍由 S 态直接写 UART3 FIFO，不要为 sbi0 改 stage0。
- 这一期 PMP 全开（`pmpaddr0=-1`，`pmpcfg0=0x1F`）。不要在没验证 `[S]` 之前改回细粒度。
- 不要改 helloworld 或 stage0 来“顺便”做 sbi0。

## 文档与提交

- 架构事实写进 `docs/architecture.md`，不要只改口头约定。
- 提交用现在时、说原因。不要提交 `target/`、密钥、整片 Flash 转储。
- 用户没要求就不要 `git push`，不要改 git config。
