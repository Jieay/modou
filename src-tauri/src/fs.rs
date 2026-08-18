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

/// 文件级变更（文件树徽章用），status: M/A/D
#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub status: String,
}

/// 行级变更（编辑器 gutter 用），行号 1-based，区间为闭区间
#[derive(Debug, Clone, Default, Serialize)]
pub struct LineDiff {
    pub added: Vec<(usize, usize)>,
    pub modified: Vec<(usize, usize)>,
    pub deleted: Vec<usize>,
}

impl GitStatus {
    pub fn new(path: &Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::discover(path)?;
        Ok(Self { repo })
    }

    /// 工作区 + 暂存区的全部变更（含未跟踪文件），返回绝对路径
    pub fn changes(&self) -> Vec<FileChange> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let mut out = Vec::new();
        let Some(workdir) = self.repo.workdir().map(|w| w.to_path_buf()) else {
            return out;
        };
        if let Ok(statuses) = self.repo.statuses(Some(&mut opts)) {
            for entry in statuses.iter() {
                let s = entry.status();
                let Some(p) = entry.path() else { continue };
                let status = if s.intersects(git2::Status::WT_NEW | git2::Status::INDEX_NEW) {
                    "A"
                } else if s.intersects(git2::Status::WT_DELETED | git2::Status::INDEX_DELETED) {
                    "D"
                } else if s.intersects(
                    git2::Status::WT_MODIFIED
                        | git2::Status::INDEX_MODIFIED
                        | git2::Status::WT_TYPECHANGE
                        | git2::Status::INDEX_TYPECHANGE
                        | git2::Status::WT_RENAMED
                        | git2::Status::INDEX_RENAMED,
                ) {
                    "M"
                } else {
                    continue;
                };
                out.push(FileChange {
                    path: workdir.join(p).to_string_lossy().to_string(),
                    status: status.to_string(),
                });
            }
        }
        out
    }

    /// 当前编辑内容与 HEAD 版本的行级差异
    pub fn diff_lines(&self, abs_path: &Path, content: &str) -> LineDiff {
        let mut result = LineDiff::default();
        let total_lines = content.lines().count().max(1);

        let Some(workdir) = self.repo.workdir() else { return result };
        // macOS 上 /var 是 /private/var 的软链，git2 可能返回真实路径，统一规范化后再比较；
        // 文件可能尚未落盘（新文件），canonicalize 失败时退化为规范化父目录 + 文件名
        let workdir = workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_path_buf());
        let abs_canon = abs_path.canonicalize().unwrap_or_else(|_| {
            abs_path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .and_then(|p| abs_path.file_name().map(|n| p.join(n)))
                .unwrap_or_else(|| abs_path.to_path_buf())
        });
        let Ok(rel) = abs_canon.strip_prefix(&workdir) else { return result };

        // HEAD 中没有该文件 → 新文件，全部行视为新增
        let blob = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_tree().ok())
            .and_then(|t| t.get_path(rel).ok())
            .and_then(|e| self.repo.find_blob(e.id()).ok());
        let Some(blob) = blob else {
            result.added.push((1, total_lines));
            return result;
        };

        let old_text = String::from_utf8_lossy(blob.content()).into_owned();
        let old_lines: Vec<&str> = old_text.lines().collect();
        let new_lines: Vec<&str> = content.lines().collect();
        myers_line_diff(&old_lines, &new_lines, &mut result);
        result
    }
}

// ====================================================================
// Myers 行级 diff（O(ND)，避免向用户仓库 .git/objects 写入临时 blob）
// ====================================================================

#[derive(Clone, Copy, PartialEq)]
enum EditOp {
    Keep,
    Insert,
    Delete,
}

fn push_merge(ranges: &mut Vec<(usize, usize)>, start: usize, end: usize) {
    if let Some(last) = ranges.last_mut() {
        if start <= last.1 + 1 {
            last.1 = last.1.max(end);
            return;
        }
    }
    ranges.push((start, end));
}

/// 计算 old -> new 的行级差异，结果写入 out（行号 1-based，闭区间）
fn myers_line_diff(old: &[&str], new: &[&str], out: &mut LineDiff) {
    let n = old.len();
    let m = new.len();
    // 防御：超大文件跳过（避免 O((N+M)D) 内存膨胀）
    if n + m > 100_000 {
        return;
    }
    if n == 0 && m == 0 {
        return;
    }

    let max = n + m;
    let offset = max as isize;
    let width = 2 * max + 1;
    let mut v = vec![0isize; width];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found_d = None;

    'outer: for d in 0..=(max as isize) {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let ki = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[ki - 1] < v[ki + 1]) {
                v[ki + 1]
            } else {
                v[ki - 1] + 1
            };
            let mut y = x - k;
            while (x as usize) < n && (y as usize) < m && old[x as usize] == new[y as usize] {
                x += 1;
                y += 1;
            }
            v[ki] = x;
            if x as usize >= n && y as usize >= m {
                found_d = Some(d);
                break 'outer;
            }
            k += 2;
        }
    }

    let Some(d_final) = found_d else { return };

    // 回溯生成操作序列（先逆序收集）
    let mut ops: Vec<EditOp> = Vec::new();
    let mut x = n as isize;
    let mut y = m as isize;
    for d in (1..=d_final).rev() {
        // trace[d] 保存的是计算第 d 行之前的 V（即 V_{d-1}）
        let v_prev = &trace[d as usize];
        let k = x - y;
        let ki = (k + offset) as usize;
        let prev_k = if k == -d || (k != d && v_prev[ki - 1] < v_prev[ki + 1]) {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v_prev[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;
        while x > prev_x && y > prev_y {
            ops.push(EditOp::Keep);
            x -= 1;
            y -= 1;
        }
        if x == prev_x {
            ops.push(EditOp::Insert);
            y -= 1;
        } else {
            ops.push(EditOp::Delete);
            x -= 1;
        }
    }
    // d=0 的纯对角线收尾
    while x > 0 && y > 0 {
        ops.push(EditOp::Keep);
        x -= 1;
        y -= 1;
    }
    while x > 0 {
        ops.push(EditOp::Delete);
        x -= 1;
    }
    while y > 0 {
        ops.push(EditOp::Insert);
        y -= 1;
    }
    ops.reverse();

    // 操作序列 → 行区间（new 文件 1-based）
    let mut new_line = 1usize;
    let mut i = 0;
    while i < ops.len() {
        if ops[i] == EditOp::Keep {
            new_line += 1;
            i += 1;
            continue;
        }
        // 收集一段连续的非 Keep 操作（Delete/Insert 任意顺序混合）
        let seg_start = new_line;
        let mut dels = 0usize;
        let mut ins = 0usize;
        while i < ops.len() && ops[i] != EditOp::Keep {
            if ops[i] == EditOp::Delete {
                dels += 1;
            } else {
                ins += 1;
                new_line += 1;
            }
            i += 1;
        }
        if dels > 0 && ins > 0 {
            push_merge(&mut out.modified, seg_start, new_line - 1);
        } else if ins > 0 {
            push_merge(&mut out.added, seg_start, new_line - 1);
        } else {
            // 纯删除：标记在删除点（seg_start-1 为删除点之前的行，0 表示文件开头）
            out.deleted.push(seg_start.saturating_sub(1).max(1));
        }
    }
}

impl GitStatus {
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

    /// 初始化一个带一次提交的临时仓库，返回 (仓库目录, GitStatus)
    fn make_repo(name: &str, files: &[(&str, &str)]) -> (PathBuf, GitStatus) {
        let dir = make_fixture(name);
        let repo = git2::Repository::init(&dir).unwrap();
        for (path, content) in files {
            fs::write(dir.join(path), content).unwrap();
        }
        let mut index = repo.index().unwrap();
        for (path, _) in files {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        let status = GitStatus::new(&dir).unwrap();
        (dir, status)
    }

    #[test]
    fn changes_reports_modified_and_untracked() {
        let (dir, status) = make_repo("changes", &[("a.txt", "hello\n")]);
        fs::write(dir.join("a.txt"), "changed\n").unwrap();
        fs::write(dir.join("new.txt"), "new\n").unwrap();

        let changes = status.changes();
        let get = |name: &str| changes.iter().find(|c| c.path.ends_with(name)).map(|c| c.status.as_str());
        assert_eq!(get("a.txt"), Some("M"));
        assert_eq!(get("new.txt"), Some("A"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_lines_modified_added_deleted() {
        let (dir, status) = make_repo("difflines", &[("f.txt", "a\nb\nc\nd\n")]);
        let content = "a\nB\nc\nx\ny\n"; // b→B 修改，末尾 d 改为 x y
        let d = status.diff_lines(&dir.join("f.txt"), content);
        // b→B 标记为修改第 2 行；d→x,y 为修改 4-5 行
        assert_eq!(d.modified, vec![(2, 2), (4, 5)]);
        assert!(d.deleted.is_empty(), "不应有纯删除标记");
        assert!(d.added.is_empty(), "不应有纯新增区间，实际 {:?}", d.added);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_lines_pure_add_and_delete() {
        let (dir, status) = make_repo("difflines2", &[("f.txt", "a\nc\n")]);
        // 中间插入一行
        let d = status.diff_lines(&dir.join("f.txt"), "a\nb\nc\n");
        assert_eq!(d.added, vec![(2, 2)]);
        assert!(d.modified.is_empty());
        // 删除一行
        let d2 = status.diff_lines(&dir.join("f.txt"), "a\n");
        assert_eq!(d2.deleted, vec![1], "删除末尾行应标记在第 1 行");
        assert!(d2.added.is_empty() && d2.modified.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_lines_new_file_all_added() {
        let (dir, status) = make_repo("difflines3", &[("f.txt", "a\n")]);
        let d = status.diff_lines(&dir.join("brand_new.txt"), "x\ny\nz\n");
        assert_eq!(d.added, vec![(1, 3)]);

        let _ = fs::remove_dir_all(&dir);
    }
}
