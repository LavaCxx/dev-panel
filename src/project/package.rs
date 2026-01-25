//! package.json 解析模块

#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// package.json 结构（仅解析需要的字段）
#[derive(Debug, Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub scripts: Option<HashMap<String, String>>,
}

/// 解析 package.json 文件
pub fn parse_package_json(path: &Path) -> anyhow::Result<PackageJson> {
    let content = std::fs::read_to_string(path)?;
    let package: PackageJson = serde_json::from_str(&content)?;
    Ok(package)
}

/// 检测项目使用的包管理器
/// 通过检查 lock 文件来判断
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl PackageManager {
    /// 获取包管理器的执行命令
    pub fn command(&self) -> &'static str {
        match self {
            PackageManager::Npm => {
                #[cfg(windows)]
                {
                    "npm.cmd"
                }
                #[cfg(unix)]
                {
                    "npm"
                }
            }
            PackageManager::Yarn => "yarn",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Bun => "bun",
        }
    }

    /// 获取运行脚本的命令前缀
    pub fn run_prefix(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm run",
            PackageManager::Yarn => "yarn",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Bun => "bun run",
        }
    }
}

/// 检测项目目录使用的包管理器
pub fn detect_package_manager(project_path: &Path) -> PackageManager {
    // 优先级：pnpm > yarn > bun > npm
    if project_path.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if project_path.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else if project_path.join("bun.lockb").exists() {
        PackageManager::Bun
    } else {
        PackageManager::Npm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_command() {
        assert_eq!(PackageManager::Pnpm.command(), "pnpm");
        assert_eq!(PackageManager::Yarn.command(), "yarn");
    }
}
