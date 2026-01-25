//! 命令面板组件
//! 显示可执行的命令列表

use crate::app::AppState;
use crate::project::CommandType;
use crate::ui::{centered_rect, Theme};
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
    list_state.select(Some(state.command_palette_idx));

    frame.render_stateful_widget(list, inner, &mut list_state);
}
