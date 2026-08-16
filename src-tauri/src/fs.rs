use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

use crate::commands::FileNode;

#[allow(dead_code)]
pub struct FileTree {
    pub root: PathBuf,
    pub children: Vec<FileNode>,
}

impl FileTree {
    pub fn new(root: &Path) -> Result<Self, String> {
        if !root.is_dir() {
            return Err("Not a directory".to_string());
        }

        // 只扫描第一层，子目录在前端展开时通过 list_dir 懒加载
        let children = Self::scan_level(root, 0)?;
        Ok(Self {
            root: root.to_path_buf(),
            children,
        })
    }

    /// 扫描单层目录（不递归），用于文件树的懒加载
    pub fn scan_level(path: &Path, depth: usize) -> Result<Vec<FileNode>, String> {
        let mut entries = Vec::new();

        let mut read_dir: Vec<_> = std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| !Self::should_ignore(&e.path()))
            .collect();

        read_dir.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in read_dir {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();

            entries.push(FileNode {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir,
                children: Vec::new(),
                depth,
            });
        }

        Ok(entries)
    }

    fn should_ignore(path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        matches!(
            name,
            ".git" | ".svn" | ".hg" | "node_modules" | "target" | "build" | "dist"
                | "__pycache__" | ".venv" | "venv" | ".idea" | ".vscode" | ".DS_Store"
                | ".next" | ".nuxt" | ".cache" | "Pods" | "DerivedData" | ".dart_tool"
                | ".gradle" | "coverage"
        )
    }

    pub fn to_nodes(&self) -> Vec<FileNode> {
        self.children.clone()
    }
}

pub struct GitStatus {
    repo: git2::Repository,
}

impl GitStatus {
    pub fn new(path: &Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::discover(path)?;
        Ok(Self { repo })
    }

    pub fn branch(&self) -> String {
        self.repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn modified_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(statuses) = self.repo.statuses(None) {
            for entry in statuses.iter() {
                if entry.status().contains(git2::Status::WT_MODIFIED) {
                    if let Some(path) = entry.path() {
                        files.push(path.to_string());
                    }
                }
            }
        }
        files
    }

    pub fn added_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(statuses) = self.repo.statuses(None) {
            for entry in statuses.iter() {
                if entry.status().contains(git2::Status::WT_NEW) {
                    if let Some(path) = entry.path() {
                        files.push(path.to_string());
                    }
                }
            }
        }
        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 在系统临时目录下创建一个唯一的测试目录
    fn make_fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "modou-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_level_returns_single_level_sorted_dirs_first() {
        let root = make_fixture("scan");
        fs::create_dir_all(root.join("zdir")).unwrap();
        fs::create_dir_all(root.join("adir/subdir")).unwrap();
        fs::write(root.join("b.txt"), "hello").unwrap();
        fs::write(root.join("adir/inner.txt"), "x").unwrap();

        let nodes = FileTree::scan_level(&root, 0).unwrap();
        let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();

        // 目录在前、按名称排序
        assert_eq!(names, vec!["adir", "zdir", "b.txt"]);
        assert!(nodes[0].is_dir && nodes[1].is_dir && !nodes[2].is_dir);
        // 单层扫描：目录的 children 必须为空（懒加载）
        assert!(nodes.iter().all(|n| n.children.is_empty()));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_level_ignores_heavy_dirs() {
        let root = make_fixture("ignore");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join(".venv")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();

        let nodes = FileTree::scan_level(&root, 0).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "src");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_level_rejects_file_path() {
        let root = make_fixture("notdir");
        let file = root.join("a.txt");
        fs::write(&file, "x").unwrap();

        assert!(FileTree::new(&file).is_err());

        let _ = fs::remove_dir_all(&root);
    }
}
