//! 目录浏览器组件
//! 用于选择项目目录

use crate::app::AppState;
use crate::ui::Theme;
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

    // 分割内部区域：路径显示 + 目录列表 + 帮助提示
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // 当前路径
            Constraint::Min(1),    // 目录列表
            Constraint::Length(2), // 帮助提示
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

                // 图标：驱动器选择模式用磁盘图标，有 package.json 用包图标，否则用文件夹图标
                let icon = if state.dir_browser.in_drive_selection {
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
    if !state.dir_browser.entries.is_empty() {
        list_state.select(Some(state.dir_browser.selected_idx));
    }

    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    // 绘制帮助提示
    let help_text = match state.language() {
        crate::i18n::Language::English => {
            " ↑↓: Navigate | Enter: Open | Backspace: Back | Space: Select | .: Hidden | Esc: Cancel"
        }
        crate::i18n::Language::Chinese => {
            " ↑↓: 导航 | Enter: 进入 | Backspace: 返回 | Space: 选择 | .: 隐藏文件 | Esc: 取消"
        }
    };

    let help = Paragraph::new(help_text).style(
        Style::default()
            .fg(theme.border)
            .add_modifier(Modifier::DIM),
    );
    frame.render_widget(help, chunks[2]);
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
