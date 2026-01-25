//! 应用状态管理模块
//! 包含全局 AppState 和核心应用逻辑

#![allow(dead_code)]

use crate::config::AppConfig;
use crate::i18n::{I18n, Language};
use crate::project::Project;
use crate::pty::PtyEvent;
use crate::ui::Spinner;
use std::time::Instant;
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

/// 命令执行目标
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CommandTarget {
    #[default]
    DevTerminal, // 在 Dev Server 面板执行
    ShellTerminal, // 在 Interactive Shell 执行
}

/// 应用模式枚举
/// 用于处理不同的交互模式（普通模式、弹窗模式等）
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    CommandPalette,
    AddCommand,
    AddProject,      // 旧的手动输入模式（保留）
    BrowseDirectory, // 新的目录浏览器模式
    EditAlias,
    Help,
    Settings,
    Confirm(String), // 确认对话框，参数为确认消息
}

/// 状态消息（带时间戳，用于自动淡出）
pub struct StatusMessage {
    pub text: String,
    pub created_at: Instant,
}

impl StatusMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            created_at: Instant::now(),
        }
    }

    /// 获取消息年龄（秒）
    pub fn age_secs(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64()
    }

    /// 消息是否应该淡出（超过 3 秒）
    pub fn should_fade(&self) -> bool {
        self.age_secs() > 3.0
    }

    /// 消息是否过期（超过 5 秒）
    pub fn is_expired(&self) -> bool {
        self.age_secs() > 5.0
    }

    /// 获取淡出透明度 (1.0 = 完全可见, 0.0 = 完全透明)
    pub fn opacity(&self) -> f64 {
        let age = self.age_secs();
        if age < 3.0 {
            1.0
        } else if age < 5.0 {
            1.0 - (age - 3.0) / 2.0
        } else {
            0.0
        }
    }
}

/// 目录浏览器状态
#[derive(Debug, Clone)]
pub struct DirectoryBrowser {
    /// 当前浏览的目录
    pub current_dir: std::path::PathBuf,
    /// 目录中的条目列表（只包含文件夹）
    pub entries: Vec<DirEntry>,
    /// 当前选中的索引
    pub selected_idx: usize,
    /// 是否显示隐藏文件
    pub show_hidden: bool,
    /// 是否在驱动器选择模式（仅 Windows）
    pub in_drive_selection: bool,
}

/// 目录条目
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: std::path::PathBuf,
    pub is_dir: bool,
    pub has_package_json: bool,
}

impl DirectoryBrowser {
    /// 创建新的目录浏览器，从用户主目录开始
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
        let mut browser = Self {
            current_dir: home,
            entries: Vec::new(),
            selected_idx: 0,
            show_hidden: false,
            in_drive_selection: false,
        };
        browser.refresh();
        browser
    }

    /// 刷新当前目录的内容
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_idx = 0;

        // Windows 驱动器选择模式
        if self.in_drive_selection {
            #[cfg(windows)]
            {
                self.entries = Self::get_windows_drives();
            }
            return;
        }

        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            let mut entries: Vec<DirEntry> = read_dir
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    // 过滤隐藏文件（除非 show_hidden）
                    if !self.show_hidden && name.starts_with('.') {
                        return false;
                    }
                    // 只显示目录
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                })
                .map(|e| {
                    let path = e.path();
                    let has_package_json = path.join("package.json").exists();
                    DirEntry {
                        name: e.file_name().to_string_lossy().to_string(),
                        path,
                        is_dir: true,
                        has_package_json,
                    }
                })
                .collect();

            // 按名称排序
            entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.entries = entries;
        }
    }

    /// 获取 Windows 驱动器列表
    #[cfg(windows)]
    fn get_windows_drives() -> Vec<DirEntry> {
        let mut drives = Vec::new();
        // 检查 A-Z 驱动器
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = std::path::PathBuf::from(&drive_path);
            if path.exists() {
                drives.push(DirEntry {
                    name: format!("{}: Drive", letter as char),
                    path,
                    is_dir: true,
                    has_package_json: false,
                });
            }
        }
        drives
    }

    /// 检查当前是否在驱动器根目录（Windows）
    #[cfg(windows)]
    fn is_at_drive_root(&self) -> bool {
        // Windows 驱动器根目录形如 "C:\" 或 "C:"
        let path_str = self.current_dir.to_string_lossy();
        path_str.len() <= 3 && path_str.chars().nth(1) == Some(':')
    }

    #[cfg(not(windows))]
    fn is_at_drive_root(&self) -> bool {
        false
    }

    /// 进入选中的目录
    pub fn enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_idx) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.in_drive_selection = false;
                self.refresh();
            }
        }
    }

    /// 返回上级目录
    pub fn go_up(&mut self) {
        // 如果在驱动器选择模式，按返回无效
        if self.in_drive_selection {
            return;
        }

        // Windows: 如果已经在驱动器根目录，进入驱动器选择模式
        #[cfg(windows)]
        if self.is_at_drive_root() {
            self.in_drive_selection = true;
            self.refresh();
            return;
        }

        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh();
        }
    }

    /// 选择下一项
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.entries.len();
        }
    }

    /// 选择上一项
    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            if self.selected_idx == 0 {
                self.selected_idx = self.entries.len() - 1;
            } else {
                self.selected_idx -= 1;
            }
        }
    }

    /// 切换隐藏文件显示
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }

    /// 获取当前选中的条目
    pub fn selected_entry(&self) -> Option<&DirEntry> {
        self.entries.get(self.selected_idx)
    }
}

impl Default for DirectoryBrowser {
    fn default() -> Self {
        Self::new()
    }
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
    /// 命令执行目标（Dev Terminal 或 Shell Terminal）
    pub command_target: CommandTarget,
    /// 设置页面选中索引
    pub settings_idx: usize,
    /// 输入缓冲区（用于各种输入场景）
    pub input_buffer: String,
    /// 状态栏消息（带时间戳）
    pub status_message: Option<StatusMessage>,
    /// 全局 Spinner（用于加载动画）
    pub spinner: Spinner,
    /// 应用启动时间
    pub start_time: Instant,
    /// 当前帧计数（用于动画）
    pub frame_count: u64,
    /// 目录浏览器状态
    pub dir_browser: DirectoryBrowser,
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
            command_target: CommandTarget::default(),
            settings_idx: 0,
            input_buffer: String::new(),
            status_message: None,
            spinner: Spinner::dots(),
            start_time: Instant::now(),
            frame_count: 0,
            dir_browser: DirectoryBrowser::new(),
        }
    }

    /// 进入目录浏览模式
    pub fn enter_browse_mode(&mut self) {
        self.dir_browser = DirectoryBrowser::new();
        self.mode = AppMode::BrowseDirectory;
    }

    /// 增加帧计数（每帧调用）
    pub fn tick(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // 自动清除过期的状态消息
        if let Some(ref msg) = self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }
    }

    /// 获取 spinner 当前帧
    pub fn spinner_frame(&self) -> &'static str {
        self.spinner.frame()
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
        self.status_message = Some(StatusMessage::new(message.to_string()));
    }

    /// 清除状态消息
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// 获取状态消息文本（如果有）
    pub fn status_text(&self) -> Option<&str> {
        self.status_message.as_ref().map(|m| m.text.as_str())
    }

    /// 获取状态消息透明度
    pub fn status_opacity(&self) -> f64 {
        self.status_message
            .as_ref()
            .map(|m| m.opacity())
            .unwrap_or(0.0)
    }

    /// 进入命令面板模式
    pub fn enter_command_palette(&mut self, target: CommandTarget) {
        self.mode = AppMode::CommandPalette;
        self.command_palette_idx = 0;
        self.command_target = target;
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
