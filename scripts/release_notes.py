#!/usr/bin/env python3
"""从 CHANGELOG.md 提取指定版本的段落，输出用作 GitHub Release notes。

用法：
    python3 scripts/release_notes.py 0.1.7
找不到对应版本段落时以非零码退出（调用方回退到自动生成）。
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CHANGELOG = os.path.join(ROOT, "CHANGELOG.md")


def main():
    if len(sys.argv) < 2:
        print("用法: release_notes.py <版本号>", file=sys.stderr)
        sys.exit(1)
    version = sys.argv[1].lstrip("v")

    text = io.open(CHANGELOG, encoding="utf-8").read()
    pattern = re.compile(
        r"^## \[%s\][^\n]*\n(.*?)(?=^## \[|\Z)" % re.escape(version),
        re.MULTILINE | re.DOTALL,
    )
    m = pattern.search(text)
    if not m or not m.group(1).strip():
        print("CHANGELOG.md 中未找到版本 [%s] 的段落" % version, file=sys.stderr)
        sys.exit(1)
    print(m.group(1).strip())


if __name__ == "__main__":
    main()
