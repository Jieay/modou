use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::docking::ax;
use crate::settings::ProviderConfig;
use crate::terminal_provider::{LaunchContext, ProviderError, ProviderWindow, TerminalProvider};

/// iTerm2 / Terminal.app AppleScript provider
pub struct Iterm2Provider {
    config: ProviderConfig,
}

impl Iterm2Provider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    /// 应用名（用于 AppleScript 的 tell application 和 System Events）
    fn app_name(&self) -> &str {
        self.config.app_name.as_deref().unwrap_or("iTerm2")
    }

    /// 探测应用路径
    fn find_app_path(&self) -> Option<String> {
        if let Some(path) = &self.config.app_path {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }
        // iTerm2 的 bundle id
        let bundle = if self.config.id == "terminal" {
            "com.apple.Terminal"
        } else {
            "com.googlecode.iterm2"
        };
        ax::mdfind(bundle)
    }

    /// 通过 osascript stdin 启动终端，返回是否成功
    fn launch_via_applescript(&self, cwd: &str) -> Result<(), ProviderError> {
        let app = self.app_name();
        let script = if self.config.id == "terminal" {
            format!(
                "tell application \"Terminal\"\n  do script \"cd \" & quoted form of \"{}\" & \" && clear\"\n  activate\nend tell",
                cwd
            )
        } else {
            format!(
                "tell application \"iTerm2\"\n  set w to create window with default profile\n  tell current session of w to write text \"cd \" & quoted form of \"{}\" & \" && clear\"\n  activate\nend tell",
                cwd
            )
        };

        let result = Self::run_osascript(&script, Duration::from_secs(30));

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // -1743 = errAEEventNotPermitted，自动化未授权
                    if stderr.contains("-1743") || stderr.contains("not authorized") || stderr.contains("not allowed") {
                        Err(ProviderError::NotAuthorized(format!(
                            "需要授权控制 {}。请前往 系统设置 > 隐私与安全性 > 自动化 允许。",
                            app
                        )))
                    } else {
                        Err(ProviderError::Other(format!(
                            "{} 启动失败: {}",
                            app,
                            stderr.trim()
                        )))
                    }
                }
            }
            Err(e) => Err(ProviderError::Other(format!("osascript 执行失败: {}", e))),
        }
    }

    /// 带超时地执行 osascript（首启授权弹窗可能耗时，30s 内均可等待；
    /// 目标应用崩溃/挂起时避免停靠线程无限阻塞）
    fn run_osascript(script: &str, timeout: Duration) -> Result<std::process::Output, String> {
        use std::io::Read;

        let mut child = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("osascript 启动失败: {}", e))?;

        let start = Instant::now();
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| format!("osascript 等待失败: {}", e))?
            {
                let mut out = String::new();
                let mut err = String::new();
                if let Some(mut so) = child.stdout.take() {
                    let _ = so.read_to_string(&mut out);
                }
                if let Some(mut se) = child.stderr.take() {
                    let _ = se.read_to_string(&mut err);
                }
                return Ok(std::process::Output {
                    status,
                    stdout: out.into_bytes(),
                    stderr: err.into_bytes(),
                });
            }
            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("osascript 执行超时（{}s）", timeout.as_secs()));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// 取 PID：优先 System Events，回退 pgrep
    fn get_pid(&self) -> Option<u32> {
        let app = self.app_name();
        if let Some(pid) = ax::pid_of_process(app) {
            return Some(pid);
        }
        // 回退 pgrep
        let pattern = if self.config.id == "terminal" {
            "Terminal.app/Contents/MacOS/Terminal"
        } else {
            "iTerm.app/Contents/MacOS/iTerm2"
        };
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(pattern)
            .output()
            .ok()?;
        if output.status.success() {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|l| l.trim().parse::<u32>().ok())
                .max()
        } else {
            None
        }
    }

    /// 等待 AX 主窗口出现，最多 timeout_ms
    fn wait_for_window(&self, pid: u32, timeout: Duration) -> Option<ax::AxWindowHandle> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(win) = ax::main_window_of(pid) {
                if !win.is_null() {
                    return Some(win);
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        None
    }
}

impl TerminalProvider for Iterm2Provider {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn is_installed(&self) -> bool {
        self.find_app_path().is_some()
    }

    fn launch(&self, ctx: &LaunchContext) -> Result<ProviderWindow, ProviderError> {
        if !self.is_installed() {
            return Err(ProviderError::NotInstalled(format!(
                "未找到 {}，请确认已安装。",
                self.app_name()
            )));
        }

        let cwd = ctx.cwd.to_string_lossy().to_string();

        // 1. 启动终端
        self.launch_via_applescript(&cwd)?;

        // 2. 取 PID（5s 超时）
        let pid = {
            let start = Instant::now();
            let mut pid = None;
            while start.elapsed() < Duration::from_secs(5) {
                if let Some(p) = self.get_pid() {
                    pid = Some(p);
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
            pid
        }
        .ok_or_else(|| ProviderError::LaunchTimeout(format!("无法获取 {} 的 PID", self.app_name())))?;

        // 3. 等待 AX 窗口出现（3s，失败重试一次）
        let win = self
            .wait_for_window(pid, Duration::from_secs(3))
            .or_else(|| {
                std::thread::sleep(Duration::from_millis(500));
                self.wait_for_window(pid, Duration::from_secs(3))
            })
            .ok_or_else(|| {
                ProviderError::WindowNotFound(format!("未找到 {} 的窗口", self.app_name()))
            })?;

        let title = win.title().unwrap_or_default();

        Ok(ProviderWindow {
            pid,
            ax: Some(win),
            title,
        })
    }

    fn close_window(&self, win: &ProviderWindow) -> Result<(), ProviderError> {
        if let Some(ref ax_win) = win.ax {
            ax_win.close();
            Ok(())
        } else {
            // 没有句柄则尝试重新获取
            if let Some(handle) = ax::main_window_of(win.pid) {
                handle.close();
                Ok(())
            } else {
                Err(ProviderError::WindowNotFound(format!(
                    "无法关闭 {} 窗口：未持有窗口句柄",
                    self.app_name()
                )))
            }
        }
    }
}
