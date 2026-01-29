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
    /// PTY 资源清理状态（Windows 专用）
    /// 当停止 Dev 进程后，等待 ConPTY 资源释放
    pub pty_cleanup: Option<PtyCleanupState>,
    /// 待执行的 Dev 命令
    /// 当 PTY 资源正在释放时，缓存用户请求的命令
    pub pending_dev_command: Option<PendingDevCommand>,
    /// PTY 创建锁（ConPTY 竞态保护）
    /// 防止多个 PTY 同时创建导致 0xc0000142 错误
    pub pty_creation_lock: Option<PtyCreationLock>,
    /// 待处理的 Shell 请求
    /// 当 PTY 创建锁被占用时，缓存用户的 Shell 启动请求
    pub pending_shell_request: Option<PendingShellRequest>,
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
            pty_cleanup: None,
            pending_dev_command: None,
            pty_creation_lock: None,
            pending_shell_request: None,
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

    /// 检查是否正在等待 PTY 资源释放
    pub fn is_waiting_for_cleanup(&self) -> bool {
        self.pty_cleanup.is_some()
    }

    /// 检查指定项目是否正在等待资源释放
    pub fn is_project_waiting_cleanup(&self, project_idx: usize) -> bool {
        self.pty_cleanup
            .as_ref()
            .map(|c| c.project_idx == project_idx)
            .unwrap_or(false)
    }

    /// 获取清理状态信息（用于 UI 显示）
    pub fn cleanup_status_text(&self) -> Option<String> {
        self.pty_cleanup.as_ref().map(|c| {
            let elapsed = c.elapsed_ms();
            match self.language() {
                Language::English => format!("Releasing... ({:.1}s)", elapsed as f64 / 1000.0),
                Language::Chinese => format!("释放中... ({:.1}s)", elapsed as f64 / 1000.0),
            }
        })
    }

    /// 轮询检测 PTY 资源是否已释放（Windows 专用）
    /// 返回 true 表示资源已释放，可以执行待处理的命令
    #[cfg(windows)]
    pub fn poll_pty_cleanup(&mut self) -> bool {
        use sysinfo::ProcessesToUpdate;

        let cleanup = match &mut self.pty_cleanup {
            Some(c) => c,
            None => return false,
        };

        cleanup.poll_count += 1;

        // 检查是否超时
        if cleanup.elapsed_ms() > PtyCleanupState::MAX_WAIT_MS {
            log::warn!(
                "PTY cleanup timeout after {}ms, proceeding anyway",
                cleanup.elapsed_ms()
            );
            self.pty_cleanup = None;
            return true;
        }

        // 刷新进程信息
        self.system.refresh_processes(ProcessesToUpdate::All, false);

        // 检查旧进程是否已经不存在
        let old_pid = sysinfo::Pid::from_u32(cleanup.old_pid);
        let process_gone = self.system.process(old_pid).is_none();

        if process_gone {
            log::info!(
                "PTY process {} terminated after {}ms ({} polls)",
                cleanup.old_pid,
                cleanup.elapsed_ms(),
                cleanup.poll_count
            );
            self.pty_cleanup = None;
            return true;
        }

        false
    }

    /// 非 Windows 平台不需要轮询
    #[cfg(not(windows))]
    pub fn poll_pty_cleanup(&mut self) -> bool {
        // 非 Windows 平台直接清除状态
        if self.pty_cleanup.is_some() {
            self.pty_cleanup = None;
            return true;
        }
        false
    }

    // ========== PTY 创建锁相关方法（ConPTY 竞态保护）==========

    /// 检查是否可以创建新的 PTY
    /// 如果有锁且未过期，返回 false
    pub fn can_create_pty(&self) -> bool {
        match &self.pty_creation_lock {
            Some(lock) => lock.is_expired(),
            None => true,
        }
    }

    /// 尝试获取 PTY 创建锁
    /// 返回 true 表示成功获取锁，可以创建 PTY
    /// 返回 false 表示锁被占用，需要等待
    pub fn try_acquire_pty_lock(&mut self, reason: &str) -> bool {
        // 先检查是否已有锁
        if let Some(ref lock) = self.pty_creation_lock {
            if !lock.is_expired() {
                log::debug!(
                    "PTY creation lock held by '{}', elapsed {}ms",
                    lock.reason,
                    lock.elapsed_ms()
                );
                return false;
            }
        }

        // 获取锁
        self.pty_creation_lock = Some(PtyCreationLock::new(reason));
        log::debug!("PTY creation lock acquired: {}", reason);
        true
    }

    /// 释放 PTY 创建锁（实际上是让锁自然过期）
    /// 不直接清除锁，而是让冷却期自然结束
    /// 这样可以确保 ConPTY 有足够时间完成初始化
    pub fn mark_pty_created(&mut self, reason: &str) {
        // 刷新锁的时间戳，开始新的冷却期
        self.pty_creation_lock = Some(PtyCreationLock::new(reason));
        log::debug!("PTY created, cooldown started: {}", reason);
    }

    /// 轮询 PTY 创建锁状态
    /// 返回 true 表示锁已释放，可以执行待处理的请求
    pub fn poll_pty_creation_lock(&mut self) -> bool {
        match &self.pty_creation_lock {
            Some(lock) if lock.is_expired() => {
                log::debug!("PTY creation lock expired after {}ms", lock.elapsed_ms());
                self.pty_creation_lock = None;
                true
            }
            Some(_) => false,
            None => {
                // 没有锁，检查是否有待处理请求
                self.pending_shell_request.is_some()
            }
        }
    }

    /// 检查是否有待处理的 Shell 请求
    pub fn has_pending_shell(&self) -> bool {
        self.pending_shell_request.is_some()
    }

    /// 缓存 Shell 请求
    pub fn queue_shell_request(&mut self, project_idx: usize) {
        self.pending_shell_request = Some(PendingShellRequest { project_idx });
        log::info!("Shell request queued for project {}", project_idx);

        // 更新状态提示
        match self.language() {
            Language::English => self.set_status("Waiting for PTY ready..."),
            Language::Chinese => self.set_status("等待 PTY 就绪..."),
        }
    }

    /// 取出待处理的 Shell 请求
    pub fn take_pending_shell(&mut self) -> Option<PendingShellRequest> {
        self.pending_shell_request.take()
    }
}
