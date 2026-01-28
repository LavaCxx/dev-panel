//! 命令面板组件
//! 显示可执行的命令列表

use crate::app::AppState;
use crate::project::CommandType;
use crate::ui::{centered_rect, draw_scrollbar, ScrollInfo, Theme};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

/// 绘制命令面板
pub fn draw_command_palette(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let area = centered_rect(60, 50, frame.area());

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Run Command ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 获取当前项目的命令列表
    let commands = if let Some(project) = state.active_project() {
        project.get_all_commands()
    } else {
        Vec::new()
    };

    if commands.is_empty() {
        let paragraph = ratatui::widgets::Paragraph::new("No commands available")
            .style(Style::default().fg(theme.border))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, inner);
        return;
    }

    // 构建命令列表
    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .map(|(idx, cmd)| {
            let is_selected = idx == state.command_palette_idx;

            // 类型标签
            let type_label = match cmd.cmd_type {
                CommandType::NpmScript => "[npm]",
                CommandType::RawShell => "[raw]",
            };

            let type_style = match cmd.cmd_type {
                CommandType::NpmScript => Style::default().fg(theme.info),
                CommandType::RawShell => Style::default().fg(theme.warning),
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme.selection_fg)
                    .bg(theme.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            ListItem::new(Line::from(vec![
                Span::styled(type_label, type_style),
                Span::raw(" "),
                Span::styled(&cmd.name, style),
            ]))
        })
        .collect();

    // 添加 "Add Custom Command" 选项
    let mut all_items = items;
    all_items.push(ListItem::new(Line::from("")));
    all_items.push(ListItem::new(Line::from(vec![Span::styled(
        "[+] Add Custom Command (c)",
        Style::default().fg(theme.success),
    )])));

    let list = List::new(all_items);

    let mut list_state = ListState::default();
    let mut scroll_offset = 0usize;
    let visible_height = inner.height as usize;
    let total_items = commands.len() + 2; // 命令数 + 空行 + "Add Custom Command"

    let selected_idx = state.command_palette_idx;
    list_state.select(Some(selected_idx));

    // 计算合适的偏移量，使选中项尽量居中
    if visible_height > 0 && total_items > visible_height {
        let half_height = visible_height / 2;

        // 计算理想的偏移量：selected_idx - half_height
        let ideal_offset = selected_idx.saturating_sub(half_height);

        // 确保偏移量不会导致列表底部出现空白
        let max_offset = total_items.saturating_sub(visible_height);
        scroll_offset = ideal_offset.min(max_offset);

        *list_state.offset_mut() = scroll_offset;
    }

    frame.render_stateful_widget(list, inner, &mut list_state);

    // 绘制滚动条
    if total_items > visible_height {
        let scroll_info = ScrollInfo::new(total_items, visible_height, scroll_offset);
        draw_scrollbar(frame, inner, &scroll_info, theme);
    }
}
