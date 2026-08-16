# 墨斗（Modou）v0.3.0 功能开发文档（Tauri 方案）

> 版本：v0.3.0
> 技术方案：Tauri + Web 前端
> 平台：macOS
> 前端：HTML + CSS + JavaScript
> 后端：Rust + Tauri 2.0

---

## 一、方案背景

### 1.1 为什么从 Iced 切换到 Tauri

| 对比维度 | Iced（方案 A） | Tauri（方案 C） |
|---------|---------------|----------------|
| 美观上限 | 60%（需大量自定义） | 100%（HTML/CSS 无限制） |
| 开发效率 | 低（需自定义 Widget） | 高（前端生态成熟） |
| 阴影/渐变/动画 | 需 canvas 手动绘制 | CSS 直接支持 |
| 图标系统 | Emoji 业余 | SVG 专业 |
| 字体渲染 | 一般 | 优秀（WebKit） |
| 内存占用 | ~50MB | ~150MB |
| 包体积 | ~20MB | ~34MB |

**结论**：Tauri 以更高的内存占用为代价，换来了 100% 的设计图还原度和开发效率。

### 1.2 设计原型

打开 `docs/design-prototype.html` 查看高保真设计原型。

---

## 二、系统架构

```
┌─────────────────────────────────────────┐
│           Web 前端（src/）              │
│  ┌─────────┐  ┌─────────────────────┐  │
│  │ 活动栏   │  │  标签栏 + 编辑器     │  │
│  │ SVG 图标 │  │  highlight.js 高亮  │  │
│  ├─────────┤  ├─────────────────────┤  │
│  │ 侧边栏   │  │  终端面板            │  │
│  │ 文件树   │  │  xterm.js（预留）    │  │
│  └─────────┘  └─────────────────────┘  │
│         Tauri IPC（invoke/command）     │
├─────────────────────────────────────────┤
│           Rust 后端（src-tauri/）        │
│  ┌─────────────────────────────────┐    │
│  │  commands.rs  - IPC 命令处理器   │    │
│  │  fs.rs        - 文件树/Git      │    │
│  │  terminal.rs  - PTY 终端        │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

---

## 三、功能清单

### 3.1 已实现功能

| 功能 | 前端实现 | 后端实现 | 状态 |
|------|---------|---------|------|
| 打开项目 | 系统对话框 | `open_project` | ✅ |
| 文件树 | 递归渲染 | `FileTree::new` | ✅ |
| 文件展开/折叠 | CSS 过渡 | - | ✅ |
| 文件图标 | Emoji/SVG | - | ✅ |
| 打开文件 | `openFile()` | `read_file` | ✅ |
| 多标签页 | 标签栏渲染 | - | ✅ |
| 标签切换/关闭 | 事件绑定 | - | ✅ |
| 语法高亮 | highlight.js | - | ✅ |
| 行号显示 | CSS 布局 | - | ✅ |
| 保存文件 | `saveCurrentFile()` | `save_file` | ✅ |
| 搜索文件（⌘+P） | 浮层 + 过滤 | - | ✅ |
| 命令面板（⌘+Shift+P） | 浮层 + 命令列表 | - | ✅ |
| 创建终端 | `createTerminal()` | `create_terminal` | ✅ |
| 终端显示 | 文本渲染 | `write_terminal` | ✅ |
| Git 状态 | 状态栏显示 | `get_git_status` | ✅ |
| 应用图标 | - | `generate_icon.py` | ✅ |

### 3.2 待完善功能

| 功能 | 优先级 | 说明 |
|------|--------|------|
| 文本编辑（contenteditable） | 高 | 当前为只读显示 |
| 终端交互（xterm.js） | 高 | 当前为基础文本显示 |
| LSP 客户端 | 中 | 跳转定义、自动补全 |
| 文件监听 | 中 | 自动刷新文件树 |
| 多窗口 | 低 | 同时打开多个项目 |
| 插件系统 | 低 | WASM 或 JS 插件 |

---

## 四、前端设计

### 4.1 页面结构（`src/index.html`）

```html
<div id="app">
  <!-- 标题栏（macOS 交通灯 + 标题） -->
  <div class="title-bar">...</div>

  <!-- 主容器 -->
  <div class="main-container">
    <!-- 活动栏（SVG 图标） -->
    <div class="activity-bar">...</div>

    <!-- 侧边栏（文件树） -->
    <div class="sidebar">...</div>

    <!-- 编辑区 -->
    <div class="editor-area">
      <div class="tab-bar">...</div>      <!-- 标签栏 -->
      <div class="editor-content">...</div> <!-- 编辑器 -->
      <div class="panel">...</div>          <!-- 终端面板 -->
    </div>
  </div>

  <!-- 状态栏（蓝色） -->
  <div class="status-bar">...</div>

  <!-- 浮层（搜索/命令面板） -->
  <div class="overlay">...</div>
</div>
```

### 4.2 样式设计（`src/style.css`）

采用 **VS Code Dark+** 主题：

| CSS 变量 | 值 | 用途 |
|---------|-----|------|
| `--bg-editor` | `#1e1e1e` | 编辑器背景 |
| `--bg-sidebar` | `#252526` | 侧边栏背景 |
| `--bg-activity` | `#333333` | 活动栏背景 |
| `--bg-tab` | `#2d2d2d` | 标签栏背景 |
| `--bg-status` | `#007acc` | 状态栏背景 |
| `--fg-primary` | `#d4d4d4` | 主要文字 |
| `--fg-secondary` | `#858585` | 次要文字 |
| `--accent` | `#007acc` | 强调色 |

### 4.3 交互逻辑（`src/app.js`）

**状态管理**：
```javascript
const state = {
    projectRoot: null,      // 项目根路径
    fileTree: [],           // 文件树数据
    openTabs: [],           // 打开的标签
    activeTabIndex: -1,     // 当前活动标签
    terminalId: null,       // 当前终端 ID
    gitStatus: null,        // Git 状态
    searchResults: [],      // 搜索结果
    commandResults: [],     // 命令面板结果
};
```

**核心函数**：
- `openProject()` - 打开项目
- `renderFileTree()` - 渲染文件树
- `openFile(path)` - 打开文件
- `renderTabs()` - 渲染标签栏
- `renderEditor()` - 渲染编辑器
- `createTerminal()` - 创建终端
- `openSearch()` / `closeSearch()` - 搜索浮层
- `openCommandPalette()` / `closeCommandPalette()` - 命令面板

---

## 五、后端设计

### 5.1 IPC 命令（`src-tauri/src/commands.rs`）

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `open_project` | `path: String` | `Vec<FileNode>` | 打开项目目录 |
| `read_file` | `path: String` | `FileContent` | 读取文件内容 |
| `save_file` | `path: String, content: String` | `()` | 保存文件 |
| `get_file_tree` | - | `Vec<FileNode>` | 获取文件树 |
| `create_terminal` | `shell: Option<String>` | `TerminalInfo` | 创建终端 |
| `write_terminal` | `id: usize, data: String` | `String` | 写入终端并读取输出 |
| `resize_terminal` | `id: usize, cols: u16, rows: u16` | `()` | 调整终端大小 |
| `get_git_status` | - | `GitInfo` | 获取 Git 状态 |

### 5.2 文件系统（`src-tauri/src/fs.rs`）

```rust
pub struct FileTree {
    pub root: PathBuf,
    pub children: Vec<FileNode>,
}

pub struct GitStatus {
    repo: git2::Repository,
}
```

### 5.3 终端（`src-tauri/src/terminal.rs`）

```rust
pub struct Terminal {
    pub id: usize,
    pub master: std::fs::File,
    pub child: Child,
    pub size: Winsize,
}

pub struct TerminalManager {
    terminals: HashMap<usize, Terminal>,
    next_id: usize,
}
```

---

## 六、应用图标

**设计**：墨斗（木匠弹线工具）
- 方形墨仓 + 圆形线轮 + 墨线 + 金色墨点
- 背景：VS Code 蓝 `#007acc`

**生成脚本**：`scripts/generate_icon.py`

**输出**：
- `src-tauri/icons/icon-*.png`（7 种尺寸）
- `src-tauri/icons/icon.icns`（macOS 图标）

---

## 七、构建与打包

### 7.1 开发模式

```bash
make dev        # cargo tauri dev（热重载）
```

### 7.2 打包

```bash
make bundle     # cargo tauri build
```

### 7.3 输出

- `src-tauri/target/release/bundle/macos/modou.app`
- `src-tauri/target/release/bundle/dmg/modou_0.1.0_aarch64.dmg`

---

## 八、后续规划

### 8.1 短期（1-2 周）

- [ ] 文本编辑（contenteditable 或 Monaco Editor）
- [ ] 终端交互（xterm.js 集成）
- [ ] 文件监听（notify crate + WebSocket 推送）

### 8.2 中期（1-2 月）

- [ ] LSP 客户端（rust-analyzer、gopls 等）
- [ ] 自动补全、跳转定义、悬浮提示
- [ ] 分屏编辑（垂直/水平分割）

### 8.3 长期（3-6 月）

- [ ] 插件系统（JS/WASM 插件）
- [ ] 调试器（DAP 协议）
- [ ] 多窗口支持

---

> 本文档对应墨斗 v0.3.0（Tauri 方案）实现版本。
