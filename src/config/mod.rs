//! 配置管理模块

#![allow(dead_code)]

mod persistence;

pub use persistence::*;

use crate::i18n::Language;
use crate::project::ProjectConfig;
use serde::{Deserialize, Serialize};

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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "catppuccin-mocha".to_string(),
            default_runner: "pnpm".to_string(),
            language: Language::default(),
        }
    }
}

/// 应用配置（持久化到 devpanel.json）
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
