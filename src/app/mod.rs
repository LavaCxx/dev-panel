//! 应用状态管理模块
//! 包含全局 AppState 和核心应用逻辑

#![allow(dead_code)]

mod dir_browser;
mod scroll;
mod status;
mod types;

pub use dir_browser::*;
pub use scroll::*;
pub use status::*;
pub use types::*;

use crate::config::AppConfig;
use crate::i18n::{I18n, Language};
use crate::project::Project;
use crate::pty::PtyEvent;
use crate::ui::Spinner;
use std::time::Instant;
use tokio::sync::mpsc;

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
    /// 系统信息（用于获取进程 CPU/内存使用）
    pub system: sysinfo::System,
    /// 上次更新资源信息的帧计数
    resource_update_frame: u64,
    /// 右侧面板布局模式
    pub panel_layout: PanelLayout,
    /// 帮助弹窗平滑滚动状态
    pub help_scroll: SmoothScroll,
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
            system: {
                // 初始化 sysinfo 并进行第一次刷新
                // 这样后续的 CPU 使用率计算才能正常工作
                let mut sys = sysinfo::System::new();
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                sys
            },
            resource_update_frame: 0,
            panel_layout: PanelLayout::default(),
            help_scroll: SmoothScroll::new(),
        }
    }

    /// 进入目录浏览模式
    /// 使用配置中保存的上次浏览目录作为起始位置（Windows 会使用盘符根目录）
    pub fn enter_browse_mode(&mut self) {
        let last_dir = self.config.settings.last_browse_dir.as_deref();
        self.dir_browser = DirectoryBrowser::with_initial_dir(last_dir);
        self.mode = AppMode::BrowseDirectory;
    }

    /// 增加帧计数（每帧调用）
    /// 返回是否有动画正在进行（用于决定是否需要持续重绘）
    pub fn tick(&mut self) -> bool {
        self.frame_count = self.frame_count.wrapping_add(1);

        // 自动清除过期的状态消息
        if let Some(ref msg) = self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }

        // 每 30 帧（约每秒）更新一次进程资源信息
        // 这样可以避免频繁刷新系统信息带来的性能开销
        if self.frame_count - self.resource_update_frame >= 30 {
            self.update_resource_usage();
            self.resource_update_frame = self.frame_count;
        }

        // 更新平滑滚动动画并返回是否仍在进行
        self.help_scroll.update()
    }

    /// 更新所有运行中进程的资源使用信息
    fn update_resource_usage(&mut self) {
        use sysinfo::ProcessesToUpdate;

        // 收集所有需要刷新的进程 PID
        let pids: Vec<sysinfo::Pid> = self
            .projects
            .iter()
            .filter_map(|p| p.dev_pty.as_ref())
            .filter_map(|pty| pty.pid)
            .map(sysinfo::Pid::from_u32)
            .collect();

        if pids.is_empty() {
            return;
        }

        // 刷新指定进程的信息（包括子进程）
        // 使用 All 来确保能获取到子进程信息
        self.system.refresh_processes(ProcessesToUpdate::All, true);

        // 更新每个项目的资源使用信息
        for project in &mut self.projects {
            if let Some(ref mut pty) = project.dev_pty {
                pty.update_resource_usage(&self.system);
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

    /// 切换 Windows Shell（仅 Windows 有效）
    #[cfg(windows)]
    pub fn toggle_windows_shell(&mut self) {
        self.config.settings.windows_shell = self.config.settings.windows_shell.toggle();
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

    /// 切换右侧面板布局
    pub fn toggle_panel_layout(&mut self) {
        self.panel_layout = self.panel_layout.next();
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
