# 贡献

先把问题或改动限制在**板级**。内核算法请尽量回馈 [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3)，不要在本仓库再养一份 fork。

## 当前欢迎的 PR

- `helloworld` 在更多 BL808 板上的复现记录
- OpenSBI 或 RustSBI 平台代码
- UART3 / PSRAM / 时钟的勘误
- 对照 Tutorial 章节的移植笔记

## 本地检查

```bash
./scripts/build.sh
```

两份 `.bin` 开头必须是 `BFNP`，且 header 在 `0x90` 处不是 `deadbeef` 占位。

## 提交说明

用现在时、说原因，例如：`fix PMP so S-mode can reach UART3`。
