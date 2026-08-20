# 开发规范

## 发版

- **执行 `make publish` 之前，必须先把所有功能代码提交到 main**。`publish` 目标只会 `git add` 版本号文件（`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json`、`CHANGELOG.md`），不会提交功能代码；若功能代码未提交就打 tag，会导致 tag 源码与发布的 dmg 二进制不一致（历史上 v0.2.1/v0.2.2 曾出现此问题，v0.2.3 已修正）。
- 标准发版顺序：功能改动 → 提交功能代码（Conventional Commits）→ CHANGELOG 的 `[Unreleased]` 段落落上版本号和日期 → `make publish`（默认 patch，`VERSION_PART=minor/major` 可选）。
