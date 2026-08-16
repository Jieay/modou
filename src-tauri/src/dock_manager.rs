use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::docking::{css_to_global, Rect};
use crate::settings::{QuitBehavior, TerminalSettings};
use crate::terminal_provider::{self, LaunchContext, ProviderWindow, TerminalProvider};

/// Dock 命令
pub enum DockCmd {
    Start {
        provider_id: String,
        cwd: Option<PathBuf>,
        reply: Sender<Result<DockSessionView, String>>,
    },
    UpdateSlot(Rect),
    Redock,
    Stop { mode: QuitBehavior },
    Shutdown,
}

/// Dock 状态
#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DockState {
    Launching,
    Docked,
    Detached,
    Exited,
    Error,
}

/// 面向前端的会话视图
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockSessionView {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub title: String,
    pub state: DockState,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockStatePayload {
    session_id: String,
    provider_id: String,
    state: DockState,
    message: String,
}

/// DockManager
pub struct DockManager {
    tx: Sender<DockCmd>,
    session: Arc<Mutex<Option<DockSessionView>>>,
}

impl DockManager {
    /// 获取命令发送器的克隆（用于在持锁时间外发送命令）
    pub fn sender(&self) -> Sender<DockCmd> {
        self.tx.clone()
    }

    pub fn new(app: AppHandle, window: WebviewWindow) -> Self {
        let (tx, rx) = mpsc::channel::<DockCmd>();
        let session = Arc::new(Mutex::new(None));

        let session_clone = session.clone();
        let app_clone = app.clone();
        std::thread::spawn(move || {
            Self::worker_loop(rx, window, app_clone, session_clone);
        });

        Self { tx, session }
    }

    pub fn send(&self, cmd: DockCmd) {
        let _ = self.tx.send(cmd);
    }

    pub fn session_view(&self) -> Option<DockSessionView> {
        self.session.lock().unwrap().clone()
    }

    fn worker_loop(
        rx: Receiver<DockCmd>,
        window: WebviewWindow,
        app: AppHandle,
        session: Arc<Mutex<Option<DockSessionView>>>,
    ) {
        let mut state = WorkerState::default();

        // 启动后先加载设置
        state.settings = Self::load_settings(&app);

        loop {
            let interval = Duration::from_millis(state.settings.redock_interval_ms.max(50));
            match rx.recv_timeout(interval) {
                Ok(cmd) => {
                    if Self::handle_cmd(&mut state, cmd, &window, &app, &session) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if state.settings.follow_window && state.has_active_window() {
                        Self::do_redock(&mut state, &window, &app, &session);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle_cmd(
        state: &mut WorkerState,
        cmd: DockCmd,
        window: &WebviewWindow,
        app: &AppHandle,
        session: &Arc<Mutex<Option<DockSessionView>>>,
    ) -> bool {
        match cmd {
            DockCmd::Start { provider_id, cwd, reply } => {
                Self::do_start(state, provider_id, cwd, window, app, session, reply);
            }
            DockCmd::UpdateSlot(rect) => {
                state.slot_rect = rect;
                if state.has_active_window() {
                    Self::do_redock(state, window, app, session);
                }
            }
            DockCmd::Redock => {
                if state.has_active_window() {
                    Self::do_redock(state, window, app, session);
                }
            }
            DockCmd::Stop { mode } => {
                Self::do_stop(state, mode, app, session);
            }
            DockCmd::Shutdown => {
                Self::do_stop(state, QuitBehavior::Close, app, session);
                return true;
            }
        }
        false
    }

    fn do_start(
        state: &mut WorkerState,
        provider_id: String,
        cwd: Option<PathBuf>,
        window: &WebviewWindow,
        app: &AppHandle,
        session: &Arc<Mutex<Option<DockSessionView>>>,
        reply: Sender<Result<DockSessionView, String>>,
    ) {
        let reply_err = |reply: &Sender<Result<DockSessionView, String>>, msg: String| {
            let _ = reply.send(Err(msg));
        };

        // 已有会话则先关闭
        if state.has_active_window() {
            Self::do_stop(state, QuitBehavior::Close, app, session);
        }

        state.settings = Self::load_settings(app);
        state.provider_id = provider_id.clone();

        let config = match state.settings.providers.iter().find(|p| p.id == provider_id) {
            Some(c) => c.clone(),
            None => {
                let msg = format!("未找到 provider: {}", provider_id);
                Self::emit_state(session, app, &provider_id, &provider_id, DockState::Error, &msg);
                reply_err(&reply, msg);
                return;
            }
        };

        state.provider_name = config.name.clone();

        if !config.enabled {
            let msg = format!("{} 已在设置中禁用", config.name);
            Self::emit_state(session, app, &provider_id, &config.name, DockState::Error, &msg);
            reply_err(&reply, msg);
            return;
        }

        Self::emit_state(session, app, &provider_id, &config.name,
                         DockState::Launching, "正在启动终端...");

        let provider = match terminal_provider::create_provider(&config) {
            Some(p) => p,
            None => {
                let msg = "该 provider 在当前平台不支持".to_string();
                Self::emit_state(session, app, &provider_id, &config.name, DockState::Error, &msg);
                reply_err(&reply, msg);
                return;
            }
        };

        if !provider.is_installed() {
            let msg = format!("未安装 {}", config.name);
            Self::emit_state(session, app, &provider_id, &config.name, DockState::Error, &msg);
            reply_err(&reply, msg);
            return;
        }

        // 检查 AX 权限
        if !crate::docking::ax::is_trusted() {
            let msg = "需要辅助功能权限。请前往 系统设置 > 隐私与安全性 > 辅助功能 授权。".to_string();
            Self::emit_state(session, app, &provider_id, &config.name, DockState::Error, &msg);
            reply_err(&reply, msg);
            return;
        }

        let cwd = cwd.unwrap_or_else(|| {
            std::env::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        });
        let project_name = cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        let ctx = LaunchContext { cwd, project_name };

        match provider.launch(&ctx) {
            Ok(pwin) => {
                state.provider = Some(provider);
                state.current_window = Some(pwin);
                state.session_id = format!("dock-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis());
                state.consecutive_deviation = 0;
                // 终端启动后会抢占焦点，前几轮必须强制贴合，否则首轮不会吸附
                state.initial_fits_remaining = 3;

                let view = Self::emit_state(
                    session, app, &state.provider_id, &state.provider_name,
                    DockState::Docked, "",
                );

                // 首轮贴合
                Self::do_redock(state, window, app, session);

                let _ = reply.send(Ok(view));
            }
            Err(e) => {
                let msg = e.to_string();
                Self::emit_state(session, app, &provider_id, &config.name, DockState::Error, &msg);
                reply_err(&reply, msg);
            }
        }
    }

    fn do_redock(
        state: &mut WorkerState,
        window: &WebviewWindow,
        app: &AppHandle,
        session: &Arc<Mutex<Option<DockSessionView>>>,
    ) {
        let pwin = match state.current_window.as_ref() {
            Some(w) => w,
            None => return,
        };
        let ax_win = match pwin.ax.as_ref() {
            Some(a) => a,
            None => return,
        };
        if state.slot_rect.w < 1.0 {
            return;
        }

        let (inner_x, inner_y, scale) = match Self::read_window_geometry(window) {
            Some(v) => v,
            None => return,
        };

        let global = css_to_global(state.slot_rect, inner_x, inner_y, scale);
        let focused = window.is_focused().unwrap_or(false);
        let forced = state.initial_fits_remaining > 0;

        if focused || forced {
            if forced {
                ax_win.set_frame_force(global);
                state.initial_fits_remaining -= 1;
            } else {
                ax_win.set_frame(global, 2.0);
            }
            state.consecutive_deviation = 0;
            if let Some(ref sv) = session.lock().unwrap().as_ref() {
                if sv.state == DockState::Detached {
                    Self::emit_state(session, app, &state.provider_id,
                                     &state.provider_name, DockState::Docked, "");
                }
            }
        } else {
            if let Some(cur) = ax_win.frame() {
                let dx = (cur.x - global.x).abs();
                let dy = (cur.y - global.y).abs();
                let dw = (cur.w - global.w).abs();
                let dh = (cur.h - global.h).abs();
                if dx > 4.0 || dy > 4.0 || dw > 4.0 || dh > 4.0 {
                    state.consecutive_deviation += 1;
                    if state.consecutive_deviation >= 12 {
                        Self::emit_state(session, app, &state.provider_id,
                                         &state.provider_name, DockState::Detached,
                                         "终端窗口已脱离停靠区域");
                    }
                } else {
                    state.consecutive_deviation = 0;
                }
            }
        }
    }

    fn do_stop(
        state: &mut WorkerState,
        mode: QuitBehavior,
        app: &AppHandle,
        session: &Arc<Mutex<Option<DockSessionView>>>,
    ) {
        match mode {
            QuitBehavior::Leave => {
                // 保留窗口句柄，展开停靠槽时可 Redock 重新吸回
                state.initial_fits_remaining = 3;
                Self::emit_state(session, app, &state.provider_id, &state.provider_name,
                                 DockState::Detached, "终端已折叠");
                return;
            }
            QuitBehavior::Close | QuitBehavior::Quit => {
                if let Some(ref provider) = state.provider {
                    if let Some(ref pwin) = state.current_window {
                        let _ = provider.close_window(pwin);
                    }
                }
            }
        }
        state.provider = None;
        state.current_window = None;
        Self::emit_state(session, app, &state.provider_id, &state.provider_name,
                         DockState::Exited, "终端已停止");
    }

    fn read_window_geometry(window: &WebviewWindow) -> Option<(i32, i32, f64)> {
        let pos = window.inner_position().ok()?;
        let scale = window.scale_factor().unwrap_or(1.0);
        Some((pos.x, pos.y, scale))
    }

    fn load_settings(app: &AppHandle) -> TerminalSettings {
        match app.path().app_config_dir() {
            Ok(dir) => TerminalSettings::load(&TerminalSettings::path_for(&dir)),
            Err(_) => TerminalSettings::default(),
        }
    }

    fn emit_state(
        session: &Arc<Mutex<Option<DockSessionView>>>,
        app: &AppHandle,
        provider_id: &str,
        provider_name: &str,
        state: DockState,
        message: &str,
    ) -> DockSessionView {
        let existing = session.lock().unwrap().clone();
        let id = existing
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| {
                format!("dock-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis())
            });

        let view = DockSessionView {
            id: id.clone(),
            provider_id: provider_id.to_string(),
            provider_name: provider_name.to_string(),
            title: String::new(),
            state: state.clone(),
            message: message.to_string(),
        };

        // 保留 session（包括 Error/Exited 状态），前端根据 state 显示
        *session.lock().unwrap() = Some(view.clone());

        let payload = DockStatePayload {
            session_id: id,
            provider_id: provider_id.to_string(),
            state,
            message: message.to_string(),
        };
        let _ = app.emit("dock://state", payload);

        view
    }
}

struct WorkerState {
    settings: TerminalSettings,
    provider: Option<Box<dyn TerminalProvider>>,
    current_window: Option<ProviderWindow>,
    provider_id: String,
    provider_name: String,
    session_id: String,
    slot_rect: Rect,
    consecutive_deviation: u32,
    initial_fits_remaining: u32,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            settings: TerminalSettings::default(),
            provider: None,
            current_window: None,
            provider_id: String::new(),
            provider_name: String::new(),
            session_id: String::new(),
            slot_rect: Rect::default(),
            consecutive_deviation: 0,
            initial_fits_remaining: 0,
        }
    }
}

impl WorkerState {
    fn has_active_window(&self) -> bool {
        self.provider.is_some() && self.current_window.is_some()
    }
}

unsafe impl Send for ProviderWindow {}
unsafe impl Sync for ProviderWindow {}
