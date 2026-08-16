# 墨斗（Modou）

> 轻量代码编辑器 + 原生终端工具

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
| ⌘+T | 新建终端 |
| ⌘+W | 关闭标签 |

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

## 许可证

MIT License
