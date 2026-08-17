use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use tauri::Manager;

use crate::fs::{FileTree, GitStatus};
use crate::terminal::TerminalManager;

pub struct AppState {
    pub project_root: Mutex<Option<PathBuf>>,
    pub file_tree: Mutex<Option<FileTree>>,
    pub terminal_manager: Mutex<TerminalManager>,
    pub git_status: Mutex<Option<GitStatus>>,
    pub dock_manager: Mutex<Option<crate::dock_manager::DockManager>>,
    pub settings: Mutex<crate::settings::TerminalSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project_root: Mutex::new(None),
            file_tree: Mutex::new(None),
            terminal_manager: Mutex::new(TerminalManager::new()),
            git_status: Mutex::new(None),
            dock_manager: Mutex::new(None),
            settings: Mutex::new(crate::settings::TerminalSettings::default()),
        }
    }
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
    state: State<'_, AppState>,
) -> Result<Vec<FileNode>, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let tree = FileTree::new(&path).map_err(|e| e.to_string())?;
    let nodes = tree.to_nodes();

    *state.project_root.lock().unwrap() = Some(path.clone());
    *state.file_tree.lock().unwrap() = Some(tree);
    *state.git_status.lock().unwrap() = GitStatus::new(&path).ok();

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

#[tauri::command]
pub async fn get_file_tree(state: State<'_, AppState>) -> Result<Vec<FileNode>, String> {
    let tree = state.file_tree.lock().unwrap();
    match tree.as_ref() {
        Some(t) => Ok(t.to_nodes()),
        None => Err("No project opened".to_string()),
    }
}

#[tauri::command]
pub async fn create_terminal(
    shell: Option<String>,
    cwd: Option<String>,
    state: State<'_, AppState>,
) -> Result<TerminalInfo, String> {
    let shell = shell.unwrap_or_else(|| {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    });

    // 优先使用传入 cwd，其次当前打开的项目，最后进程工作目录
    let cwd = cwd
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| state.project_root.lock().unwrap().clone())
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
pub async fn get_git_status(state: State<'_, AppState>) -> Result<GitInfo, String> {
    let git = state.git_status.lock().unwrap();
    match git.as_ref() {
        Some(g) => Ok(GitInfo {
            branch: g.branch(),
            modified: g.modified_files(),
            added: g.added_files(),
        }),
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
    state: State<'_, AppState>,
) -> Result<crate::dock_manager::DockSessionView, String> {
    // 只在取 sender 时短暂持锁，等待启动结果期间必须释放，
    // 否则窗口 Moved/Resized 事件会在 on_window_event 里阻塞最长 30 秒
    let dock_tx = {
        let guard = state.dock_manager.lock().unwrap();
        let dock = guard.as_ref().ok_or("Dock manager not initialized")?;
        dock.sender()
    };

    let cwd = match cwd {
        Some(c) => Some(PathBuf::from(c)),
        None => state.project_root.lock().unwrap().clone(),
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
