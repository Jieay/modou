use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::docking::ax;
use crate::settings::ProviderConfig;
use crate::terminal_provider::{LaunchContext, ProviderError, ProviderWindow, TerminalProvider};

/// CLI-based terminal provider (WezTerm / Alacritty / kitty / custom command)
pub struct CliProvider {
    config: ProviderConfig,
}

impl CliProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    fn executable(&self) -> Option<&str> {
        self.config.command.first().map(|s| s.as_str())
    }

    fn find_executable(&self) -> Option<String> {
        let exe = self.executable()?;
        if let Some(ref path) = self.config.app_path {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }
        let output = Command::new("which")
            .arg(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()?
                .trim()
                .to_string();
            if !path.is_empty() && Path::new(&path).exists() {
                return Some(path);
            }
        }
        None
    }

    fn build_args(&self, cwd: &str) -> Vec<String> {
        self.config
            .command
            .iter()
            .skip(1)
            .map(|arg| arg.replace("{cwd}", cwd))
            .collect()
    }

    fn launch_process(&self, cwd: &str) -> Result<u32, ProviderError> {
        let exe = self.find_executable().ok_or_else(|| {
            ProviderError::NotInstalled(format!(
                "未找到 {}",
                self.executable().unwrap_or("命令")
            ))
        })?;

        let args = self.build_args(cwd);

        let child = Command::new(&exe)
            .args(&args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ProviderError::Other(format!("启动失败: {}", e)))?;

        let pid = child.id();
        drop(child);
        Ok(pid)
    }

    fn find_window(&self, child_pid: u32, timeout: Duration) -> Option<ax::AxWindowHandle> {
        let start = Instant::now();
        let app_name = self.config.app_name.as_deref();

        while start.elapsed() < timeout {
            if let Some(win) = ax::main_window_of(child_pid) {
                if !win.is_null() {
                    return Some(win);
                }
            }
            if let Some(name) = app_name {
                if let Some(pid) = ax::pid_of_process(name) {
                    if let Some(win) = ax::main_window_of(pid) {
                        if !win.is_null() {
                            return Some(win);
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        None
    }
}

impl TerminalProvider for CliProvider {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn is_installed(&self) -> bool {
        self.find_executable().is_some()
    }

    fn launch(&self, ctx: &LaunchContext) -> Result<ProviderWindow, ProviderError> {
        if !self.is_installed() {
            let exe = self.executable().unwrap_or("命令");
            return Err(ProviderError::NotInstalled(format!(
                "未找到 {}，请确认已安装。",
                exe
            )));
        }

        let cwd = ctx.cwd.to_string_lossy().to_string();
        let child_pid = self.launch_process(&cwd)?;

        let win = self
            .find_window(child_pid, Duration::from_secs(4))
            .or_else(|| {
                std::thread::sleep(Duration::from_millis(500));
                self.find_window(child_pid, Duration::from_secs(3))
            })
            .ok_or_else(|| {
                let name = self.config.app_name.as_deref().unwrap_or("终端");
                ProviderError::WindowNotFound(format!("未找到 {} 的窗口", name))
            })?;

        let pid = self
            .config
            .app_name
            .as_deref()
            .and_then(ax::pid_of_process)
            .unwrap_or(child_pid);

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
        } else if let Some(handle) = ax::main_window_of(win.pid) {
            handle.close();
            Ok(())
        } else {
            let name = self.config.app_name.as_deref().unwrap_or("终端");
            Err(ProviderError::WindowNotFound(format!(
                "无法关闭 {} 窗口",
                name
            )))
        }
    }
}
