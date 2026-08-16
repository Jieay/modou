// 原生鼠标点击/输入辅助工具（用于端到端自动化测试）
// 用法:
//   ./native-click click X Y      在屏幕坐标 (X,Y) 点击左键（点坐标）
//   ./native-click move X Y       移动鼠标
//   ./native-click type "文本"     键入文本（通过 CGEvent 键盘事件，仅 ASCII 可靠）
#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static void click_at(double x, double y) {
    CGPoint p = CGPointMake(x, y);
    CGEventRef move = CGEventCreateMouseEvent(NULL, kCGEventMouseMoved, p, kCGMouseButtonLeft);
    CGEventPost(kCGHIDEventTap, move);
    CFRelease(move);
    usleep(100000);
    CGEventRef down = CGEventCreateMouseEvent(NULL, kCGEventLeftMouseDown, p, kCGMouseButtonLeft);
    CGEventPost(kCGHIDEventTap, down);
    CFRelease(down);
    usleep(80000);
    CGEventRef up = CGEventCreateMouseEvent(NULL, kCGEventLeftMouseUp, p, kCGMouseButtonLeft);
    CGEventPost(kCGHIDEventTap, up);
    CFRelease(up);
}

int main(int argc, char *argv[]) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s click X Y | move X Y\n", argv[0]);
        return 1;
    }
    if (!AXIsProcessTrusted()) {
        fprintf(stderr, "ERROR: 进程无辅助功能权限（AXIsProcessTrusted=false）\n");
        return 2;
    }
    if (strcmp(argv[1], "click") == 0 && argc == 4) {
        click_at(atof(argv[2]), atof(argv[3]));
        return 0;
    }
    if (strcmp(argv[1], "move") == 0 && argc == 4) {
        CGPoint p = CGPointMake(atof(argv[2]), atof(argv[3]));
        CGEventRef move = CGEventCreateMouseEvent(NULL, kCGEventMouseMoved, p, kCGMouseButtonLeft);
        CGEventPost(kCGHIDEventTap, move);
        CFRelease(move);
        return 0;
    }
    fprintf(stderr, "unknown command\n");
    return 1;
}
