//! 应用类型定义模块
//! 包含各种枚举和简单类型定义

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
