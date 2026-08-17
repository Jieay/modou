# 墨斗（Modou）

> 轻量代码编辑器 + 原生终端工具

[![CI](https://github.com/Jieay/modou/actions/workflows/ci.yml/badge.svg)](https://github.com/Jieay/modou/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

墨斗是一个为资深程序员设计的 macOS 原生 IDE，采用 Tauri + Web 前端技术栈，兼顾美观与性能。

## 技术栈

- **前端**：HTML + CSS + JavaScript（VS Code Dark+ 风格）
- **后端**：Rust + Tauri 2.0
- **终端**：rustix-openpty + PTY
- **Git**：git2 crate
- **语法高亮**：highlight.js

## 功能特性

- **代码编辑**：多标签页、语法高亮、行号显示
- **项目浏览**：文件树、模糊搜索（⌘+P）
- **原生终端**：多终端会话、PTY 支持
- **Git 集成**：状态栏显示分支
- **命令面板**：⌘+Shift+P 快速执行命令
- **主题**：VS Code Dark+ 深色主题

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| ⌘+O | 打开项目 |
| ⌘+P | 搜索文件 |
| ⌘+Shift+P | 命令面板 |
| ⌘+S | 保存文件 |
| ⌘+` | 新建终端 |
| ⌘+J | 显示/隐藏终端 |
| ⌘+W | 关闭标签 |

## 下载安装

从 [Releases](https://github.com/Jieay/modou/releases) 下载最新的 `.dmg`（Apple Silicon），拖拽到「应用程序」即可。

> **注意**：应用未经过 Apple 开发者签名和公证，首次打开会提示"已损坏，无法打开"。这是 Gatekeeper 对未签名应用的拦截，并非应用真的损坏。在终端执行以下命令移除隔离属性后即可正常打开：
>
> ```bash
> xattr -cr /Applications/modou.app
> ```

## 构建与打包

```bash
# 安装依赖
cargo install tauri-cli

# 开发模式（热重载）
make dev

# 构建 release
make release

# 打包 .app 和 .dmg
make bundle

# 生成应用图标
make icons

# 安装到 /Applications
make install

# 一键发布新版本（递增版本号、构建、打 Tag、推送、创建 GitHub Release 并上传 .dmg）
make publish
# 发布 minor / major 版本
make publish VERSION_PART=minor
```

## 项目结构

```
modou/
├── Makefile                # 构建命令
├── README.md               # 项目说明
├── docs/                   # 文档
│   ├── design-prototype.html   # 设计原型
│   ├── design-analysis.md      # 设计分析
│   └── tech-solution-comparison.md  # 技术方案对比
├── src/                   # Web 前端
│   ├── index.html          # 主页面
│   ├── style.css           # 样式（VS Code Dark+）
│   └── app.js              # 前端逻辑
├── scripts/                # 工具脚本
│   └── generate_icon.py    # 图标生成器
└── src-tauri/              # Rust 后端
    ├── src/
    │   ├── main.rs         # 入口
    │   ├── lib.rs          # Tauri 应用
    │   ├── commands.rs     # IPC 命令
    │   ├── fs.rs           # 文件系统
    │   └── terminal.rs     # PTY 终端
    ├── icons/              # 应用图标
    └── tauri.conf.json     # Tauri 配置
```

## 文档

- [设计原型](docs/design-prototype.html) - 高保真界面设计
- [设计分析](docs/design-analysis.md) - 界面问题与设计方案
- [技术方案对比](docs/tech-solution-comparison.md) - Iced vs egui vs Tauri
- [功能开发文档](docs/功能开发文档-v2.md) - 完整功能规格
- [开发计划：三方终端停靠](docs/开发计划-三方终端停靠.md) - 面向 AI 助手的详细执行规格

## 贡献

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

[MIT](LICENSE) © 2026 zhufeng
