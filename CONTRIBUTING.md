# 贡献指南

感谢考虑为墨斗（Modou）做贡献！

## 报告 Bug

1. 在 [Issues](../../issues) 中搜索是否已有相同问题
2. 使用 Bug 报告模板创建新 Issue
3. 请描述：复现步骤、预期行为、实际行为、环境信息（macOS 版本、墨斗版本）

## 提交代码

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feature/your-feature`
3. 遵循 Conventional Commits 提交规范：
   ```bash
   git commit -m "feat(scope): 简短描述"
   ```
4. 推送并创建 Pull Request
5. 等待 Code Review

## 开发环境

```bash
git clone https://github.com/Jieay/modou.git
cd modou

# 安装 Tauri CLI
cargo install tauri-cli

# 开发模式（前后端热重载）
make dev
```

## 代码规范

- Rust 后端：提交前运行 `make check` 确保无编译错误，`make test` 通过全部单元测试
- 前端：无构建步骤，直接编辑 `src/` 下的 HTML/CSS/JS
- 提交前请确认 `make check` 与 `make test` 均通过

## PR 审查标准

- 是否解决了 Issue 中描述的问题
- 是否包含足够的测试（如涉及 Rust 逻辑）
- 是否更新了相关文档（README / docs）
