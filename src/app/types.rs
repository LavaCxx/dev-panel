//! 应用类型定义模块
//! 包含各种枚举和简单类型定义

use std::time::Instant;

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

/// 右侧面板布局模式
/// 用于控制 Dev Terminal 和 Shell Terminal 的显示比例
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PanelLayout {
    #[default]
    Split, // 平分 (50% / 50%)
    DevMax,   // Dev Terminal 最大化 (Shell 只显示标题)
    ShellMax, // Shell Terminal 最大化 (Dev 只显示标题)
}

impl PanelLayout {
    /// 切换到下一个布局模式
    pub fn next(&self) -> Self {
        match self {
            PanelLayout::Split => PanelLayout::DevMax,
            PanelLayout::DevMax => PanelLayout::ShellMax,
            PanelLayout::ShellMax => PanelLayout::Split,
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
    AddProject,      // 旧的手动输入模式（保留）
    BrowseDirectory, // 新的目录浏览器模式
    EditAlias,
    Help,
    Settings,
    Confirm(String), // 确认对话框，参数为确认消息
}

/// PTY 资源清理状态（Windows 专用）
/// 用于追踪 ConPTY 资源释放进度
#[derive(Debug, Clone)]
pub struct PtyCleanupState {
    /// 开始清理的时间
    pub started_at: Instant,
    /// 旧进程的 PID（用于检测是否已终止）
    pub old_pid: u32,
    /// 对应的项目索引
    pub project_idx: usize,
    /// 已经轮询的次数
    pub poll_count: u32,
}

impl PtyCleanupState {
    /// 创建新的清理状态
    pub fn new(old_pid: u32, project_idx: usize) -> Self {
        Self {
            started_at: Instant::now(),
            old_pid,
            project_idx,
            poll_count: 0,
        }
    }

    /// 获取已等待的时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// 最大等待时间（毫秒）
    pub const MAX_WAIT_MS: u64 = 3000;
}

/// 待执行的命令
/// 当 PTY 资源正在释放时，缓存用户请求的命令
#[derive(Debug, Clone)]
pub struct PendingDevCommand {
    /// 命令在命令面板中的索引
    pub command_idx: usize,
    /// 项目索引
    pub project_idx: usize,
}

/// PTY 创建锁状态（Windows ConPTY 竞态保护）
/// 用于防止多个 PTY 同时创建时的竞态条件
#[derive(Debug, Clone)]
pub struct PtyCreationLock {
    /// 锁定开始时间
    pub locked_at: Instant,
    /// 锁定原因（用于调试）
    pub reason: String,
}

impl PtyCreationLock {
    /// 创建新的锁
    pub fn new(reason: &str) -> Self {
        Self {
            locked_at: Instant::now(),
            reason: reason.to_string(),
        }
    }

    /// 获取已锁定的时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.locked_at.elapsed().as_millis() as u64
    }

    /// 检查锁是否已过期（冷却期结束）
    /// Windows ConPTY 需要一定时间完成初始化
    pub fn is_expired(&self) -> bool {
        self.elapsed_ms() > Self::COOLDOWN_MS
    }

    /// PTY 创建后的冷却期（毫秒）
    /// 给 ConPTY 足够的时间完成内部初始化
    #[cfg(windows)]
    pub const COOLDOWN_MS: u64 = 150;

    /// 非 Windows 平台不需要冷却期
    #[cfg(not(windows))]
    pub const COOLDOWN_MS: u64 = 0;
}

/// 待处理的 Shell 请求
/// 当 PTY 创建锁被占用时，缓存用户的 Shell 启动请求
#[derive(Debug, Clone)]
pub struct PendingShellRequest {
    /// 项目索引
    pub project_idx: usize,
}
