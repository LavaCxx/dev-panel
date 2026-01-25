//! 布局管理模块
//! 负责主界面的布局划分

use crate::app::{AppMode, AppState, FocusArea};
use crate::ui::{
    draw_command_palette, draw_input_popup, draw_settings_popup, draw_sidebar, draw_terminal_panel,
    Theme,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// 绘制主界面
pub fn draw_ui(frame: &mut Frame, state: &AppState, theme: &Theme) {
    // 主布局：顶部标题 + 中间内容 + 底部状态栏
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 标题栏
            Constraint::Min(1),    // 主内容区
            Constraint::Length(1), // 状态栏
        ])
        .split(frame.area());

    // 绘制标题栏
    draw_title_bar(frame, main_chunks[0], state, theme);

    // 内容区布局：左侧边栏 + 右侧工作区
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28), // 侧边栏宽度
            Constraint::Min(1),     // 工作区
        ])
        .split(main_chunks[1]);

    // 绘制侧边栏
    draw_sidebar(frame, content_chunks[0], state, theme);

    // 右侧工作区：上方 Dev Terminal + 下方 Shell Terminal
    let work_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Dev Terminal
            Constraint::Percentage(50), // Shell Terminal
        ])
        .split(content_chunks[1]);

    let i18n = state.i18n();
    
    // Dev Terminal 标题（如果暂停则显示状态，如果滚动则显示提示）
    let (dev_title, dev_scroll_offset) = if let Some(project) = state.active_project() {
        let scroll = project.dev_scroll_offset;
        let title = if let Some(ref pty) = project.dev_pty {
            if pty.suspended {
                format!("{} [{}]", i18n.dev_server(), i18n.paused())
            } else if scroll > 0 {
                format!("{} [↑{}]", i18n.dev_server(), scroll)
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
    
    // 绘制 Dev Terminal
    draw_terminal_panel(
        frame,
        work_chunks[0],
        &dev_title,
        state.active_project().and_then(|p| p.dev_pty.as_ref()),
        state.focus == FocusArea::DevTerminal,
        dev_scroll_offset,
        &i18n,
        theme,
    );

    // 绘制 Shell Terminal（Shell 不支持滚动）
    draw_terminal_panel(
        frame,
        work_chunks[1],
        i18n.interactive_shell(),
        state.active_project().and_then(|p| p.shell_pty.as_ref()),
        state.focus == FocusArea::ShellTerminal,
        0, // Shell 不滚动
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

/// 绘制标题栏
fn draw_title_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let i18n = state.i18n();
    let title = Paragraph::new(i18n.app_title())
        .style(Style::default().fg(theme.title).bold())
        .bg(theme.status_bg);
    frame.render_widget(title, area);
}

/// 绘制状态栏
fn draw_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let i18n = state.i18n();
    let status_text = if let Some(ref msg) = state.status_message {
        msg.clone()
    } else {
        // 根据焦点区域显示不同的快捷键提示
        match state.focus {
            FocusArea::Sidebar | FocusArea::DevTerminal => {
                i18n.status_hint_sidebar().to_string()
            }
            FocusArea::ShellTerminal => {
                i18n.status_hint_shell().to_string()
            }
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(theme.status_fg))
        .bg(theme.status_bg);
    frame.render_widget(status, area);
}

/// 绘制确认对话框
fn draw_confirm_popup(frame: &mut Frame, state: &AppState, message: &str, theme: &Theme) {
    let i18n = state.i18n();
    let area = centered_rect(40, 7, frame.area());

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.confirm())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!("{}\n\n{}", message, i18n.yes_no());
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme.fg))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, inner);
}

/// 帮助弹窗
pub fn draw_help_popup(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let i18n = state.i18n();
    let area = centered_rect(50, 60, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.help())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.info))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = match state.language() {
        crate::i18n::Language::English => r#"
  PROJECT NAVIGATION
  ──────────────────
  1-9              Quick switch to project
  Tab / Shift+Tab  Switch between projects
  j / k / ↑ / ↓    Navigate project list
  Enter            Enter Interactive Shell

  DEV SERVER
  ──────────
  r                Run command (opens palette)
  s                Stop dev server
  x                Send interrupt (Ctrl+C)
  p                Pause/Resume (freeze process)

  INTERACTIVE SHELL
  ─────────────────
  All keys         Sent to shell directly
  Esc              Return to sidebar

  PROJECT MANAGEMENT
  ──────────────────
  a                Add new project
  e                Edit project alias
  c                Add custom command
  d                Delete project

  GENERAL
  ───────
  ,                Open settings
  q / Ctrl+C       Quit application
  ?                Toggle this help

  Press Esc or ? to close
"#,
        crate::i18n::Language::Chinese => r#"
  项目导航
  ────────
  1-9              快速切换项目
  Tab / Shift+Tab  切换项目
  j / k / ↑ / ↓    上下导航
  Enter            进入交互终端

  开发服务
  ────────
  r                运行命令
  s                停止服务
  x                发送中断 (Ctrl+C)
  p                暂停/恢复 (冻结进程)

  交互终端
  ────────
  所有按键         直接发送给终端
  Esc              返回侧边栏

  项目管理
  ────────
  a                添加项目
  e                编辑别名
  c                添加自定义命令
  d                删除项目

  通用
  ────
  ,                打开设置
  q / Ctrl+C       退出程序
  ?                帮助

  按 Esc 或 ? 关闭
"#,
    };

    let paragraph = Paragraph::new(help_text).style(Style::default().fg(theme.fg));

    frame.render_widget(paragraph, inner);
}

/// 计算居中矩形
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
