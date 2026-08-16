use serde::{Deserialize, Serialize};
use std::path::Path;

/// 终端停靠设置，序列化为 settings.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalSettings {
    pub default_provider: String,
    pub width_ratio: f64,
    pub follow_window: bool,
    pub auto_redock: bool,
    pub redock_interval_ms: u64,
    pub quit_behavior: QuitBehavior,
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QuitBehavior {
    Close,
    Quit,
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Applescript,
    Cli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub enabled: bool,
    pub app_name: Option<String>,
    pub app_path: Option<String>,
    pub command: Vec<String>,
    #[serde(default = "default_extra")]
    pub extra: serde_json::Value,
}

fn default_extra() -> serde_json::Value {
    serde_json::json!({})
}

impl Default for QuitBehavior {
    fn default() -> Self {
        QuitBehavior::Close
    }
}

impl Default for ProviderKind {
    fn default() -> Self {
        ProviderKind::Cli
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: ProviderKind::Cli,
            enabled: false,
            app_name: None,
            app_path: None,
            command: vec![],
            extra: serde_json::json!({}),
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            default_provider: "iterm2".to_string(),
            width_ratio: 0.38,
            follow_window: true,
            auto_redock: true,
            redock_interval_ms: 250,
            quit_behavior: QuitBehavior::Close,
            providers: vec![
                ProviderConfig {
                    id: "iterm2".into(),
                    name: "iTerm2".into(),
                    kind: ProviderKind::Applescript,
                    enabled: true,
                    app_name: Some("iTerm2".into()),
                    app_path: Some("/Applications/iTerm.app".into()),
                    command: vec![],
                    extra: serde_json::json!({}),
                },
                ProviderConfig {
                    id: "wezterm".into(),
                    name: "WezTerm".into(),
                    kind: ProviderKind::Cli,
                    enabled: true,
                    app_name: Some("wezterm-gui".into()),
                    app_path: None,
                    command: vec!["wezterm".into(), "start".into(), "--cwd".into(), "{cwd}".into()],
                    extra: serde_json::json!({}),
                },
                ProviderConfig {
                    id: "alacritty".into(),
                    name: "Alacritty".into(),
                    kind: ProviderKind::Cli,
                    enabled: true,
                    app_name: Some("Alacritty".into()),
                    app_path: None,
                    command: vec!["alacritty".into(), "--working-directory".into(), "{cwd}".into()],
                    extra: serde_json::json!({}),
                },
                ProviderConfig {
                    id: "kitty".into(),
                    name: "kitty".into(),
                    kind: ProviderKind::Cli,
                    enabled: true,
                    app_name: Some("kitty".into()),
                    app_path: None,
                    command: vec!["kitty".into(), "--single-instance".into(), "--directory".into(), "{cwd}".into()],
                    extra: serde_json::json!({}),
                },
                ProviderConfig {
                    id: "terminal".into(),
                    name: "Terminal".into(),
                    kind: ProviderKind::Applescript,
                    enabled: true,
                    app_name: Some("Terminal".into()),
                    app_path: None,
                    command: vec![],
                    extra: serde_json::json!({}),
                },
            ],
        }
    }
}

impl TerminalSettings {
    /// 从 TOML 文件加载；文件不存在或解析失败时返回默认值
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 保存为 TOML 文件
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    /// settings.toml 的路径：app_config_dir/settings.toml
    pub fn path_for(app_config_dir: &Path) -> std::path::PathBuf {
        app_config_dir.join("settings.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_iterm2_as_default() {
        let s = TerminalSettings::default();
        assert_eq!(s.default_provider, "iterm2");
        assert!((s.width_ratio - 0.38).abs() < 1e-9);
        assert!(s.follow_window);
        assert!(s.auto_redock);
        assert_eq!(s.quit_behavior, QuitBehavior::Close);
        assert!(s.providers.iter().any(|p| p.id == "iterm2" && p.enabled));
    }

    #[test]
    fn missing_keys_fall_back_to_default() {
        let toml_str = "defaultProvider = \"wezterm\"\n";
        let s: TerminalSettings = toml::from_str(toml_str).unwrap();
       assert_eq!(s.default_provider, "wezterm");
       assert!((s.width_ratio - 0.38).abs() < 1e-9);
       assert!(s.follow_window);
       assert_eq!(s.quit_behavior, QuitBehavior::Close);
        // 缺省 providers 回退到 Default（5 个内置 provider）
        assert_eq!(s.providers.len(), 5);
        assert!(s.providers.iter().any(|p| p.id == "iterm2"));
    }

    #[test]
    fn round_trip_serialize_deserialize() {
        let original = TerminalSettings::default();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let restored: TerminalSettings = toml::from_str(&toml_str).unwrap();
        assert_eq!(restored.default_provider, original.default_provider);
        assert!((restored.width_ratio - original.width_ratio).abs() < 1e-9);
        assert_eq!(restored.quit_behavior, original.quit_behavior);
        assert_eq!(restored.providers.len(), original.providers.len());
        assert_eq!(restored.providers[0].id, "iterm2");
        assert_eq!(restored.providers[0].kind, ProviderKind::Applescript);
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let path = std::env::temp_dir().join(format!(
            "modou-test-nonexist-{}-settings.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let s = TerminalSettings::load(&path);
        assert_eq!(s.default_provider, "iterm2");
    }

    #[test]
    fn save_then_load_preserves_data() {
        let path = std::env::temp_dir().join(format!(
            "modou-test-save-{}-settings.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let mut original = TerminalSettings::default();
        original.width_ratio = 0.50;
        original.default_provider = "wezterm".into();

        original.save(&path).unwrap();
        let loaded = TerminalSettings::load(&path);

        assert!((loaded.width_ratio - 0.50).abs() < 1e-9);
        assert_eq!(loaded.default_provider, "wezterm");

        let _ = std::fs::remove_file(&path);
    }
}
