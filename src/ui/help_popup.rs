//! 帮助弹窗模块

use crate::app::AppState;
use crate::ui::{centered_rect, draw_scrollbar, ScrollInfo, Theme};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// 帮助弹窗
pub fn draw_help_popup(frame: &mut Frame, state: &mut AppState, theme: &Theme) {
    let i18n = state.i18n();
    let area = centered_rect(55, 80, frame.area());

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
                ("DEV LOG VIEW (click Dev panel)", ""),
                ("  j/k/↑/↓", "Scroll log"),
                ("  PgUp/PgDn", "Fast scroll"),
                ("  Home", "Jump to latest"),
                ("  Esc", "Exit log view"),
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
                ("  z", "Toggle panel layout"),
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
                ("日志查看 (点击开发服务面板)", ""),
                ("  j/k/↑/↓", "滚动日志"),
                ("  PgUp/PgDn", "快速滚动"),
                ("  Home", "跳到最新"),
                ("  Esc", "退出查看"),
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
                ("  z", "切换面板布局"),
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

    // 底部提示固定区域高度（1行提示 + 1行空行）
    let hint_height = 2u16;

    // 分割内容区域：上方为可滚动内容，下方为固定提示
    let content_height = inner.height.saturating_sub(hint_height);
    let content_area = ratatui::layout::Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: content_height,
    };
    let hint_area = ratatui::layout::Rect {
        x: inner.x,
        y: inner.y + content_height,
        width: inner.width,
        height: hint_height,
    };

    // 计算滚动信息
    let total_lines = lines.len();
    let visible_height = content_area.height as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);

    // 限制目标位置不超过最大滚动值（避免过度滚动）
    state.help_scroll.clamp_target(max_scroll as f32);

    // 使用平滑滚动的当前位置
    let scroll_offset = state.help_scroll.position();

    // 渲染可滚动内容
    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, content_area);

    // 绘制滚动条
    let scroll_info = ScrollInfo::new(total_lines, visible_height, scroll_offset);
    draw_scrollbar(frame, content_area, &scroll_info, theme);

    // 渲染固定在底部的关闭提示
    let hint_lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {}", close_hint),
            Style::default()
                .fg(theme.border)
                .add_modifier(Modifier::DIM),
        )]),
    ];
    let hint_paragraph = Paragraph::new(hint_lines);
    frame.render_widget(hint_paragraph, hint_area);
}
