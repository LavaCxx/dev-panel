//! 布局管理模块
//! 负责主界面的布局划分

use crate::app::{AppMode, AppState, FocusArea};
use crate::ui::{
    draw_command_palette, draw_dir_browser, draw_input_popup, draw_settings_popup, draw_sidebar,
    draw_terminal_panel, Theme,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
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
            Constraint::Length(38), // 侧边栏宽度（增加以显示 CPU/内存信息）
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

    // 绘制 Shell Terminal（交互式，聚焦时显示光标）
    draw_terminal_panel(
        frame,
        work_chunks[1],
        i18n.interactive_shell(),
        state.active_project().and_then(|p| p.shell_pty.as_ref()),
        state.focus == FocusArea::ShellTerminal,
        true, // Shell Terminal 是交互式的，聚焦时显示光标
        0,    // Shell 不滚动
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

    // 检查是否有状态消息
    let (status_text, is_message) = if let Some(ref msg) = state.status_message {
        (msg.text.clone(), true)
    } else {
        // 根据焦点区域显示不同的快捷键提示
        let hint = match state.focus {
            FocusArea::Sidebar | FocusArea::DevTerminal => i18n.status_hint_sidebar().to_string(),
            FocusArea::ShellTerminal => i18n.status_hint_shell().to_string(),
        };
        (hint, false)
    };

    // 计算状态消息的颜色（支持淡出效果）
    let fg_color = if is_message {
        let opacity = state.status_opacity();
        if opacity > 0.5 {
            theme.success // 高亮显示
        } else {
            theme.border // 淡出时变暗
        }
    } else {
        theme.status_fg
    };

    // 添加图标
    let display_text = if is_message {
        format!(" ✓ {}", status_text)
    } else {
        status_text
    };

    let status = Paragraph::new(display_text)
        .style(Style::default().fg(fg_color))
        .bg(theme.status_bg);
    frame.render_widget(status, area);
}

/// 绘制确认对话框
fn draw_confirm_popup(frame: &mut Frame, state: &AppState, message: &str, theme: &Theme) {
    let i18n = state.i18n();
    // 使用固定尺寸而非百分比，确保弹窗大小足够显示内容
    let area = centered_fixed_rect(40, 7, frame.area());

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
    let area = centered_rect(55, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.help())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.info))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 使用 Span 构建带样式的帮助文本
    let key_style = Style::default().fg(theme.info).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.fg);
    let section_style = Style::default()
        .fg(theme.title)
        .add_modifier(Modifier::BOLD);
    let divider_style = Style::default().fg(theme.border);

    let (sections, close_hint) = match state.language() {
        crate::i18n::Language::English => (
            vec![
                ("PROJECT NAVIGATION", "──────────────────"),
                ("  1-9", "Quick switch to project"),
                ("  Tab/Shift+Tab", "Switch between projects"),
                ("  j/k/↑/↓", "Navigate project list"),
                ("  Enter", "Enter Interactive Shell"),
                ("", ""),
                ("DEV SERVER", "──────────"),
                ("  r", "Run command (opens palette)"),
                ("  s", "Stop dev server"),
                ("  x", "Send interrupt (Ctrl+C)"),
                ("  p", "Pause/Resume (freeze)"),
                ("", ""),
                ("INTERACTIVE SHELL", "─────────────────"),
                ("  R (Shift+r)", "Run command in shell"),
                ("  All keys", "Sent to shell directly"),
                ("  Esc", "Return to sidebar"),
                ("", ""),
                ("PROJECT MANAGEMENT", "──────────────────"),
                ("  a", "Add new project"),
                ("  e", "Edit project alias"),
                ("  c", "Add custom command"),
                ("  d", "Delete project"),
                ("", ""),
                ("GENERAL", "───────"),
                ("  ,", "Open settings"),
                ("  q/Ctrl+C", "Quit application"),
                ("  ?", "Toggle this help"),
            ],
            "Press Esc or ? to close",
        ),
        crate::i18n::Language::Chinese => (
            vec![
                ("项目导航", "────────"),
                ("  1-9", "快速切换项目"),
                ("  Tab/Shift+Tab", "切换项目"),
                ("  j/k/↑/↓", "上下导航"),
                ("  Enter", "进入交互终端"),
                ("", ""),
                ("开发服务", "────────"),
                ("  r", "运行命令"),
                ("  s", "停止服务"),
                ("  x", "发送中断 (Ctrl+C)"),
                ("  p", "暂停/恢复 (冻结进程)"),
                ("", ""),
                ("交互终端", "────────"),
                ("  R (Shift+r)", "在终端运行命令"),
                ("  所有按键", "直接发送给终端"),
                ("  Esc", "返回侧边栏"),
                ("", ""),
                ("项目管理", "────────"),
                ("  a", "添加项目"),
                ("  e", "编辑别名"),
                ("  c", "添加自定义命令"),
                ("  d", "删除项目"),
                ("", ""),
                ("通用", "────"),
                ("  ,", "打开设置"),
                ("  q/Ctrl+C", "退出程序"),
                ("  ?", "帮助"),
            ],
            "按 Esc 或 ? 关闭",
        ),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("")); // 顶部空行

    for (key, desc) in sections {
        if key.is_empty() {
            lines.push(Line::from(""));
        } else if !key.starts_with("  ") {
            // 章节标题
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", key), section_style),
                Span::styled(format!(" {}", desc), divider_style),
            ]));
        } else {
            // 快捷键行
            let padded_key = format!("{:16}", key);
            lines.push(Line::from(vec![
                Span::styled(padded_key, key_style),
                Span::styled(desc, desc_style),
            ]));
        }
    }

    // 底部提示
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", close_hint),
        Style::default()
            .fg(theme.border)
            .add_modifier(Modifier::DIM),
    )]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
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

/// 计算居中矩形（使用固定尺寸）
/// width: 弹窗宽度（字符数）
/// height: 弹窗高度（行数）
fn centered_fixed_rect(width: u16, height: u16, r: Rect) -> Rect {
    // 确保弹窗不超过可用区域
    let actual_width = width.min(r.width.saturating_sub(2));
    let actual_height = height.min(r.height.saturating_sub(2));

    // 计算居中位置
    let x = r.x + (r.width.saturating_sub(actual_width)) / 2;
    let y = r.y + (r.height.saturating_sub(actual_height)) / 2;

    Rect::new(x, y, actual_width, actual_height)
}
