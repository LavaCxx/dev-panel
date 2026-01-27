//! 状态栏模块

use crate::app::{AppState, FocusArea};
use crate::ui::Theme;
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// 获取状态栏帮助项（用于计算高度和渲染）
pub fn get_status_help_items(state: &AppState) -> Vec<(&'static str, &'static str)> {
    match state.focus {
        FocusArea::Sidebar | FocusArea::DevTerminal => match state.language() {
            crate::i18n::Language::English => vec![
                ("Tab", "Project"),
                ("Enter", "Shell"),
                ("r/R", "Run"),
                ("s", "Stop"),
                ("z", "Layout"),
                ("?", "Help"),
            ],
            crate::i18n::Language::Chinese => vec![
                ("Tab", "切换项目"),
                ("Enter", "终端"),
                ("r/R", "运行"),
                ("s", "停止"),
                ("z", "布局"),
                ("?", "帮助"),
            ],
        },
        FocusArea::ShellTerminal => match state.language() {
            crate::i18n::Language::English => vec![("Esc", "Back")],
            crate::i18n::Language::Chinese => vec![("Esc", "返回")],
        },
    }
}

/// 计算状态栏需要的高度
pub fn calculate_status_bar_height(state: &AppState, screen_width: usize) -> u16 {
    // 如果有状态消息，只需要1行
    if state.status_message.is_some() {
        return 1;
    }

    let items = get_status_help_items(state);
    let lines = build_status_help_lines(&items, screen_width);
    (lines.len() as u16).clamp(1, 3) // 最少1行，最多3行
}

/// 根据可用宽度构建状态栏帮助提示行（自动换行）
pub fn build_status_help_lines<'a>(
    items: &[(&'a str, &'a str)],
    available_width: usize,
) -> Vec<Vec<(&'a str, &'a str)>> {
    let mut lines: Vec<Vec<(&'a str, &'a str)>> = Vec::new();
    let mut current_line: Vec<(&'a str, &'a str)> = Vec::new();
    let mut current_width: usize = 1; // 起始空格

    for (key, desc) in items.iter() {
        // 计算这个条目的宽度（包括分隔符）
        let separator_width = if current_line.is_empty() { 0 } else { 3 }; // " | "
        let key_width = key.chars().count();
        let desc_width = desc
            .chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>();
        let item_width = separator_width + key_width + 2 + desc_width; // key + ": " + desc

        // 检查是否需要换行
        if current_width + item_width > available_width && !current_line.is_empty() {
            lines.push(std::mem::take(&mut current_line));
            current_width = 1;
        }

        current_line.push((key, desc));
        current_width += item_width;
    }

    // 添加最后一行
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(Vec::new());
    }

    lines
}

/// 绘制状态栏
pub fn draw_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    // 检查是否有状态消息
    if let Some(ref msg) = state.status_message {
        // 计算状态消息的颜色（支持淡出效果）
        let fg_color = {
            let opacity = state.status_opacity();
            if opacity > 0.5 {
                theme.success // 高亮显示
            } else {
                theme.border // 淡出时变暗
            }
        };

        let display_text = format!(" ✓ {}", msg.text);
        let status = Paragraph::new(display_text)
            .style(Style::default().fg(fg_color))
            .bg(theme.status_bg);
        frame.render_widget(status, area);
        return;
    }

    // 绘制帮助提示（支持换行）
    let items = get_status_help_items(state);
    let item_lines = build_status_help_lines(&items, area.width as usize);

    let key_style = Style::default().fg(theme.info);
    let desc_style = Style::default().fg(theme.status_fg);
    let sep_style = Style::default().fg(theme.border);

    let mut lines: Vec<Line> = Vec::new();

    // 如果是 Shell 终端模式，添加提示文本
    if state.focus == FocusArea::ShellTerminal {
        let prefix = match state.language() {
            crate::i18n::Language::English => " Interactive Shell - type freely ",
            crate::i18n::Language::Chinese => " 交互终端 - 自由输入 ",
        };
        let mut spans = vec![Span::styled(prefix, desc_style)];

        // 添加 Esc 提示
        if let Some(first_line) = item_lines.first() {
            if let Some((key, desc)) = first_line.first() {
                spans.push(Span::styled("(", sep_style));
                spans.push(Span::styled(*key, key_style));
                spans.push(Span::styled(": ", sep_style));
                spans.push(Span::styled(*desc, desc_style));
                spans.push(Span::styled(")", sep_style));
            }
        }
        lines.push(Line::from(spans));
    } else {
        // 普通模式，显示所有帮助项
        for line_items in item_lines {
            let mut spans: Vec<Span> = vec![Span::raw(" ")];

            for (i, (key, desc)) in line_items.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled(" | ", sep_style));
                }
                spans.push(Span::styled(*key, key_style));
                spans.push(Span::styled(": ", sep_style));
                spans.push(Span::styled(*desc, desc_style));
            }

            lines.push(Line::from(spans));
        }
    }

    let status = Paragraph::new(lines).bg(theme.status_bg);
    frame.render_widget(status, area);
}
