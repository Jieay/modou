.PHONY: build release bundle dmg app clean clean-releases run check test install open help icons bump-version publish

# 默认目标：显示帮助信息
all: help

# 检查代码编译错误
check:
	cd src-tauri && cargo check

# 运行开发模式（前后端热重载）
dev:
	cargo tauri dev

# 递增版本号（默认 patch；可 VERSION_PART=minor / major）
VERSION_PART ?= patch
bump-version:
	python3 scripts/bump_version.py $(VERSION_PART)

# 构建 release 优化版本（先自动递增版本号）
release: bump-version
	cargo tauri build

# 打包 macOS 应用（.app 和 .dmg）
bundle: release

# 仅打包 .app 应用包
app: bundle

# 仅打包 .dmg 安装镜像
dmg: bundle

# 生成应用图标
icons:
	.venv/bin/python scripts/generate_icon.py

# 运行测试
test:
	cd src-tauri && cargo test

# 清理所有构建产物
clean:
	cd src-tauri && cargo clean

# 清理历史打包版本（保留最新的 .dmg；.app 每次构建会覆盖，天然是最新）
clean-releases:
	@ls -t src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | tail -n +2 | while read -r f; do rm -f "$$f"; done
	@echo "已清理历史版本 .dmg（保留最新）"

# 安装应用到 /Applications 目录
install: bundle
	cp -R src-tauri/target/release/bundle/macos/modou.app /Applications/

# 打包并打开应用
open: bundle
	open src-tauri/target/release/bundle/macos/modou.app

# 一键发布新版本：递增版本号 → 构建 → 提交 → 打 Tag → 推送 → 创建 GitHub Release 并上传 .dmg
# Release notes 取自 CHANGELOG.md 对应版本段落（无该段落时回退为 GitHub 自动生成）
# 用法：make publish（默认 patch；可 VERSION_PART=minor / major）
publish: release
	@VERSION=$$(python3 -c "import json;print(json.load(open('src-tauri/tauri.conf.json'))['version'])"); \
	git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md; \
	git commit -m "chore: 发布 v$$VERSION" \
		-m "Co-Authored-By: $$(git config user.name) <$$(git config user.email)>"; \
	git tag -a "v$$VERSION" -m "Release v$$VERSION"; \
	git push origin HEAD; \
	git push origin "v$$VERSION"; \
	DMG=$$(ls src-tauri/target/release/bundle/dmg/*_$$VERSION_*.dmg | head -1); \
	if python3 scripts/release_notes.py "$$VERSION" > /tmp/modou_release_notes.md 2>/dev/null; then \
		NOTES_OPT="--notes-file /tmp/modou_release_notes.md"; \
	else \
		NOTES_OPT="--generate-notes"; \
	fi; \
	gh release create "v$$VERSION" "$$DMG" --title "v$$VERSION" $$NOTES_OPT; \
	echo "已发布 v$$VERSION 并上传 $$DMG"

# 显示帮助信息
help:
	@echo "墨斗（Modou）构建系统"
	@echo ""
	@echo "使用方法：make [命令]"
	@echo ""
	@echo "可用命令："
	@echo "  make dev       - 运行开发模式（前后端热重载）"
	@echo "  make release   - 构建 release 优化版本（自动递增版本号）"
	@echo "  make bundle    - 打包 .app 应用包和 .dmg 安装镜像（自动递增版本号）"
	@echo "  make app       - 仅打包 .app 应用包"
	@echo "  make dmg       - 仅打包 .dmg 安装镜像"
	@echo "  make icons     - 生成应用图标"
	@echo "  make check     - 检查代码编译错误"
	@echo "  make test      - 运行单元测试"
	@echo "  make clean     - 清理所有构建产物"
	@echo "  make clean-releases - 清理历史打包版本（保留最新 .dmg）"
	@echo "  make install   - 安装应用到 /Applications 目录"
	@echo "  make open      - 打包并打开应用"
	@echo "  make publish   - 一键发布新版本（构建、打 Tag、推送、创建 GitHub Release）"
	@echo "  make help      - 显示此帮助信息"
