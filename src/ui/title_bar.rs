//! 标题栏模块

use crate::app::AppState;
use crate::ui::Theme;
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::Paragraph,
    Frame,
};

/// 绘制标题栏
pub fn draw_title_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let i18n = state.i18n();
    let title = Paragraph::new(i18n.app_title())
        .style(Style::default().fg(theme.title).bold())
        .bg(theme.status_bg);
    frame.render_widget(title, area);
}
