//! 输入弹窗组件
//! 用于各种需要文本输入的场景

use crate::ui::{centered_rect, Theme};
use ratatui::{
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// 绘制输入弹窗
pub fn draw_input_popup(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    input: &str,
    theme: &Theme,
) {
    let area = centered_rect(60, 20, frame.area());

    // 清除背景
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 提示文字
    let prompt_text = format!("{}\n\n> {}█", prompt, input);

    let paragraph = Paragraph::new(prompt_text)
        .style(Style::default().fg(theme.fg));

    frame.render_widget(paragraph, inner);
}
