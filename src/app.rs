//! 应用状态管理模块
//! 包含全局 AppState 和核心应用逻辑

#![allow(dead_code)]

use crate::config::AppConfig;
use crate::i18n::{I18n, Language};
use crate::project::Project;
use crate::pty::PtyEvent;
use tokio::sync::mpsc;

/// 焦点区域枚举
/// 用于追踪当前用户焦点所在的 UI 区域
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FocusArea {
    #[default]
    Sidebar,
    DevTerminal,
    ShellTerminal,
}

impl FocusArea {
    /// 切换到下一个焦点区域
    pub fn next(&self) -> Self {
        match self {
            FocusArea::Sidebar => FocusArea::DevTerminal,
            FocusArea::DevTerminal => FocusArea::ShellTerminal,
            FocusArea::ShellTerminal => FocusArea::Sidebar,
        }
    }

    /// 切换到上一个焦点区域
    pub fn prev(&self) -> Self {
        match self {
            FocusArea::Sidebar => FocusArea::ShellTerminal,
            FocusArea::DevTerminal => FocusArea::Sidebar,
            FocusArea::ShellTerminal => FocusArea::DevTerminal,
        }
    }
}

/// 应用模式枚举
/// 用于处理不同的交互模式（普通模式、弹窗模式等）
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    CommandPalette,
    AddCommand,
    AddProject,
    EditAlias,
    Help,
    Settings,
    Confirm(String), // 确认对话框，参数为确认消息
}

/// 全局应用状态
pub struct AppState {
    /// 项目列表
    pub projects: Vec<Project>,
    /// 当前激活的项目索引
    pub active_project_idx: usize,
    /// 当前焦点区域
    pub focus: FocusArea,
    /// 当前应用模式
    pub mode: AppMode,
    /// 应用配置
    pub config: AppConfig,
    /// 是否应该退出
    pub should_quit: bool,
    /// PTY 事件接收器
    pub pty_rx: mpsc::UnboundedReceiver<PtyEvent>,
    /// PTY 事件发送器（用于克隆给 PTY 任务）
    pub pty_tx: mpsc::UnboundedSender<PtyEvent>,
    /// 命令面板选中索引
    pub command_palette_idx: usize,
    /// 设置页面选中索引
    pub settings_idx: usize,
    /// 输入缓冲区（用于各种输入场景）
    pub input_buffer: String,
    /// 状态栏消息
    pub status_message: Option<String>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(config: AppConfig) -> Self {
        let (pty_tx, pty_rx) = mpsc::unbounded_channel();

        Self {
            projects: Vec::new(),
            active_project_idx: 0,
            focus: FocusArea::default(),
            mode: AppMode::default(),
            config,
            should_quit: false,
            pty_rx,
            pty_tx,
            command_palette_idx: 0,
            settings_idx: 0,
            input_buffer: String::new(),
            status_message: None,
        }
    }

    /// 获取当前语言
    pub fn language(&self) -> Language {
        self.config.settings.language
    }

    /// 获取国际化实例
    pub fn i18n(&self) -> I18n {
        I18n::new(self.language())
    }

    /// 切换语言
    pub fn toggle_language(&mut self) {
        self.config.settings.language = self.config.settings.language.toggle();
    }

    /// 获取当前激活的项目（如果有）
    pub fn active_project(&self) -> Option<&Project> {
        self.projects.get(self.active_project_idx)
    }

    /// 获取当前激活的项目的可变引用
    pub fn active_project_mut(&mut self) -> Option<&mut Project> {
        self.projects.get_mut(self.active_project_idx)
    }

    /// 选择下一个项目
    pub fn select_next_project(&mut self) {
        if !self.projects.is_empty() {
            self.active_project_idx = (self.active_project_idx + 1) % self.projects.len();
        }
    }

    /// 选择上一个项目
    pub fn select_prev_project(&mut self) {
        if !self.projects.is_empty() {
            if self.active_project_idx == 0 {
                self.active_project_idx = self.projects.len() - 1;
            } else {
                self.active_project_idx -= 1;
            }
        }
    }

    /// 切换焦点到下一个区域
    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    /// 切换焦点到上一个区域
    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
    }

    /// 添加新项目
    pub fn add_project(&mut self, project: Project) {
        self.projects.push(project);
    }

    /// 移除项目
    pub fn remove_project(&mut self, idx: usize) {
        if idx < self.projects.len() {
            self.projects.remove(idx);
            // 调整当前选中索引
            if self.active_project_idx >= self.projects.len() && !self.projects.is_empty() {
                self.active_project_idx = self.projects.len() - 1;
            }
        }
    }

    /// 设置状态消息
    pub fn set_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
    }

    /// 清除状态消息
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// 进入命令面板模式
    pub fn enter_command_palette(&mut self) {
        self.mode = AppMode::CommandPalette;
        self.command_palette_idx = 0;
    }

    /// 退出当前模式，返回普通模式
    pub fn exit_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
    }

    /// 命令面板选择下一项
    pub fn command_palette_next(&mut self) {
        if let Some(project) = self.active_project() {
            let commands = project.get_all_commands();
            if !commands.is_empty() {
                self.command_palette_idx = (self.command_palette_idx + 1) % commands.len();
            }
        }
    }

    /// 命令面板选择上一项
    pub fn command_palette_prev(&mut self) {
        if let Some(project) = self.active_project() {
            let commands = project.get_all_commands();
            if !commands.is_empty() {
                if self.command_palette_idx == 0 {
                    self.command_palette_idx = commands.len() - 1;
                } else {
                    self.command_palette_idx -= 1;
                }
            }
        }
    }
}
