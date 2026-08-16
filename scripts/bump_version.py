#!/usr/bin/env python3
"""递增 src-tauri 的版本号（Cargo.toml 与 tauri.conf.json 保持同步）。

用法：
    python3 scripts/bump_version.py            # 递增 patch（默认）
    python3 scripts/bump_version.py minor      # 递增 minor，patch 归零
    python3 scripts/bump_version.py major      # 递增 major，minor/patch 归零
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CARGO_TOML = os.path.join(ROOT, "src-tauri", "Cargo.toml")
TAURI_CONF = os.path.join(ROOT, "src-tauri", "tauri.conf.json")

# 只匹配 [package] 顶层的 version = "..."（行首），不会误伤依赖里的 version
CARGO_VERSION_RE = re.compile(r'(^[ \t]*version\s*=\s*")([^"]+)(")', re.MULTILINE)
CONF_VERSION_RE = re.compile(r'("version"\s*:\s*")([^"]+)(")')


def read(path):
    with io.open(path, "r", encoding="utf-8") as f:
        return f.read()


def write(path, content):
    with io.open(path, "w", encoding="utf-8") as f:
        f.write(content)


def bump(version, part):
    segs = version.split(".")
    if len(segs) < 3:
        raise ValueError("版本号格式不支持: %s" % version)
    major, minor, patch = (int(x) for x in segs[:3])
    if part == "major":
        major, minor, patch = major + 1, 0, 0
    elif part == "minor":
        minor, patch = minor + 1, 0
    else:
        patch += 1
    return "%d.%d.%d" % (major, minor, patch)


def main():
    part = sys.argv[1] if len(sys.argv) > 1 else "patch"
    if part not in ("patch", "minor", "major"):
        print("未知版本段: %s（可选 patch / minor / major）" % part, file=sys.stderr)
        sys.exit(1)

    cargo = read(CARGO_TOML)
    conf = read(TAURI_CONF)

    m_cargo = CARGO_VERSION_RE.search(cargo)
    m_conf = CONF_VERSION_RE.search(conf)
    if not m_cargo or not m_conf:
        print("未能在 Cargo.toml / tauri.conf.json 中找到版本号", file=sys.stderr)
        sys.exit(1)

    old = m_cargo.group(2)
    try:
        new = bump(old, part)
    except ValueError as e:
        print(str(e), file=sys.stderr)
        sys.exit(1)

    cargo = CARGO_VERSION_RE.sub(lambda m: m.group(1) + new + m.group(3), cargo, count=1)
    conf = CONF_VERSION_RE.sub(lambda m: m.group(1) + new + m.group(3), conf, count=1)

    write(CARGO_TOML, cargo)
    write(TAURI_CONF, conf)

    print("版本号已更新: %s -> %s" % (old, new))


if __name__ == "__main__":
    main()
