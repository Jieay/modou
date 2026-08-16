use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};

use rustix::termios::Winsize;

#[allow(dead_code)]
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

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create(&mut self, shell: &str, cwd: Option<&Path>) -> Result<usize, String> {
        let size = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let pty_pair = rustix_openpty::openpty(None, Some(&size))
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let master = pty_pair.controller;
        let slave = pty_pair.user;

        let mut cmd = Command::new(shell);
        cmd.env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor");
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // 让子进程成为独立会话首进程，并把 PTY slave 设为它的控制终端。
        // 这样 shell 才有 job control，tmux / vim / htop / lazygit 等 TUI 工具才能正常工作。
        unsafe {
            cmd.pre_exec(move || {
                let fd = slave.as_raw_fd();
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::ioctl(fd, libc::TIOCSCTTY as libc::c_ulong, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                // 把 slave 复制到标准输入 / 输出 / 错误
                libc::dup2(fd, 0);
                libc::dup2(fd, 1);
                libc::dup2(fd, 2);
                if fd > 2 {
                    libc::close(fd);
                }
                Ok(())
            });
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let master_fd = master.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(master_fd, libc::F_GETFL);
            libc::fcntl(master_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let master_file = unsafe { std::fs::File::from_raw_fd(master_fd) };
        std::mem::forget(master);

        let id = self.next_id;
        self.next_id += 1;

        self.terminals.insert(
            id,
            Terminal {
                id,
                master: master_file,
                child,
                size,
            },
        );

        Ok(id)
    }

    pub fn write(&mut self, id: usize, data: &[u8]) -> Result<(), String> {
        let term = self
            .terminals
            .get_mut(&id)
            .ok_or_else(|| "Terminal not found".to_string())?;
        term.master
            .write_all(data)
            .map_err(|e| format!("Write error: {}", e))
    }

    pub fn read(&mut self, id: usize) -> Result<String, String> {
        let term = self
            .terminals
            .get_mut(&id)
            .ok_or_else(|| "Terminal not found".to_string())?;

        let mut buf = [0u8; 4096];
        match term.master.read(&mut buf) {
            Ok(n) if n > 0 => Ok(String::from_utf8_lossy(&buf[..n]).to_string()),
            _ => Ok(String::new()),
        }
    }

    pub fn resize(&mut self, id: usize, cols: u16, rows: u16) -> Result<(), String> {
        let term = self
            .terminals
            .get_mut(&id)
            .ok_or_else(|| "Terminal not found".to_string())?;

        term.size = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        rustix::termios::tcsetwinsize(&term.master, term.size)
            .map_err(|e| format!("Resize error: {}", e))
    }

    pub fn close(&mut self, id: usize) {
        if let Some(mut term) = self.terminals.remove(&id) {
            let pid = term.child.id() as i32;
            // 子进程是会话/进程组首进程，向整个进程组发 SIGHUP，
            // 让 shell 及其后代（tmux / vim / 后台任务等）一起优雅退出。
            unsafe {
                libc::kill(-pid, libc::SIGHUP);
            }
            // 给一点时间优雅退出，再兜底 SIGKILL。
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = term.child.kill();
            let _ = term.child.wait();
        }
    }
}
