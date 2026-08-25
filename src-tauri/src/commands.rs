use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri::{Emitter, Manager};
use tauri::WebviewWindow;

use crate::fs::{FileTree, GitStatus};
use crate::terminal::TerminalManager;
use notify::Watcher as _;

/// 单个窗口的项目状态（多窗口各自独立，互不覆盖）
#[derive(Default)]
pub struct WindowProjectState {
    pub project_root: Option<PathBuf>,
    pub file_tree: Option<FileTree>,
    pub git_status: Option<GitStatus>,
    /// 项目内的 git 仓库根目录列表（多仓工作区：根目录不是仓库时收集子仓库）
    pub git_repos: Vec<PathBuf>,
    /// 已打开文件的外部变更监听器（终端/AI 修改磁盘文件后通知前端刷新）
    pub file_watcher: Option<notify::RecommendedWatcher>,
    /// 监听路径（含 canonicalize 变体）-> 前端注册的原始路径
    pub watched_files: Arc<Mutex<HashMap<PathBuf, String>>>,
}

pub struct AppState {
    /// 窗口 label -> 该窗口打开的项目状态
    pub windows: Mutex<HashMap<String, WindowProjectState>>,
    /// 窗口 label -> 窗口加载完成后待打开的项目路径（Dock 菜单打开最近项目用）
    pub pending_open: Mutex<HashMap<String, String>>,
    pub terminal_manager: Mutex<TerminalManager>,
    pub dock_manager: Mutex<Option<crate::dock_manager::DockManager>>,
    pub settings: Mutex<crate::settings::TerminalSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            pending_open: Mutex::new(HashMap::new()),
            terminal_manager: Mutex::new(TerminalManager::new()),
            dock_manager: Mutex::new(None),
            settings: Mutex::new(crate::settings::TerminalSettings::default()),
        }
    }
}

/// 窗口关闭时清理其项目状态与待打开记录
pub fn remove_window_state(state: &AppState, label: &str) {
    state.windows.lock().unwrap().remove(label);
    state.pending_open.lock().unwrap().remove(label);
}

/// 窗口启动时取走待打开的项目路径（仅新窗口有值，取走即删）
#[tauri::command]
pub fn take_pending_open(window: WebviewWindow, state: State<'_, AppState>) -> Option<String> {
    state.pending_open.lock().unwrap().remove(window.label())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub id: usize,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub modified: Vec<String>,
    pub added: Vec<String>,
}

#[tauri::command]
pub async fn open_project(
    path: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<FileNode>, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    // 允许 asset 协议访问当前项目目录（图片预览等本地资源加载）
    app.asset_protocol_scope()
        .allow_directory(&path, true)
        .map_err(|e| e.to_string())?;

    let tree = FileTree::new(&path).map_err(|e| e.to_string())?;
    let nodes = tree.to_nodes();

    // git：根目录是仓库则直接用；否则浅层扫描子目录收集子仓库（多仓工作区）
    let git = GitStatus::new(&path).ok();
    let mut repos = Vec::new();
    match &git {
        Some(g) => {
            if let Some(w) = g.workdir() {
                repos.push(w);
            }
        }
        None => {
            repos = crate::fs::find_repo_roots(&path);
        }
    }

    let mut windows = state.windows.lock().unwrap();
    let ws = windows.entry(window.label().to_string()).or_default();
    ws.project_root = Some(path.clone());
    ws.file_tree = Some(tree);
    ws.git_status = git;
    ws.git_repos = repos;

    Ok(nodes)
}

/// 弹出系统目录选择框，返回选中的文件夹路径（取消返回 None）
/// 使用回调式 API，由插件内部调度到主线程执行（AppKit 要求 UI 操作在主线程）
#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder.map(|p| p.to_string()));
    });
    rx.recv().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_dir(path: String) -> Result<Vec<FileNode>, String> {
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return Err("Not a directory".to_string());
    }
    FileTree::scan_level(&path, 0)
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<FileContent, String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let language = detect_language(&path);

    Ok(FileContent {
        path: path.to_string_lossy().to_string(),
        content,
        language,
    })
}

#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// 已打开文件外部变更事件 payload（前端按 window label 过滤 + path 匹配标签）
#[derive(Debug, Clone, Serialize)]
pub struct FileChangedPayload {
    pub window: String,
    pub path: String,
    /// "modified" | "removed"
    pub kind: String,
}

/// 设置当前窗口需要监听的已打开文件列表（整体替换，幂等）。
/// 终端/AI 助手等外部进程修改磁盘文件后，向该窗口发送 file:changed 事件。
#[tauri::command]
pub fn set_watched_files(
    paths: Vec<String>,
    window: WebviewWindow,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let label = window.label().to_string();
    let mut windows = state.windows.lock().unwrap();
    let ws = windows.entry(label.clone()).or_default();

    // 首次调用时创建 watcher。回调内按 watched_files 反查前端注册路径，
    // 避免 macOS 下符号链接（如 /tmp -> /private/tmp）导致事件路径与注册路径不一致
    if ws.file_watcher.is_none() {
        let watched = ws.watched_files.clone();
        let app_handle = app.clone();
        let win_label = label.clone();
        let watcher = notify::recommended_watcher(
            move |res: Result<notify::Event, notify::Error>| {
                let event = match res {
                    Ok(e) => e,
                    Err(_) => return,
                };
                let kind = match event.kind {
                    // Create 覆盖原子保存（写临时文件再改名）场景
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_) => "modified",
                    notify::EventKind::Remove(_) => "removed",
                    _ => return,
                };
                for p in &event.paths {
                    let frontend_path = {
                        let map = watched.lock().unwrap();
                        map.get(p).cloned().or_else(|| {
                            std::fs::canonicalize(p)
                                .ok()
                                .and_then(|c| map.get(&c).cloned())
                        })
                    };
                    if let Some(path) = frontend_path {
                        let _ = app_handle.emit_to(
                            &win_label,
                            "file:changed",
                            FileChangedPayload {
                                window: win_label.clone(),
                                path,
                                kind: kind.to_string(),
                            },
                        );
                    }
                }
            },
        )
        .map_err(|e| e.to_string())?;
        ws.file_watcher = Some(watcher);
    }

    // 目标集合：同时登记原始路径与 canonicalize 后的路径（均映射回前端路径）
    let mut new_map: HashMap<PathBuf, String> = HashMap::new();
    for p in &paths {
        let pb = PathBuf::from(p);
        new_map.insert(pb.clone(), p.clone());
        if let Ok(c) = std::fs::canonicalize(&pb) {
            new_map.insert(c, p.clone());
        }
    }

    let watcher = ws.file_watcher.as_mut().unwrap();
    let mut watched = ws.watched_files.lock().unwrap();

    // 移除不再打开的文件
    let old_keys: Vec<PathBuf> = watched.keys().cloned().collect();
    for key in old_keys {
        if !new_map.contains_key(&key) {
            let _ = watcher.unwatch(&key);
            watched.remove(&key);
        }
    }
    // 注册新增文件（文件可能暂不存在，忽略错误）
    for (key, frontend_path) in &new_map {
        if !watched.contains_key(key) && watcher.watch(key, notify::RecursiveMode::NonRecursive).is_ok() {
            watched.insert(key.clone(), frontend_path.clone());
        }
    }

    Ok(())
}

/// 重命名文件或文件夹（同目录下改名，拒绝覆盖已存在路径）
#[tauri::command]
pub async fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    let old_p = PathBuf::from(&old_path);
    let new_p = PathBuf::from(&new_path);
    if !old_p.exists() {
        return Err("源路径不存在".to_string());
    }
    if new_p.exists() {
        return Err("目标名称已存在".to_string());
    }
    std::fs::rename(&old_p, &new_p).map_err(|e| e.to_string())
}

/// 新建空文件（已存在则报错）
#[tauri::command]
pub async fn create_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.exists() {
        return Err("文件已存在".to_string());
    }
    std::fs::write(&p, "").map_err(|e| e.to_string())
}

/// 新建文件夹（已存在则报错）
#[tauri::command]
pub async fn create_dir(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.exists() {
        return Err("文件夹已存在".to_string());
    }
    std::fs::create_dir(&p).map_err(|e| e.to_string())
}

/// 删除文件或文件夹（文件夹递归删除，不可恢复）
#[tauri::command]
pub async fn delete_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".to_string());
    }
    if p.is_dir() {
        std::fs::remove_dir_all(&p).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&p).map_err(|e| e.to_string())
    }
}

/// 复制文件或文件夹到目标路径（文件夹递归复制，拒绝覆盖已存在路径）
#[tauri::command]
pub async fn copy_path(src_path: String, dst_path: String) -> Result<(), String> {
    fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        if src.is_dir() {
            std::fs::create_dir(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            }
            Ok(())
        } else {
            std::fs::copy(src, dst).map(|_| ())
        }
    }

    let src = PathBuf::from(&src_path);
    let dst = PathBuf::from(&dst_path);
    if !src.exists() {
        return Err("源路径不存在".to_string());
    }
    if dst.exists() {
        return Err("目标路径已存在".to_string());
    }
    copy_recursive(&src, &dst).map_err(|e| e.to_string())
}

/// 完整的 git 变更状态表（文件树徽章用，绝对路径；聚合项目内所有仓库）
#[tauri::command]
pub async fn get_git_changes(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<crate::fs::FileChange>, String> {
    let repos = {
        let windows = state.windows.lock().unwrap();
        windows
            .get(window.label())
            .map(|ws| ws.git_repos.clone())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for root in repos {
        if let Ok(g) = GitStatus::open(&root) {
            out.extend(g.changes());
        }
    }
    Ok(out)
}

/// 当前编辑内容与 HEAD 的行级差异（编辑器 gutter 用；按文件路径向上查找所属仓库）
#[tauri::command]
pub async fn diff_lines(
    path: String,
    content: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<crate::fs::LineDiff, String> {
    let p = Path::new(&path);
    let from = p.parent().unwrap_or(p);
    // 从文件所在目录向上 discover，多仓工作区下自动命中所属子仓库
    match GitStatus::new(from) {
        Ok(g) => Ok(g.diff_lines(p, &content)),
        Err(_) => {
            // 回退到打开项目时的主仓库（兼容旧路径）
            let windows = state.windows.lock().unwrap();
            Ok(match windows.get(window.label()).and_then(|ws| ws.git_status.as_ref()) {
                Some(g) => g.diff_lines(p, &content),
                None => crate::fs::LineDiff::default(),
            })
        }
    }
}

#[tauri::command]
pub async fn get_file_tree(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<FileNode>, String> {
    let windows = state.windows.lock().unwrap();
    match windows.get(window.label()).and_then(|ws| ws.file_tree.as_ref()) {
        Some(t) => Ok(t.to_nodes()),
        None => Err("No project opened".to_string()),
    }
}

/// 用户登录 shell：getpwuid 是权威来源（与 Terminal.app 一致），
/// 不依赖容易被启动环境污染的 $SHELL（如从 bash 会话启动应用时 $SHELL=/bin/bash）
fn default_login_shell() -> String {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() && !(*pw).pw_shell.is_null() {
            let s = std::ffi::CStr::from_ptr((*pw).pw_shell)
                .to_string_lossy()
                .to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

#[tauri::command]
pub async fn create_terminal(
    shell: Option<String>,
    cwd: Option<String>,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<TerminalInfo, String> {
    let shell = shell.unwrap_or_else(default_login_shell);

    // 优先使用传入 cwd，其次当前窗口打开的项目，最后进程工作目录
    let window_root = state
        .windows
        .lock()
        .unwrap()
        .get(window.label())
        .and_then(|ws| ws.project_root.clone());
    let cwd = cwd
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or(window_root)
        .or_else(|| std::env::current_dir().ok());

    let mut manager = state.terminal_manager.lock().unwrap();
    let id = manager.create(&shell, cwd.as_deref()).map_err(|e| e.to_string())?;

    Ok(TerminalInfo {
        id,
        title: format!("Terminal {}", id + 1),
    })
}

#[tauri::command]
pub async fn close_terminal(id: usize, state: State<'_, AppState>) -> Result<(), String> {
    state.terminal_manager.lock().unwrap().close(id);
    Ok(())
}

#[tauri::command]
pub async fn write_terminal(
    id: usize,
    data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut manager = state.terminal_manager.lock().unwrap();
    manager.write(id, data.as_bytes()).map_err(|e| e.to_string())?;
    manager.read(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(
    id: usize,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut manager = state.terminal_manager.lock().unwrap();
    manager.resize(id, cols, rows).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_git_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<GitInfo, String> {
    let (git_opt, first_repo) = {
        let windows = state.windows.lock().unwrap();
        match windows.get(window.label()) {
            Some(ws) => (
                // GitStatus 不便克隆，这里只取是否存在的标记
                ws.git_status.as_ref().map(|g| (g.branch(), g.modified_files(), g.added_files())),
                ws.git_repos.first().cloned(),
            ),
            None => (None, None),
        }
    };
    if let Some((branch, modified, added)) = git_opt {
        return Ok(GitInfo { branch, modified, added });
    }
    // 根目录不是仓库时，回退到第一个子仓库（多仓工作区）
    match first_repo {
        Some(root) => {
            let g = GitStatus::open(&root).map_err(|e| e.to_string())?;
            Ok(GitInfo {
                branch: g.branch(),
                modified: g.modified_files(),
                added: g.added_files(),
            })
        }
        None => Err("No git repository".to_string()),
    }
}

fn detect_language(path: &PathBuf) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "rs" => "rust",
        "go" => "go",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "md" => "markdown",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
       _ => "plaintext",
   }
   .to_string()
}

// ====================================================================
// Dock 命令
// ====================================================================

/// provider 信息（面向前端）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub installed: bool,
}

/// dock 槽矩形（前端上报）
#[derive(Debug, Clone, Deserialize)]
pub struct DockSlotRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 设置文件路径
fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    crate::settings::TerminalSettings::path_for(&config_dir)
}

#[tauri::command]
pub async fn list_terminal_providers(
    state: State<'_, AppState>,
) -> Result<Vec<DockProviderInfo>, String> {
    let settings = state.settings.lock().unwrap().clone();
    let providers = settings
        .providers
        .iter()
        .map(|p| {
            let installed = if let Some(provider) = crate::terminal_provider::create_provider(p) {
                provider.is_installed()
            } else {
                false
            };
            DockProviderInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                kind: format!("{:?}", p.kind).to_lowercase(),
                enabled: p.enabled,
                installed,
            }
        })
        .collect();
    Ok(providers)
}

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(crate::docking::ax::is_trusted())
}

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<bool, String> {
    Ok(crate::docking::ax::request_trust())
}

#[tauri::command]
pub async fn get_terminal_settings(
    state: State<'_, AppState>,
) -> Result<crate::settings::TerminalSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_terminal_settings(
    settings: crate::settings::TerminalSettings,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = settings_path(&app);
    settings.save(&path)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub async fn start_terminal(
    provider_id: String,
    cwd: Option<String>,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<crate::dock_manager::DockSessionView, String> {
    // 只在取 sender 时短暂持锁，等待启动结果期间必须释放，
    // 否则窗口 Moved/Resized 事件会在 on_window_event 里阻塞最长 30 秒
    let dock_tx = {
        let guard = state.dock_manager.lock().unwrap();
        let dock = guard.as_ref().ok_or("Dock manager not initialized")?;
        dock.sender()
    };

    let window_root = state
        .windows
        .lock()
        .unwrap()
        .get(window.label())
        .and_then(|ws| ws.project_root.clone());
    let cwd = match cwd {
        Some(c) => Some(PathBuf::from(c)),
        None => window_root,
    };

    // 同步等待工作线程完成启动（osascript + AX 窗口定位）
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = dock_tx.send(crate::dock_manager::DockCmd::Start {
        provider_id,
        cwd,
        reply: tx,
    });

    // 等待最多 30 秒（osascript 首次运行可能触发授权弹窗）
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|e| format!("启动超时: {}", e))?
}

#[tauri::command]
pub async fn set_dock_slot(
    rect: DockSlotRect,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dock = state.dock_manager.lock().unwrap();
    if let Some(dock) = dock.as_ref() {
        dock.send(crate::dock_manager::DockCmd::UpdateSlot(
            crate::docking::Rect {
                x: rect.x,
                y: rect.y,
                w: rect.width,
                h: rect.height,
            },
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn redock_terminal(
    _session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dock = state.dock_manager.lock().unwrap();
    if let Some(dock) = dock.as_ref() {
        dock.send(crate::dock_manager::DockCmd::Redock);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_terminal(
    _session_id: Option<String>,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let quit_behavior = match mode.as_str() {
        "close" => crate::settings::QuitBehavior::Close,
        "quit" => crate::settings::QuitBehavior::Quit,
        "leave" => crate::settings::QuitBehavior::Leave,
        _ => crate::settings::QuitBehavior::Close,
    };
    let dock = state.dock_manager.lock().unwrap();
    if let Some(dock) = dock.as_ref() {
        dock.send(crate::dock_manager::DockCmd::Stop { mode: quit_behavior });
    }
    Ok(())
}

#[tauri::command]
pub async fn get_dock_session(
    state: State<'_, AppState>,
) -> Result<Option<crate::dock_manager::DockSessionView>, String> {
    let dock = state.dock_manager.lock().unwrap();
    match dock.as_ref() {
        Some(d) => Ok(d.session_view()),
        None => Ok(None),
    }
}

// ====================================================================
// 会话持久化：记住上次打开的项目与文件
// ====================================================================

/// 需要跨重启保留的会话状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    pub project_root: Option<String>,
    pub open_files: Vec<String>,
    pub active_file: Option<String>,
    /// 文件树中处于展开状态的目录路径
    pub expanded_dirs: Vec<String>,
}

fn session_path(app: &tauri::AppHandle) -> PathBuf {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    config_dir.join("session.json")
}

#[tauri::command]
pub async fn save_session(session: SessionState, app: tauri::AppHandle) -> Result<(), String> {
    let path = session_path(&app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_session(app: tauri::AppHandle) -> Result<SessionState, String> {
    let path = session_path(&app);
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(serde_json::from_str(&content).unwrap_or_default()),
        Err(_) => Ok(SessionState::default()),
    }
}
