//! 配置管理模块

#![allow(dead_code)]

mod persistence;

pub use persistence::*;

use crate::i18n::Language;
use crate::project::ProjectConfig;
use serde::{Deserialize, Serialize};

/// Windows Shell 选项
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum WindowsShell {
    #[default]
    PowerShell,
    Cmd,
}

impl WindowsShell {
    /// 切换到另一个 Shell
    pub fn toggle(&self) -> Self {
        match self {
            WindowsShell::PowerShell => WindowsShell::Cmd,
            WindowsShell::Cmd => WindowsShell::PowerShell,
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            WindowsShell::PowerShell => "PowerShell",
            WindowsShell::Cmd => "CMD",
        }
    }
}

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 主题名称
    pub theme: String,
    /// 默认包管理器
    pub default_runner: String,
    /// 界面语言
    #[serde(default)]
    pub language: Language,
    /// Windows Shell 选择（仅 Windows 有效）
    #[serde(default)]
    pub windows_shell: WindowsShell,
    /// 是否已显示过首次启动设置引导
    #[serde(default)]
    pub first_run_shown: bool,
    /// 最后一次浏览的目录（用于记住 Windows 盘符）
    #[serde(default)]
    pub last_browse_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "catppuccin-mocha".to_string(),
            default_runner: "pnpm".to_string(),
            language: Language::default(),
            windows_shell: WindowsShell::default(),
            first_run_shown: false,
            last_browse_dir: None,
        }
    }
}

/// 应用配置（持久化到 ~/.devpanel/config.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// 项目配置列表
    pub projects: Vec<ProjectConfig>,
    /// 应用设置
    #[serde(default)]
    pub settings: AppSettings,
}

impl AppConfig {
    /// 创建新的空配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加项目配置
    pub fn add_project(&mut self, config: ProjectConfig) {
        // 避免重复添加
        if !self.projects.iter().any(|p| p.path == config.path) {
            self.projects.push(config);
        }
    }

    /// 移除项目配置
    pub fn remove_project(&mut self, path: &str) {
        self.projects.retain(|p| p.path != path);
    }

    /// 更新项目配置
    pub fn update_project(&mut self, config: ProjectConfig) {
        if let Some(existing) = self.projects.iter_mut().find(|p| p.path == config.path) {
            *existing = config;
        } else {
            self.projects.push(config);
        }
    }
}
