//! 侧边栏组件
//! 显示项目列表和进程资源使用信息

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

    // 计算可用宽度（减去边框和内边距）
    let content_width = area.width.saturating_sub(2) as usize;

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
                    ("⏸ ", Style::default().fg(theme.warning))
                } else if is_running {
                    // 运行中时显示绿色圆点
                    ("● ", Style::default().fg(theme.success))
                } else {
                    (" ", Style::default())
                };

                // 资源信息的预估宽度（用于计算项目名称最大宽度）
                // 格式: " 100%|999.9M" 约 12 个字符
                let resource_info_width = if is_running { 12 } else { 0 };

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

                // 资源信息样式 - CPU 使用黄色，内存使用青色
                let cpu_style = if is_selected {
                    Style::default().fg(theme.warning).bg(theme.selection)
                } else {
                    Style::default().fg(theme.warning)
                };

                let mem_style = if is_selected {
                    Style::default().fg(theme.info).bg(theme.selection)
                } else {
                    Style::default().fg(theme.info)
                };

                let separator_style = if is_selected {
                    Style::default().fg(theme.border).bg(theme.selection)
                } else {
                    Style::default().fg(theme.border)
                };

                // 计算项目名称的最大宽度
                // 格式: "1 ▶ project_name ● 50%|128M"
                let fixed_width =
                    number_badge.len() + prefix.len() + 1 + status_icon.len() + resource_info_width;
                let max_name_width = content_width.saturating_sub(fixed_width);

                // 截断项目名称（如果需要）
                let display_name = project.display_name();
                let truncated_name = if display_name.len() > max_name_width && max_name_width > 3 {
                    format!("{}...", &display_name[..max_name_width.saturating_sub(3)])
                } else {
                    display_name.to_string()
                };

                // 构建资源信息的 spans（分别着色）
                let mut spans = vec![
                    Span::styled(number_badge, badge_style),
                    Span::styled(prefix, selector_style),
                    Span::styled(truncated_name, style),
                    Span::styled(format!(" {}", status_icon), status_style),
                ];

                // 如果有资源信息，分别添加 CPU 和内存（不同颜色）
                if is_running {
                    if let Some(ref pty) = project.dev_pty {
                        let usage = &pty.resource_usage;
                        spans.push(Span::styled(" ", separator_style));
                        spans.push(Span::styled(usage.format_cpu(), cpu_style));
                        spans.push(Span::styled("|", separator_style));
                        spans.push(Span::styled(usage.format_memory(), mem_style));
                        spans.push(Span::styled(" ", separator_style));
                    }
                }

                ListItem::new(Line::from(spans))
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
