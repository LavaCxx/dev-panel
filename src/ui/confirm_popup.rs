//! 确认对话框模块

use crate::app::AppState;
use crate::ui::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// 计算居中矩形（使用固定尺寸）
/// width: 弹窗宽度（字符数）
/// height: 弹窗高度（行数）
pub fn centered_fixed_rect(width: u16, height: u16, r: Rect) -> Rect {
    // 确保弹窗不超过可用区域
    let actual_width = width.min(r.width.saturating_sub(2));
    let actual_height = height.min(r.height.saturating_sub(2));

    // 计算居中位置
    let x = r.x + (r.width.saturating_sub(actual_width)) / 2;
    let y = r.y + (r.height.saturating_sub(actual_height)) / 2;

    Rect::new(x, y, actual_width, actual_height)
}

/// 绘制确认对话框
pub fn draw_confirm_popup(frame: &mut Frame, state: &AppState, message: &str, theme: &Theme) {
    let i18n = state.i18n();
    // 使用固定尺寸而非百分比，确保弹窗大小足够显示内容
    let area = centered_fixed_rect(40, 7, frame.area());

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(i18n.confirm())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!("{}\n\n{}", message, i18n.yes_no());
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme.fg))
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, inner);
}
