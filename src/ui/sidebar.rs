//! 侧边栏组件
//! 显示项目列表

use crate::app::{AppState, FocusArea};
use crate::ui::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

/// 绘制侧边栏
pub fn draw_sidebar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let is_focused = state.focus == FocusArea::Sidebar;
    let i18n = state.i18n();

    // 边框颜色根据焦点状态变化
    let border_color = if is_focused {
        theme.border_focused
    } else {
        theme.border
    };

    let block = Block::default()
        .title(i18n.projects())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    // 构建项目列表项
    let items: Vec<ListItem> = if state.projects.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            i18n.no_projects(),
            Style::default().fg(theme.border),
        )]))]
    } else {
        state
            .projects
            .iter()
            .enumerate()
            .map(|(idx, project)| {
                let is_selected = idx == state.active_project_idx;
                let is_running = project.is_dev_running();

                // 数字角标 (1-9 可快捷切换)
                let number_badge = if idx < 9 {
                    format!("{} ", idx + 1)
                } else {
                    "  ".to_string()
                };

                // 选中指示器
                let prefix = if is_selected { "▶ " } else { "  " };

                // 状态指示器：运行中/暂停/无
                let is_suspended = project
                    .dev_pty
                    .as_ref()
                    .map(|p| p.suspended)
                    .unwrap_or(false);

                let (status_icon, status_style) = if is_suspended {
                    ("⏸", Style::default().fg(theme.warning))
                } else if is_running {
                    // 运行中时显示绿色圆点
                    ("●", Style::default().fg(theme.success))
                } else {
                    ("", Style::default())
                };

                // 运行时间（如果有）
                let uptime = if is_running && !is_suspended {
                    project
                        .dev_uptime()
                        .map(|t| format!(" {}", t))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                // 主样式
                let style = if is_selected {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                // 角标样式
                let badge_style = if is_selected {
                    Style::default()
                        .fg(theme.info)
                        .bg(theme.selection)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(theme.border)
                };

                // 选择器样式
                let selector_style = if is_selected {
                    Style::default().fg(theme.info).bg(theme.selection)
                } else {
                    Style::default().fg(theme.border)
                };

                // 运行时间样式
                let uptime_style = if is_selected {
                    Style::default()
                        .fg(theme.border)
                        .bg(theme.selection)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default()
                        .fg(theme.border)
                        .add_modifier(Modifier::DIM)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(number_badge, badge_style),
                    Span::styled(prefix, selector_style),
                    Span::styled(project.display_name(), style),
                    Span::styled(uptime, uptime_style),
                    Span::styled(format!(" {}", status_icon), status_style),
                ]))
            })
            .collect()
    };

    // 添加 "Add Project" 按钮
    let mut all_items = items;
    all_items.push(ListItem::new(Line::from(""))); // 空行
    all_items.push(ListItem::new(Line::from(vec![Span::styled(
        i18n.add_project_hint(),
        Style::default().fg(theme.info),
    )])));

    let list = List::new(all_items).block(block);

    // 设置选中状态
    let mut list_state = ListState::default();
    if !state.projects.is_empty() {
        list_state.select(Some(state.active_project_idx));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}
