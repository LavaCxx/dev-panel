//! 项目扫描模块
//! 用于扫描目录并发现可管理的项目

use std::path::Path;

/// 检查目录是否是一个有效的前端项目
/// 判断标准：存在 package.json 文件
pub fn is_valid_project(path: &Path) -> bool {
    path.is_dir() && path.join("package.json").exists()
}

/// 扫描目录，返回所有有效项目的路径
/// 仅扫描一级子目录
pub fn scan_projects(root: &Path) -> Vec<std::path::PathBuf> {
    let mut projects = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_valid_project(&path) {
                projects.push(path);
            }
        }
    }

    projects
}
