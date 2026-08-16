# 墨斗（Mòdǒu）轻量 IDE 项目方案

> 一个为资深程序员设计的简洁代码查看器 + 原生终端工具。

---

## 一、项目命名

### 推荐名：墨斗（Mòdǒu）

**寓意**：
- 墨斗是中国古代木匠最核心的生产力工具之一，用于弹出笔直的墨线，是"规矩"与"精准"的象征。
- 对程序员而言，IDE 正是现代意义上的"墨斗"——在代码森林里画线、定位、构造。
- 名字简短、有辨识度、有文化质感，且国际传播时可直接用拼音 `Modou`。

### 备选名称

| 名称 | 拼音 | 寓意 |
|------|------|------|
| 墨斗 | Mòdǒu | 木匠弹线工具，精准、 craftsman's tool |
| 矩尺 | Jǔchǐ | 鲁班尺/曲尺，代表规范与标准 |
| 规 | Guī | 圆规，代表设计、规划、范围 |
| 榫卯 | Sǔnmǎo | 榫卯结构，代表代码的精密咬合 |
| 耒耜 | Lěisì | 上古农具，代表生产力的起源 |
| 机杼 | Jīzhù | 织布机，代表构造与编排 |

**最终建议**：使用 **墨斗（Modou）** 作为主名称。

---

## 二、项目定位

### 核心目标

打造一个**启动快、内存小、终端原生、语法高亮美观**的轻量代码查看器 + 终端工作台。

### 阶段定位

- **第一版（本方案）**：**只读代码查看器** + **原生终端面板**。支持打开项目、浏览文件树、查看带语法高亮的源码、在底部面板运行 shell。
- **后续迭代**：再逐步加入文本编辑、保存、LSP 跳转、多标签等功能，向真正的轻量 IDE 演进。

> 明确第一版不做编辑，可以显著降低 MVP 复杂度，让团队聚焦在「渲染 + 终端」两个核心技术点上。

### 不做什么（边界）

- **第一版不做文本编辑**：不支持插入、删除、保存文件（仅只读浏览）。
- 不实现完整的 LSP 语言服务器协议（第一阶段）。
- 不内置 Git 图形化、调试器、插件市场等重型功能。
- 不追求替代 VS Code，而是作为"代码阅读 + 终端工作"的专用工具。

### 核心用户场景

1. 快速打开项目，浏览不同语言的源码。
2. 在应用内部开启一个与系统终端体验一致的 shell，运行 Kimi Code 等终端 AI 工具。
3. 长时间开着不占用过多内存，秒级启动。

### 目标用户画像

- **重度终端 AI 用户**：每天使用 Kimi Code、Claude Code、Aider 等终端 AI 工具，需要一边查看 AI 修改的代码，一边在终端继续对话。
- **多项目浏览者**：经常需要快速打开不同仓库阅读源码，但不想为每个项目启动一个重型 IDE。
- **性能敏感型开发者**：使用 16GB 以下内存的 MacBook，或同时运行 Docker、浏览器、多个 IDE，对内存和启动速度敏感。
- **Rust / 原生工具爱好者**：偏好原生应用，反感 Electron 的内存占用和启动延迟。

### 第一版验收标准（v0.1.0）

发布前必须满足：

- [ ] 能通过 `modou <path>` 或 ⌘+O 打开任意本地目录
- [ ] 文件树正确显示目录结构，支持展开/折叠与文件变更自动刷新
- [ ] 点击文件后能在 500ms 内渲染带语法高亮的内容（5MB 以内文件）
- [ ] 底部终端能启动默认 shell，并稳定运行 Kimi Code 等终端 AI 工具 30 分钟以上
- [ ] 支持深色/浅色主题切换，默认暗色主题可读性不低于 VS Code Dark+
- [ ] 冷启动时间 ≤ 500ms，空载内存 ≤ 120MB
- [ ] 能打包成 `.app` 并在未签名机器上通过右键打开运行

### 量化性能目标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 冷启动时间 | ≤ 500ms | 从点击图标到窗口可见 |
| 空载内存 | ≤ 120MB | 仅打开窗口、无项目 |
| 中等项目内存 | ≤ 250MB | 打开 10k 文件项目，未打开大文件 |
| 大文件打开 | ≤ 500ms | 打开 5MB 源码文件并渲染首屏 |
| 包体积 | ≤ 80MB | `.app` 压缩后分发包 |
| 启动终端 | ≤ 300ms | 从点击到 shell 提示符出现 |

### 竞品与差异化

| 工具 | 优势 | 不足 | 墨斗的切入点 |
|------|------|------|-------------|
| **VS Code / Cursor** | 生态完整、插件丰富 | 启动慢、内存高、基于 Electron | 不做全能 IDE，只做极速只读浏览 + 原生终端 |
| **Zed** | 原生、快、现代化 | 仍偏编辑器定位，终端不是核心 | 把终端体验做到与独立终端同等优先级 |
| **Helix / Neovim** | 极快、键盘流 | 学习曲线陡峭，GUI 体验弱 | 提供开箱即用的 GUI + 终端组合 |
| **Sublime Text** | 启动快、体验好 | 闭源、终端集成弱 | 开源、原生 Rust、终端为核心组件 |
| **Terminal.app / iTerm2** | 终端体验成熟 | 无项目浏览与语法高亮 | 在同一个窗口内整合文件树 + 高亮 + 终端 |

**核心差异化**：不是「又一个编辑器」，而是「**代码阅读器 + 终端工作台**」，专为需要频繁查看源码并运行终端 AI 工具的用户设计。

---

## 三、技术栈

| 模块 | 技术选型 | 说明 |
|------|---------|------|
| 主语言 | **Rust** | 零成本抽象、无 GC、启动快、二进制体积小 |
| GUI 框架 | **Iced**（推荐）或 **egui** | 纯 Rust 原生 GUI，无 WebView/Electron |
| 语法高亮 | **Tree-sitter** | AST 级解析，支持 100+ 语言，高亮精准 |
| 终端仿真 | **alacritty_terminal** crate | Alacritty 核心终端库，性能顶级 |
| 字体排版 | **cosmic-text** | 高质量文本布局，支持 HarfBuzz  shaping |
| 异步/PTY | **tokio** + **rustix/nix** | 异步 IO、伪终端管理 |
| 打包发布 | **cargo-bundle** | 生成 macOS `.app` 应用包 |

### 为什么不选 Electron/Tauri？

- Electron：打包 Chromium，内存占用 300MB+ 起步，违背"轻量"。
- Tauri：虽比 Electron 轻，但终端若用 xterm.js 仍是 Web 终端，无法做到真正的"原生终端感"。

---

## 四、系统架构

```
┌─────────────────────────────────────────────┐
│              墨斗（Modou）主窗口              │
│  ┌─────────────────┐  ┌───────────────────┐ │
│  │    侧边栏        │  │    代码编辑区      │ │
│  │   文件目录树      │  │  Iced 自定义渲染   │ │
│  │                 │  │  Tree-sitter 高亮  │ │
│  └─────────────────┘  └───────────────────┘ │
│  ┌─────────────────────────────────────────┐│
│  │              原生终端面板                ││
│  │      alacritty_terminal + PTY + Shell   ││
│  │            支持 Kimi Code 等工具        ││
│  └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

### 模块与目录映射

| 目录/模块 | 职责 |
|----------|------|
| `ui/` | 窗口管理、布局、主题、事件分发 |
| `editor/` | 只读文本渲染、滚动、Tree-sitter 高亮（第一版无编辑） |
| `project/` | 文件树、文件打开、路径管理、文件变更监听 |
| `terminal/` | PTY 创建、alacritty_terminal 集成、输入输出桥接与渲染 |
| `theme/` | 配色方案、字体配置、深色/浅色模式 |
| `config/` | 用户配置持久化（TOML）、最近打开项目、窗口状态 |
| `platform/` | 平台适配，第一版聚焦 `macos.rs` |

---

## 五、推荐目录结构

```
modou/
├── Cargo.toml                 # 包含 [package.metadata.bundle] 打包配置
├── README.md
├── assets/
│   ├── fonts/
│   │   └── JetBrainsMono-Regular.ttf
│   ├── icons/
│   │   └── modou.icns
│   └── themes/
│       └── default-dark.toml
├── src/
│   ├── main.rs
│   ├── app.rs                 # 应用主状态、消息分发
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── window.rs          # 窗口逻辑
│   │   ├── layout.rs          # 分栏布局
│   │   └── widgets/
│   │       ├── file_tree.rs
│   │       └── status_bar.rs
│   ├── editor/
│   │   ├── mod.rs
│   │   ├── document.rs        # 只读文档模型
│   │   ├── highlighter.rs     # Tree-sitter 高亮
│   │   ├── renderer.rs        # 文本渲染
│   │   └── scroll.rs          # 滚动与视口管理
│   ├── project/
│   │   ├── mod.rs
│   │   ├── tree.rs            # 文件树
│   │   └── watcher.rs         # 文件变更监听（notify）
│   ├── terminal/
│   │   ├── mod.rs
│   │   ├── pty.rs             # 伪终端封装
│   │   ├── bridge.rs          # 终端 ↔ GUI 桥接
│   │   └── renderer.rs        # 终端网格渲染
│   ├── config/
│   │   ├── mod.rs
│   │   ├── settings.rs        # 配置结构与默认值
│   │   └── paths.rs           # 配置目录路径
│   ├── theme/
│   │   ├── mod.rs
│   │   └── palette.rs         # 暗色/浅色配色
│   └── platform/
│       ├── mod.rs
│       └── macos.rs           # macOS 平台适配
└── crates/
    └── modou-core/            # 后续可拆分为独立 crate
```

---

## 六、风险与挑战

| 风险点 | 影响 | 应对策略 |
|--------|------|----------|
| **alacritty_terminal 集成复杂** | 高 | 该 crate 面向 Alacritty 自身设计，API 文档较少。预留 2 周做 POC，先验证能否在 Iced 中渲染终端网格。若阻塞，可降级为直接操作 PTY + 简易 VT 解析。 |
| **Iced 自定义渲染成熟度** | 中 | 代码编辑器和终端都需要在 Iced 中做精细绘制。先做最小可运行原型，验证 Canvas/Custom Widget 性能后再深入。 |
| **Tree-sitter 版本兼容** | 中 | `tree-sitter` 核心与各语言 parser 版本需严格匹配。采用固定版本，升级前跑 parser 冒烟测试。 |
| **大文件渲染性能** | 中 | 5MB 以上文件若全量加载高亮会卡顿。第一版采用「行区间懒加载 + 视口外截断」，不一次性构建完整 token 列表。 |
| **macOS 签名与 Gatekeeper** | 低 | 本地开发可绕过；对外分发需 Apple Developer 账号做签名 + 公证，需提前申请。 |

---

## 七、MVP 开发路线（约 6–8 周）

> 以下按「单兵开发、每周投入约 20–30 小时」估算。若人力或经验不同，可适当延长。

### 第 1 周：项目骨架与 Iced 窗口

- [ ] 创建 `modou` crate，配置 `cargo-bundle`
- [ ] 用 Iced 创建可运行的空白窗口
- [ ] 确定窗口最小尺寸、默认尺寸与 macOS 菜单栏
- [ ] 搭建日志（`tracing`）与 panic 处理

### 第 2 周：项目浏览与文件树 + 终端技术预研

- [ ] 实现左侧文件目录树（支持展开/折叠）
- [ ] 支持从命令行 `modou /path/to/project` 打开项目
- [ ] 支持 ⌘+O 打开项目目录对话框
- [ ] 文件变更监听（`notify`），目录变化时刷新树
- [ ] **并行任务**：调研 `alacritty_terminal` API，编写最小 PTY + 终端渲染 POC

### 第 3 周：只读文本显示

- [ ] 实现右侧文本显示区域（纯文本、等宽字体）
- [ ] 支持垂直/水平滚动、行号显示
- [ ] 大文件懒加载：仅加载视口附近行，避免内存爆炸
- [ ] 状态栏显示文件路径、行数、文件大小

### 第 4 周：语法高亮

- [ ] 集成 Tree-sitter 与核心语言 parser
- [ ] 实现基于 Tree-sitter 的 token 高亮管线
- [ ] 支持 5–8 种核心语言：Rust、Go、Python、TypeScript、JavaScript、C/C++、Markdown
- [ ] 设计默认暗色主题（参考 One Dark / Dark+）

### 第 5 周：原生终端面板实现

- [ ] 基于第 2 周 POC，通过 `rustix`/`nix` 创建 PTY
- [ ] 启动用户默认 shell（`$SHELL`）
- [ ] 集成 `alacritty_terminal` 解析 VT 序列
- [ ] 将终端内容渲染到 GUI 底部面板

### 第 6 周：终端交互完善

- [ ] 键盘输入、粘贴、窗口 resize 事件转发到 PTY
- [ ] 终端滚动、选区（第一版可只做基础选区）
- [ ] 多终端标签/会话管理（可选，视进度决定）
- [ ] 验证 Kimi Code 在终端中正常运行

### 第 7 周：主题、字体与 macOS 适配

- [ ] 深色/浅色主题切换
- [ ] 字体配置与 Retina 屏 DPI 缩放
- [ ] 快捷键统一（⌘+O 打开、⌘+T 新建终端、⌘+Q 退出等）
- [ ] 应用图标、菜单项、关于窗口

### 第 8 周：打包、性能优化与测试

- [ ] 使用 `cargo-bundle` 打包成 `.app`
- [ ] 大文件懒加载与增量解析优化
- [ ] 编写核心模块单元测试
- [ ] 整理 README、截图、Release Notes，发布 v0.1.0

---

## 八、关键依赖参考

```toml
[package]
name = "modou"
version = "0.1.0"
edition = "2021"

[dependencies]
# GUI
iced = { version = "0.13", features = ["canvas"] }

# 语法高亮
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-go = "0.23"
tree-sitter-cpp = "0.23"

# 终端仿真
alacritty_terminal = "0.24"

# PTY / 系统调用
rustix = { version = "0.38", features = ["pty", "process", "pipe"] }
nix = { version = "0.29", features = ["pty", "process"] }

# 异步运行时
tokio = { version = "1", features = ["rt-multi-thread", "io-std", "io-util", "process", "sync"] }

# 字体与文本排版
cosmic-text = "0.12"

# 配置与序列化
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# 文件监听
notify = "6.1"

# 日志
tracing = "0.1"
tracing-subscriber = "0.3"
```

> `cargo-bundle` 为独立的 Cargo 子命令，通过 `cargo install cargo-bundle` 安装，无需加入项目依赖。

> **版本验证提示**：以上版本号为方案撰写时的参考值。实际开发前请执行 `cargo search <crate>` 或在 [crates.io](https://crates.io) 确认最新稳定版，并验证 `tree-sitter` 核心与各语言 parser 的版本兼容性。`iced` 的 feature 名称也可能随版本变化，请以官方文档为准。

---

## 九、测试策略

### 单元测试

- **`project/tree.rs`**：验证目录扫描、过滤隐藏文件、路径排序。
- **`config/settings.rs`**：验证默认值、反序列化、缺失字段回退。
- **`theme/palette.rs`**：验证暗色/浅色主题颜色定义完整。

### 集成测试

- **文件打开流程**：给定一个临时目录，验证文件树能正确显示，点击后编辑器能加载文本。
- **Tree-sitter 高亮**：为每种核心语言准备一段示例代码，断言高亮 token 类型符合预期。
- **PTY 启动**：启动默认 shell，写入命令并读取输出，验证终端能正常交互。

### UI 与手动测试

- Rust 原生 GUI 的自动化 UI 测试生态较弱，第一版以手动测试为主。
- 关键路径 checklist：
  - [ ] 启动速度 ≤ 500ms
  - [ ] 打开 5MB 文件不卡死
  - [ ] 终端能运行 `kimi` 并正常交互
  - [ ] 深色/浅色切换无闪烁
  - [ ] 打包后的 `.app` 能在未签名机器上通过右键打开

---

## 十、配置持久化

### 配置目录

macOS 标准路径：

```
~/Library/Application Support/modou/
├── settings.toml
├── recent_projects.toml
└── window-state.toml
```

### 配置项示例

```toml
# settings.toml
[ui]
theme = "dark"           # "dark" | "light"
font_family = "JetBrains Mono"
font_size = 14
show_line_numbers = true

[editor]
tab_size = 4
word_wrap = false
max_file_size_mb = 20    # 超过此大小提示是否继续打开

[terminal]
shell = "/bin/zsh"       # 默认读取 $SHELL
font_size = 13
```

### 最近打开项目

```toml
# recent_projects.toml
[[projects]]
path = "/Users/xxx/work/my-project"
last_opened = "2026-07-27T14:18:28Z"
```

---

## 十一、macOS 注意事项

### 1. PTY 与 Shell

macOS 是类 Unix 系统，PTY 创建与 Linux 基本一致。默认 shell 通常为 `/bin/zsh`，建议读取 `$SHELL` 环境变量。

### 2. 渲染

- Iced 在 macOS 上默认使用 **Metal** 后端。
- 需处理 Retina 高分屏的 DPI 缩放。

### 3. 应用打包

使用 `cargo-bundle` 生成 `.app`：

```bash
cargo install cargo-bundle
cargo bundle --release
```

生成的应用在 `target/release/bundle/osx/墨斗.app`。

### 4. 签名与 Gatekeeper

- 自己使用时，首次打开若被拦截，可在 **系统设置 → 隐私与安全性** 中允许。
- 分发给他人时，建议进行 **代码签名 + 公证（Notarization）**。
- 需要一个 Apple Developer 账号（个人或组织）才能进行正式签名与公证。

### 5. 菜单与快捷键

- macOS 用户期望 ⌘+O 打开项目、⌘+Q 退出、⌘+W 关闭标签/窗口。
- 需要在应用菜单中提供 **关于、偏好设置、退出** 等标准入口。

### 6. 分发与更新策略

| 阶段 | 方式 | 说明 |
|------|------|------|
| 内部测试 | 手动分发 `.zip` | 无需签名，接收方在隐私设置中允许即可 |
| 公开预览 | GitHub Release + `.dmg`/`.zip` | 提供签名版，避免用户安装困难 |
| 后续迭代 | 可选自动更新 | 调研 `sparkle-rs` 或自行实现版本检查 + 下载替换 |

第一版建议只做 GitHub Release 手动下载，自动更新放到后续迭代。

### 7. 字体与版权

- 默认字体推荐使用 **JetBrains Mono**（Apache 2.0 License），可随应用一起分发。
- 需在应用内或 README 中保留字体版权声明与许可证文本。
- 允许用户在 `settings.toml` 中切换为系统已安装的其他等宽字体（如 SF Mono、Menlo）。

### 8. 终端环境变量

启动 PTY 时建议显式设置以下环境变量，确保终端行为与系统终端一致：

- `TERM=xterm-256color`：声明终端能力，使 CLI 工具正确输出颜色。
- `COLORTERM=truecolor`：支持 24-bit 真彩色。
- `LANG=zh_CN.UTF-8` 或 `en_US.UTF-8`：根据系统语言设置。
- `PATH`：继承用户默认 shell 的 PATH，确保 `kimi`、`git` 等命令可用。

---

## 十二、后续可扩展功能（非 MVP）

| 功能 | 优先级 | 说明 |
|------|--------|------|
| **文本编辑与保存** | 高 | 从只读查看器演进为真正的轻量 IDE |
| LSP 客户端 | 中 | 跳转定义、悬浮提示 |
| 多标签/多窗口 | 中 | 同时打开多个文件 |
| 模糊搜索 | 中 | 类似 VS Code 的 ⌘+P |
| Git blame/状态 | 低 | 行级 Git 信息 |
| 命令面板 | 低 | ⌘+Shift+P 触发 |
| 插件系统 | 低 | WASM 插件或 Lua 脚本 |

---

## 十三、快速开始

### 1. 初始化项目

```bash
# 创建 Rust 项目
cargo new modou --bin
cd modou

# 安装打包工具
cargo install cargo-bundle
```

### 2. 添加核心依赖

将「八、关键依赖参考」中的依赖复制到 `Cargo.toml`，然后：

```bash
cargo check
```

### 3. 编写最小可运行原型

先实现一个能打开目录并显示文件列表的 Iced 窗口，跑通后再逐步加入文本显示、高亮和终端。

### 4. 打包成 macOS 应用

在 `Cargo.toml` 中添加 cargo-bundle 所需的 metadata：

```toml
[package]
name = "modou"
version = "0.1.0"
edition = "2021"

[package.metadata.bundle]
name = "墨斗"
identifier = "com.example.modou"
icns = "assets/icons/modou.icns"
copyright = "Copyright (c) 2026"
category = "Developer Tool"
short_description = "墨斗：轻量代码查看器 + 原生终端"
long_description = """
墨斗（Modou）是一个为资深程序员设计的轻量代码查看器，
内置原生终端，专注于快速浏览源码和运行终端 AI 工具。
"""
```

执行打包：

```bash
cargo bundle --release
open target/release/bundle/osx/墨斗.app
```

> cargo-bundle 的具体字段可能随版本变化，请以 [cargo-bundle 文档](https://github.com/burtonageo/cargo-bundle) 为准。

---

## 十四、总结

墨斗（Modou）第一版采用 **Rust + Iced + Tree-sitter + alacritty_terminal** 技术栈，目标是在 macOS 上实现一个：

- **启动快**：原生编译，无 Electron 运行时负担。
- **只读浏览**：第一版聚焦代码查看，降低复杂度，快速验证核心体验。
- **高亮美**：基于 Tree-sitter 的真实 AST 高亮。
- **终端原生**：通过 PTY + alacritty_terminal 提供与系统终端一致的体验，完美支持 Kimi Code 等终端 AI 工具。
- **长期可维护**：Rust 的类型安全与现代包管理降低维护成本。

**下一步建议**：从 **Iced 窗口 + 文件树 + 单个文件文本显示** 开始，跑通第一个可执行原型，再逐步集成语法高亮与原生终端。
