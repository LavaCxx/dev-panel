//! 目录浏览器组件
//! 用于选择项目目录

use crate::app::AppState;
use crate::ui::{draw_scrollbar, ScrollInfo, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// 绘制目录浏览器弹窗
pub fn draw_dir_browser(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let i18n = state.i18n();
    let area = centered_rect(70, 80, frame.area());

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.select_directory())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.info))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 预计算帮助提示需要的行数
    let help_items_count = 6; // 6个帮助项
    let avg_item_width = 15; // 平均每个项目宽度（包括分隔符）
    let total_width_needed = help_items_count * avg_item_width;
    let help_lines_needed = if inner.width > 0 {
        ((total_width_needed as u16) / inner.width).max(1) + 1
    } else {
        2
    };
    let help_height = help_lines_needed.min(3); // 最多3行

    // 分割内部区域：路径显示 + 目录列表 + 帮助提示
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),           // 当前路径
            Constraint::Min(1),              // 目录列表
            Constraint::Length(help_height), // 帮助提示（动态高度）
        ])
        .split(inner);

    // 绘制当前路径
    let path_text = if state.dir_browser.in_drive_selection {
        format!(" {} {}", "💾", i18n.select_drive())
    } else {
        format!(" {} {}", "📂", state.dir_browser.current_dir.display())
    };
    let path = Paragraph::new(path_text).style(
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(path, chunks[0]);

    // 构建目录列表
    let items: Vec<ListItem> = if state.dir_browser.entries.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            i18n.empty_directory(),
            Style::default().fg(theme.border),
        )]))]
    } else {
        state
            .dir_browser
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == state.dir_browser.selected_idx;

                // 图标：返回上级用箭头，驱动器选择模式用磁盘图标，有 package.json 用包图标，否则用文件夹图标
                let icon = if entry.name == ".." {
                    "⬆️"
                } else if state.dir_browser.in_drive_selection {
                    "💿"
                } else if entry.has_package_json {
                    "📦"
                } else {
                    "📁"
                };

                // 样式
                let style = if is_selected {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                // 如果有 package.json，高亮显示
                let name_style = if entry.has_package_json {
                    style.fg(theme.success)
                } else {
                    style
                };

                let prefix = if is_selected { "▶ " } else { "  " };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(&entry.name, name_style),
                    if entry.has_package_json {
                        Span::styled(" (project)", Style::default().fg(theme.border))
                    } else {
                        Span::raw("")
                    },
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border)),
    );

    let mut list_state = ListState::default();
    let mut scroll_offset = 0usize;
    let visible_height = chunks[1].height.saturating_sub(1) as usize; // 减去顶部边框
    let total_items = state.dir_browser.entries.len();

    if !state.dir_browser.entries.is_empty() {
        let selected_idx = state.dir_browser.selected_idx;
        list_state.select(Some(selected_idx));

        // 计算合适的偏移量，使选中项尽量居中
        // 目标：将选中项放在可见区域的中间位置
        if visible_height > 0 {
            let half_height = visible_height / 2;

            // 计算理想的偏移量：selected_idx - half_height
            // 但要确保偏移量不会超出有效范围
            let ideal_offset = selected_idx.saturating_sub(half_height);

            // 确保偏移量不会导致列表底部出现空白
            // 最大偏移量 = 总项数 - 可见高度
            let max_offset = total_items.saturating_sub(visible_height);
            scroll_offset = ideal_offset.min(max_offset);

            *list_state.offset_mut() = scroll_offset;
        }
    }

    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    // 绘制滚动条
    if total_items > 0 {
        let scroll_info = ScrollInfo::new(total_items, visible_height, scroll_offset);
        draw_scrollbar(frame, chunks[1], &scroll_info, theme);
    }

    // 绘制帮助提示（支持自动换行）
    let help_items = match state.language() {
        crate::i18n::Language::English => vec![
            ("↑↓", "Navigate"),
            ("Enter", "Open"),
            ("Backspace", "Back"),
            ("Space", "Select"),
            (".", "Hidden"),
            ("Esc", "Cancel"),
        ],
        crate::i18n::Language::Chinese => vec![
            ("↑↓", "导航"),
            ("Enter", "进入"),
            ("Backspace", "返回"),
            ("Space", "选择"),
            (".", "隐藏文件"),
            ("Esc", "取消"),
        ],
    };

    let lines = build_help_lines(&help_items, chunks[2].width as usize, theme);
    let help = Paragraph::new(lines);
    frame.render_widget(help, chunks[2]);
}

/// 根据可用宽度构建帮助提示行（自动换行）
fn build_help_lines<'a>(
    items: &[(&'a str, &'a str)],
    available_width: usize,
    theme: &Theme,
) -> Vec<Line<'a>> {
    let key_style = Style::default().fg(theme.info);
    let desc_style = Style::default().fg(theme.fg);
    let sep_style = Style::default().fg(theme.border);

    let mut lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = vec![Span::raw(" ")];
    let mut current_width: usize = 1; // 起始空格

    for (i, (key, desc)) in items.iter().enumerate() {
        // 计算这个条目的宽度（包括分隔符）
        let separator_width = if i > 0 { 3 } else { 0 }; // " | "
                                                         // 使用 Unicode 宽度计算（中文字符算2个宽度）
        let key_width = key.chars().count();
        let desc_width = desc
            .chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>();
        let item_width = separator_width + key_width + 2 + desc_width; // key + ": " + desc

        // 检查是否需要换行
        if current_width + item_width > available_width && current_spans.len() > 1 {
            // 当前行已满，换行
            lines.push(Line::from(std::mem::take(&mut current_spans)));
            current_spans = vec![Span::raw(" ")];
            current_width = 1;
        }

        // 添加分隔符（如果不是行首）
        if current_spans.len() > 1 {
            current_spans.push(Span::styled(" | ", sep_style));
            current_width += 3;
        }

        // 添加 key: desc
        current_spans.push(Span::styled(*key, key_style));
        current_spans.push(Span::styled(": ", sep_style));
        current_spans.push(Span::styled(*desc, desc_style));
        current_width += item_width - separator_width;
    }

    // 添加最后一行
    if current_spans.len() > 1 {
        lines.push(Line::from(current_spans));
    }

    // 确保至少有一行
    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

/// 计算居中矩形
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
