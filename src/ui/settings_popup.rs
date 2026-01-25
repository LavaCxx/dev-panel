//! 设置弹窗组件

use crate::app::AppState;
use crate::ui::{centered_rect, Theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// 设置项枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingItem {
    Language,
    #[cfg(windows)]
    WindowsShell,
}

impl SettingItem {
    /// 获取所有设置项（根据平台动态返回）
    pub fn all() -> Vec<Self> {
        #[cfg(windows)]
        {
            vec![SettingItem::Language, SettingItem::WindowsShell]
        }
        #[cfg(not(windows))]
        {
            vec![SettingItem::Language]
        }
    }

    /// 设置项数量
    pub fn count() -> usize {
        Self::all().len()
    }
}

/// 绘制设置弹窗
pub fn draw_settings_popup(frame: &mut Frame, state: &AppState, theme: &Theme) {
    let area = centered_rect(50, 40, frame.area());
    let i18n = state.i18n();

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.settings())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.info))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 构建设置列表
    let setting_items = SettingItem::all();
    let items: Vec<ListItem> = setting_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = state.settings_idx == idx;
            match item {
                SettingItem::Language => create_setting_item(
                    i18n.language(),
                    state.config.settings.language.display_name(),
                    is_selected,
                    theme,
                ),
                #[cfg(windows)]
                SettingItem::WindowsShell => create_setting_item(
                    i18n.shell(),
                    state.config.settings.windows_shell.display_name(),
                    is_selected,
                    theme,
                ),
            }
        })
        .collect();

    let list = List::new(items);

    // 计算内容区域（留出底部提示区域）
    let content_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(2),
    };

    let mut list_state = ListState::default();
    list_state.select(Some(state.settings_idx));
    frame.render_stateful_widget(list, content_area, &mut list_state);

    // 底部提示
    let hint_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };

    let hint = Paragraph::new(i18n.settings_hint())
        .style(Style::default().fg(theme.border))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(hint, hint_area);
}

/// 创建设置项
fn create_setting_item<'a>(
    label: &'a str,
    value: &'a str,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'a> {
    let style = if is_selected {
        Style::default()
            .fg(theme.selection_fg)
            .bg(theme.selection)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    };

    let value_style = if is_selected {
        Style::default().fg(theme.info).bg(theme.selection)
    } else {
        Style::default().fg(theme.info)
    };

    let prefix = if is_selected { "▸ " } else { "  " };

    ListItem::new(Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(label, style),
        Span::styled(": ", style),
        Span::styled(value, value_style),
    ]))
}
