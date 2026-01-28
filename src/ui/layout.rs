//! 布局管理模块
//! 负责主界面的布局划分

use crate::app::{AppMode, AppState, FocusArea, PanelLayout};
use crate::ui::{
    calculate_status_bar_height, draw_command_palette, draw_confirm_popup, draw_dir_browser,
    draw_help_popup, draw_input_popup, draw_settings_popup, draw_sidebar, draw_status_bar,
    draw_terminal_panel, draw_title_bar, Theme,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// 绘制主界面
pub fn draw_ui(frame: &mut Frame, state: &mut AppState, theme: &Theme) {
    let screen_width = frame.area().width as usize;

    // 计算状态栏需要的高度（根据内容和屏幕宽度）
    let status_height = calculate_status_bar_height(state, screen_width);

    // 主布局：顶部标题 + 中间内容 + 底部状态栏
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),             // 标题栏
            Constraint::Min(1),                // 主内容区
            Constraint::Length(status_height), // 状态栏（动态高度）
        ])
        .split(frame.area());

    // 绘制标题栏
    draw_title_bar(frame, main_chunks[0], state, theme);

    // 内容区布局：左侧边栏 + 右侧工作区
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38), // 侧边栏宽度（增加以显示 CPU/内存信息）
            Constraint::Min(1),     // 工作区
        ])
        .split(main_chunks[1]);

    // 绘制侧边栏
    draw_sidebar(frame, content_chunks[0], state, theme);

    // 右侧工作区：上方 Dev Terminal + 下方 Shell Terminal
    // 根据面板布局模式调整高度比例
    let work_constraints = match state.panel_layout {
        PanelLayout::Split => [Constraint::Percentage(50), Constraint::Percentage(50)],
        PanelLayout::DevMax => [Constraint::Min(3), Constraint::Length(3)], // Shell 只显示标题栏
        PanelLayout::ShellMax => [Constraint::Length(3), Constraint::Min(3)], // Dev 只显示标题栏
    };
    let work_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(work_constraints)
        .split(content_chunks[1]);

    let i18n = state.i18n();

    // Dev Terminal 标题（显示状态：暂停/资源释放中）
    let (dev_title, dev_scroll_offset) = if let Some(project) = state.active_project() {
        let scroll = project.dev_scroll_offset;
        let title = if state.is_project_waiting_cleanup(state.active_project_idx) {
            // 正在等待 PTY 资源释放
            if let Some(status) = state.cleanup_status_text() {
                format!("{} [{}]", i18n.dev_server(), status)
            } else {
                i18n.dev_server().to_string()
            }
        } else if let Some(ref pty) = project.dev_pty {
            if pty.suspended {
                format!("{} [{}]", i18n.dev_server(), i18n.paused())
            } else {
                i18n.dev_server().to_string()
            }
        } else {
            i18n.dev_server().to_string()
        };
        (title, scroll)
    } else {
        (i18n.dev_server().to_string(), 0)
    };

    // 绘制 Dev Terminal（只读，不显示光标）
    draw_terminal_panel(
        frame,
        work_chunks[0],
        &dev_title,
        state.active_project().and_then(|p| p.dev_pty.as_ref()),
        state.focus == FocusArea::DevTerminal,
        false, // Dev Terminal 是只读的，不显示光标
        dev_scroll_offset,
        &i18n,
        theme,
    );

    // Shell Terminal 滚动偏移量
    let shell_scroll_offset = state
        .active_project()
        .map(|p| p.shell_scroll_offset)
        .unwrap_or(0);

    // 绘制 Shell Terminal（交互式，聚焦时显示光标）
    draw_terminal_panel(
        frame,
        work_chunks[1],
        i18n.interactive_shell(),
        state.active_project().and_then(|p| p.shell_pty.as_ref()),
        state.focus == FocusArea::ShellTerminal,
        true, // Shell Terminal 是交互式的，聚焦时显示光标
        shell_scroll_offset,
        &i18n,
        theme,
    );

    // 绘制状态栏
    draw_status_bar(frame, main_chunks[2], state, theme);

    // 根据模式绘制弹窗
    let i18n = state.i18n();
    match &state.mode {
        AppMode::CommandPalette => {
            draw_command_palette(frame, state, theme);
        }
        AppMode::AddProject => {
            draw_input_popup(
                frame,
                i18n.add_project(),
                i18n.enter_project_path(),
                &state.input_buffer,
                theme,
            );
        }
        AppMode::BrowseDirectory => {
            draw_dir_browser(frame, state, theme);
        }
        AppMode::AddCommand => {
            draw_input_popup(
                frame,
                i18n.add_command(),
                i18n.command_format_hint(),
                &state.input_buffer,
                theme,
            );
        }
        AppMode::EditAlias => {
            draw_input_popup(
                frame,
                i18n.edit_alias(),
                i18n.alias_hint(),
                &state.input_buffer,
                theme,
            );
        }
        AppMode::Help => {
            draw_help_popup(frame, state, theme);
        }
        AppMode::Settings => {
            draw_settings_popup(frame, state, theme);
        }
        AppMode::Confirm(msg) => {
            draw_confirm_popup(frame, state, msg, theme);
        }
        AppMode::Normal => {}
    }
}

/// 计算居中矩形（使用百分比）
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
