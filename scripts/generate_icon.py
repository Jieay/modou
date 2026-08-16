#!/usr/bin/env python3
"""
墨斗（Modou）应用图标生成器

设计理念：
- 墨斗是中国古代木匠弹线工具，象征精准与规矩
- 图标采用简约几何风格：方形墨仓 + 圆形线轮 + 墨线
- 配色：VS Code 蓝（#007acc）背景 + 白色图形
- 现代扁平化设计，适配 macOS Big Sur+ 圆角矩形风格
"""

from PIL import Image, ImageDraw
import os

# 输出目录
OUTPUT_DIR = "src-tauri/icons"
os.makedirs(OUTPUT_DIR, exist_ok=True)

# 设计参数
BG_COLOR = (0, 122, 204, 255)  # #007acc VS Code 蓝
FG_COLOR = (255, 255, 255, 255)  # 白色
ACCENT_COLOR = (255, 215, 0, 255)  # 金色点缀

# macOS 图标尺寸
SIZES = [16, 32, 64, 128, 256, 512, 1024]


def draw_modou_icon(size: int) -> Image.Image:
    """绘制墨斗图标"""
    # 创建正方形画布（带圆角）
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 计算圆角半径（macOS 风格约为 22.5%）
    radius = int(size * 0.225)

    # 绘制圆角矩形背景
    draw.rounded_rectangle(
        [(0, 0), (size - 1, size - 1)],
        radius=radius,
        fill=BG_COLOR,
    )

    # 计算图形元素位置和尺寸
    center = size // 2
    scale = size / 1024  # 基于 1024 基准缩放

    # 1. 绘制墨仓（方形，略带圆角）
    ink_width = int(360 * scale)
    ink_height = int(480 * scale)
    ink_x = center - ink_width // 2
    ink_y = center - ink_height // 2 - int(40 * scale)
    ink_radius = int(40 * scale)

    draw.rounded_rectangle(
        [(ink_x, ink_y), (ink_x + ink_width, ink_y + ink_height)],
        radius=ink_radius,
        fill=FG_COLOR,
    )

    # 2. 绘制墨仓顶部凹槽
    groove_width = int(240 * scale)
    groove_height = int(60 * scale)
    groove_x = center - groove_width // 2
    groove_y = ink_y + int(40 * scale)
    groove_radius = int(20 * scale)

    draw.rounded_rectangle(
        [(groove_x, groove_y), (groove_x + groove_width, groove_y + groove_height)],
        radius=groove_radius,
        fill=BG_COLOR,
    )

    # 3. 绘制线轮（圆形）
    wheel_radius = int(140 * scale)
    wheel_x = center - wheel_radius
    wheel_y = ink_y + ink_height - wheel_radius - int(20 * scale)

    draw.ellipse(
        [(wheel_x, wheel_y), (wheel_x + wheel_radius * 2, wheel_y + wheel_radius * 2)],
        fill=FG_COLOR,
    )

    # 4. 绘制线轮中心孔
    hole_radius = int(50 * scale)
    hole_x = center - hole_radius
    hole_y = wheel_y + wheel_radius - hole_radius

    draw.ellipse(
        [(hole_x, hole_y), (hole_x + hole_radius * 2, hole_y + hole_radius * 2)],
        fill=BG_COLOR,
    )

    # 5. 绘制墨线（从线轮延伸的曲线）
    line_start_x = center + wheel_radius - int(10 * scale)
    line_start_y = wheel_y + wheel_radius // 2
    line_end_x = center + int(280 * scale)
    line_end_y = line_start_y + int(80 * scale)
    line_width = max(int(24 * scale), 2)

    # 绘制墨线（贝塞尔曲线效果，用多段线模拟）
    points = []
    for t in range(21):
        t_norm = t / 20
        # 三次贝塞尔曲线
        x = (
            (1 - t_norm) ** 3 * line_start_x
            + 3 * (1 - t_norm) ** 2 * t_norm * (line_start_x + 100 * scale)
            + 3 * (1 - t_norm) * t_norm**2 * (line_end_x - 50 * scale)
            + t_norm**3 * line_end_x
        )
        y = (
            (1 - t_norm) ** 3 * line_start_y
            + 3 * (1 - t_norm) ** 2 * t_norm * (line_start_y - 30 * scale)
            + 3 * (1 - t_norm) * t_norm**2 * (line_end_y + 20 * scale)
            + t_norm**3 * line_end_y
        )
        points.append((x, y))

    # 绘制墨线
    for i in range(len(points) - 1):
        draw.line([points[i], points[i + 1]], fill=FG_COLOR, width=line_width)

    # 6. 绘制墨线末端墨点
    dot_radius = int(30 * scale)
    dot_x = line_end_x - dot_radius
    dot_y = line_end_y - dot_radius

    draw.ellipse(
        [(dot_x, dot_y), (dot_x + dot_radius * 2, dot_y + dot_radius * 2)],
        fill=ACCENT_COLOR,
    )

    return img


def generate_all_sizes():
    """生成所有尺寸的图标"""
    for size in SIZES:
        img = draw_modou_icon(size)
        filename = os.path.join(OUTPUT_DIR, f"icon-{size}x{size}.png")
        img.save(filename, "PNG")
        print(f"Generated: {filename}")

    # 生成 macOS 特殊尺寸
    mac_sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.icns": None,  # 需要 iconutil 生成
    }

    for filename, size in mac_sizes.items():
        if size:
            img = draw_modou_icon(size)
            filepath = os.path.join(OUTPUT_DIR, filename)
            img.save(filepath, "PNG")
            print(f"Generated: {filepath}")


def create_icns():
    """使用 iconutil 创建 .icns 文件（macOS 专用）"""
    # 创建 iconset 目录
    iconset_dir = os.path.join(OUTPUT_DIR, "icon.iconset")
    os.makedirs(iconset_dir, exist_ok=True)

    # macOS iconset 需要的尺寸
    iconset_sizes = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]

    for filename, size in iconset_sizes:
        img = draw_modou_icon(size)
        filepath = os.path.join(iconset_dir, filename)
        img.save(filepath, "PNG")

    # 使用 iconutil 生成 .icns
    icns_path = os.path.join(OUTPUT_DIR, "icon.icns")
    os.system(f"iconutil -c icns '{iconset_dir}' -o '{icns_path}'")
    print(f"Generated: {icns_path}")

    # 清理 iconset 目录
    import shutil
    shutil.rmtree(iconset_dir)


if __name__ == "__main__":
    print("Generating Modou app icons...")
    generate_all_sizes()
    create_icns()
    print("Done!")
