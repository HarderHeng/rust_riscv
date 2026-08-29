# sbi0：周期性 S 态时钟（`sbi_set_timer`）

日期：2026-08-29  
对照：rCore-Tutorial-v3 第 3 章（S 态时钟中断）  
板子：Sipeed M1s Dock / BL808 D0（C906）  
分支 / worktree：`sbi0-timer` @ `.worktrees/sbi-timer`

## 目标

在已跑通的 `sbi0` 上加上旧版 `sbi_set_timer`（`a7=0`），让 S 态周期性收到时钟中断，经 SBI 打出 5 行 tick，再 shutdown。

ACM0（`2000000 8N1`）必须按这个顺序出现，每行 `\r\n`：

```text
rust helloworld from D0/C906, uart up
rust helloworld from D0/C906, ipc synced
rust helloworld from D0/C906, icache on
[M] sbi0 entering S-mode
[S] hello via sbi
[S] tick 1
[S] tick 2
[S] tick 3
[S] tick 4
[S] tick 5
[M] shutdown
```

`[S] tick N` 只能由 S 态时钟中断处理函数经 `ecall` putchar 打出。看不到 5 行 tick 就算失败。

## 非目标

- 不上 RustSBI / OpenSBI，不用 `stimecmp`，不碰 PLIC。
- 不做 `sbi_console_getchar`、IPI、RFENCE。
- 不启页表，不改细粒度 PMP，不启 PSRAM。
- 不改 helloworld / stage0。
- shutdown 仍是打字后空转，不真断电。

## 结构

继续改 crate `sbi0/`，产物仍是 `sbi0.bin` @ Flash `0x100000`。不新建 crate。

## 时钟硬件

对照 C `CPU_Set_MTimer_CLK`（D0 / `board_init`）：

- 寄存器：`MM_MISC + 0x18` = `0x30000018`（`CPU_RTC`）。
- `[9:0]` = `C906_RTC_DIV`。DSP 320 MHz，目标 mtimer **1 MHz**，`div = 319`。
- `[30]` = `C906_RTC_RST`；`[31]` = `C906_RTC_EN`。
- 顺序：先清 EN，写 DIV，再置 EN。读当前时间用 `rdtime`（C906 没有 SiFive 式 `mtime` MMIO）。

`mtimecmp`（C SDK `CORET`，`PLIC_BASE + 0x4000000`）：

- `MTIMECMPL0` = `0xE4004000`
- `MTIMECMPH0` = `0xE4004004`
- 只按 32 位写：先写高半 `0xffffffff`，再写低半，再写高半，避免比较窗口误触发。

## M 态相对现 sbi0 的改动

1. I-cache 之后、`mret` 之前：开 mtimer 时钟（上面的 `0x30000018`）。
2. `install_trap`：`medeleg=0`，`mideleg = 1<<5`（只委托 S 态时钟）。不要关死 `mstatus.MIE` 直到进 S 态；`enter_s_mode` 必须置 `mstatus.MPIE`，使第一次 `mret` 后 `MIE=1`，S 态运行时机器时钟还能进 M 态。
3. `trap_m` **禁止**对所有陷阱 `mepc+=4`。只有 `mcause==9`（S 态 `ecall`）才加 4（可在 `trap_handle` 里做）。
4. 旧版 SBI 增加 `a7=0`：`sbi_set_timer`，`a0` = 下次 `time`。写 `mtimecmp`，清 `mip.STIP`（bit 5），开 `mie.MTIE`（bit 7），返回 `0`。
5. `mcause == (1<<63)|7`（机器时钟）：置 `mip.STIP`，关 `mie.MTIE`，不改 `mepc`，`mret` 回 S 态。
6. 其它中断 / 异常：仍打 `[M] trap` 后空转。未知 `a7`：`[M] bad sbi`。

## S 态

`s_main`：

1. 打 `[S] hello via sbi\r\n`（仍只走 putchar `ecall`）。
2. `stvec = trap_s`（direct）。
3. 开 `sie.STIE`。
4. `sbi_set_timer(rdtime() + 200000)`（200 ms @ 1 MHz）。
5. 开 `sstatus.SIE`。
6. `loop { wfi() }`。满 5 次 tick 后 `sbi_shutdown`。

`trap_s`：保存 GPR，S 态时钟（`scause == (1<<63)|5`）则打印 `[S] tick N`（N=1..5），再设下一次定时器；第 5 次之后 shutdown。其它 `scause`：可打 `[S] trap` 后空转。中断不改 `sepc`。

S 态仍禁止写 UART FIFO。

## 失败怎么看

| 现象 | 含义 |
|------|------|
| 只有 `[S] hello via sbi` 然后 shutdown / 沉默 | 没收到时钟：mtimer 时钟、`mtimecmp`、`MIE`/`MTIE` |
| `[M] trap` | 中断进了 M 态但 `mcause` 不是 7，或 `mepc+=4` 仍套在中断上 |
| tick 乱序 / 停在 1 | S 态没再 `set_timer`，或 `STIP` 没清 |
| tick 极快刷屏 | 1 MHz 假设错了，或 `mtimecmp` 一直落后 `time` |
| `[S] trap` | S 态收到的不是时钟中断 |

## 文档

上板通过后再改 `docs/chapter-status.md` 第 3 章。实现过程中改 `docs/architecture.md` / `AGENTS.md` / `README.md` 期望输出。
