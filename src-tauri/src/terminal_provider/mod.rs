use std::path::PathBuf;

use crate::docking::ax::AxWindowHandle;

#[cfg(target_os = "macos")]
pub mod iterm2;

#[cfg(target_os = "macos")]
pub mod cli;

/// 启动上下文
pub struct LaunchContext {
    pub cwd: PathBuf,
    pub project_name: String,
}

/// provider 返回的窗口信息
pub struct ProviderWindow {
    pub pid: u32,
    pub ax: Option<AxWindowHandle>,
    pub title: String,
}

/// provider 错误
#[derive(Debug, Clone)]
pub enum ProviderError {
    NotInstalled(String),
    NotAuthorized(String),
    LaunchTimeout(String),
    WindowNotFound(String),
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::NotInstalled(m) => write!(f, "未安装: {}", m),
            ProviderError::NotAuthorized(m) => write!(f, "未授权: {}", m),
            ProviderError::LaunchTimeout(m) => write!(f, "启动超时: {}", m),
            ProviderError::WindowNotFound(m) => write!(f, "窗口未找到: {}", m),
            ProviderError::Other(m) => write!(f, "{}", m),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Terminal provider trait
pub trait TerminalProvider: Send + Sync {
    fn id(&self) -> &str;
    fn is_installed(&self) -> bool;
    fn launch(&self, ctx: &LaunchContext) -> Result<ProviderWindow, ProviderError>;
    fn close_window(&self, win: &ProviderWindow) -> Result<(), ProviderError>;
}

/// 注册表信息（面向前端）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub installed: bool,
}

/// 根据 provider config 创建 provider 实例
pub fn create_provider(config: &crate::settings::ProviderConfig) -> Option<Box<dyn TerminalProvider>> {
   #[cfg(target_os = "macos")]
   {
        match config.kind {
            crate::settings::ProviderKind::Applescript => match config.id.as_str() {
                "iterm2" | "terminal" => {
                    Some(Box::new(iterm2::Iterm2Provider::new(config.clone())))
                }
                _ => None,
            },
            crate::settings::ProviderKind::Cli => {
                Some(Box::new(cli::CliProvider::new(config.clone())))
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = config;
        None
    }
}
