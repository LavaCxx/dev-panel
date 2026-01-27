//! 终端面板组件
//! 用于显示 PTY 输出，支持 ANSI 颜色

use crate::i18n::I18n;
use crate::pty::PtyHandle;
use crate::ui::{draw_scrollbar, ScrollInfo, Theme};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// 绘制终端面板
/// is_interactive: 是否是交互式终端（Shell），交互式终端在聚焦时显示光标
#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    pty: Option<&PtyHandle>,
    is_focused: bool,
    is_interactive: bool,
    scroll_offset: usize,
    i18n: &I18n,
    theme: &Theme,
) {
    let border_color = if is_focused {
        theme.border_focused
    } else {
        theme.border
    };

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 如果有 PTY，渲染终端内容
    if let Some(pty_handle) = pty {
        // 尝试获取 parser 锁并渲染内容
        if let Ok(parser) = pty_handle.parser.try_lock() {
            let screen = parser.screen();
            let mut lines: Vec<Line> = Vec::new();

            // 使用屏幕的实际大小
            let screen_rows = screen.size().0 as usize;
            let screen_cols = screen.size().1 as usize;
            let visible_rows = inner.height as usize;

            // 获取光标位置（用于交互式终端）
            let (cursor_row, cursor_col) = screen.cursor_position();

            // 找到有内容的最后一行（用于计算滚动）
            let mut last_content_row = 0;
            for row in 0..screen_rows {
                for col in 0..screen_cols {
                    if let Some(cell) = screen.cell(row as u16, col as u16) {
                        if !cell.contents().is_empty() && cell.contents() != " " {
                            last_content_row = row;
                        }
                    }
                }
            }

            // 计算起始行
            let start_row = if is_interactive {
                // 交互式终端：以光标位置为基准
                let base_start = if (cursor_row as usize) >= visible_rows {
                    (cursor_row as usize) + 1 - visible_rows
                } else {
                    0
                };
                // 应用滚动偏移（向上滚动查看历史）
                base_start.saturating_sub(scroll_offset)
            } else {
                // 非交互式终端（Dev Server）：显示最后的内容
                // 计算起始行（考虑滚动偏移）
                let base_start = if last_content_row >= visible_rows {
                    last_content_row + 1 - visible_rows
                } else {
                    0
                };

                // 应用滚动偏移（向上滚动）
                base_start.saturating_sub(scroll_offset)
            };

            for row in start_row..screen_rows.min(start_row + visible_rows) {
                let mut spans: Vec<Span> = Vec::new();
                let mut current_text = String::new();
                let mut current_style = Style::default();

                for col in 0..screen_cols.min(inner.width as usize) {
                    let cell = screen.cell(row as u16, col as u16);
                    if let Some(cell) = cell {
                        let char_content = cell.contents();
                        let char_to_add = if char_content.is_empty() {
                            ' '
                        } else {
                            char_content.chars().next().unwrap_or(' ')
                        };

                        // 转换 vt100 颜色到 ratatui 颜色
                        let new_style = vt100_to_ratatui_style(cell);

                        // 如果样式变化，保存当前 span 并开始新的
                        if new_style != current_style && !current_text.is_empty() {
                            spans.push(Span::styled(current_text.clone(), current_style));
                            current_text.clear();
                        }

                        current_style = new_style;
                        current_text.push(char_to_add);
                    } else {
                        current_text.push(' ');
                    }
                }

                // 添加剩余文本（保留行，即使是空的）
                let trimmed = current_text.trim_end();
                if !trimmed.is_empty() {
                    spans.push(Span::styled(trimmed.to_string(), current_style));
                }

                lines.push(Line::from(spans));
            }

            // 如果完全没有内容，显示等待提示
            if lines.iter().all(|l| l.spans.is_empty()) {
                let hint = if pty_handle.running {
                    i18n.shell_waiting()
                } else {
                    i18n.shell_ended()
                };
                let paragraph = Paragraph::new(hint).style(Style::default().fg(theme.border));
                frame.render_widget(paragraph, inner);
            } else {
                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, inner);
            }

            // 绘制滚动条（如果有内容需要滚动）
            let total_lines = last_content_row + 1;
            if total_lines > visible_rows {
                let scroll_position = if is_interactive {
                    // 交互式终端：滚动位置基于光标位置
                    start_row
                } else {
                    // 非交互式终端：滚动位置基于用户滚动偏移
                    let max_scroll = total_lines.saturating_sub(visible_rows);
                    max_scroll.saturating_sub(scroll_offset)
                };
                let scroll_info = ScrollInfo::new(total_lines, visible_rows, scroll_position);
                draw_scrollbar(frame, inner, &scroll_info, theme);
            }

            // 在聚焦状态下显示光标
            // 只在交互式终端（Shell Terminal）聚焦时显示光标
            if is_focused && is_interactive {
                // 计算光标相对于可见区域的位置
                let cursor_relative_row = (cursor_row as usize).saturating_sub(start_row);

                // 光标应该始终在可见区域内（因为我们以光标为基准计算 start_row）
                if cursor_relative_row < visible_rows {
                    let cursor_x = inner.x.saturating_add(cursor_col);
                    let cursor_y = inner.y.saturating_add(cursor_relative_row as u16);

                    // 确保光标在内部区域内
                    if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                        frame.set_cursor_position((cursor_x, cursor_y));
                    }
                }
            }
        } else {
            // 无法获取锁，显示加载中
            let paragraph = Paragraph::new(i18n.loading()).style(Style::default().fg(theme.border));
            frame.render_widget(paragraph, inner);
        }
    } else {
        // 没有 PTY，显示提示信息
        let hint = if title.contains("Dev") || title.contains("开发") {
            i18n.press_r_to_run()
        } else {
            i18n.press_enter_for_shell()
        };

        let paragraph = Paragraph::new(hint)
            .style(Style::default().fg(theme.border))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, inner);
    }
}

/// 将 vt100 Cell 的颜色转换为 ratatui Style
fn vt100_to_ratatui_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default();

    // 前景色
    style = style.fg(vt100_color_to_ratatui(cell.fgcolor()));

    // 背景色
    let bg = vt100_color_to_ratatui(cell.bgcolor());
    if bg != Color::Reset {
        style = style.bg(bg);
    }

    // 文字样式
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }

    style
}

/// 将 vt100 颜色转换为 ratatui 颜色
fn vt100_color_to_ratatui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
