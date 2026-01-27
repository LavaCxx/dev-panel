//! 滚动条辅助模块
//! 提供统一的滚动条渲染功能
//! 使用手动绘制方式实现稳定的滚动条效果

use crate::ui::Theme;
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget, Frame};

/// 滚动条信息
pub struct ScrollInfo {
    /// 总内容行数
    pub total_lines: usize,
    /// 可见区域高度
    pub visible_height: usize,
    /// 当前滚动偏移量
    pub scroll_offset: usize,
}

impl ScrollInfo {
    pub fn new(total_lines: usize, visible_height: usize, scroll_offset: usize) -> Self {
        Self {
            total_lines,
            visible_height,
            scroll_offset,
        }
    }

    /// 检查是否需要滚动条
    pub fn needs_scrollbar(&self) -> bool {
        self.total_lines > self.visible_height
    }

    /// 获取最大滚动偏移量
    pub fn max_scroll(&self) -> usize {
        self.total_lines.saturating_sub(self.visible_height)
    }
}

/// 自定义滚动条 Widget
/// 使用固定大小的 thumb，避免伸缩效果
struct CustomScrollbar<'a> {
    scroll_info: &'a ScrollInfo,
    track_style: Style,
    thumb_style: Style,
}

impl<'a> CustomScrollbar<'a> {
    fn new(scroll_info: &'a ScrollInfo, track_style: Style, thumb_style: Style) -> Self {
        Self {
            scroll_info,
            track_style,
            thumb_style,
        }
    }
}

impl Widget for CustomScrollbar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || !self.scroll_info.needs_scrollbar() {
            return;
        }

        let track_height = area.height as usize;
        let max_scroll = self.scroll_info.max_scroll();

        // 计算 thumb 的大小
        // 与可见比例成正比，但设置最小值为轨道高度的 10%（至少 2 行）
        let visible_ratio =
            self.scroll_info.visible_height as f64 / self.scroll_info.total_lines as f64;
        let min_thumb_height = (track_height / 10).max(2);
        let thumb_height = ((track_height as f64 * visible_ratio).ceil() as usize)
            .max(min_thumb_height)
            .min(track_height);

        // 计算 thumb 的位置
        let scroll_ratio = if max_scroll > 0 {
            self.scroll_info.scroll_offset as f64 / max_scroll as f64
        } else {
            0.0
        };
        let thumb_max_pos = track_height.saturating_sub(thumb_height);
        let thumb_pos = (thumb_max_pos as f64 * scroll_ratio).round() as usize;

        // 绘制滚动条（在区域最右侧一列）
        let x = area.x + area.width.saturating_sub(1);

        for i in 0..track_height {
            let y = area.y + i as u16;
            let is_thumb = i >= thumb_pos && i < thumb_pos + thumb_height;

            let (symbol, style) = if is_thumb {
                ("┃", self.thumb_style)
            } else {
                ("│", self.track_style)
            };

            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(symbol);
                cell.set_style(style);
            }
        }
    }
}

/// 在指定区域右侧绘制滚动条
/// area: 内容区域（滚动条会绘制在右边）
/// scroll_info: 滚动信息
/// theme: 主题
pub fn draw_scrollbar(frame: &mut Frame, area: Rect, scroll_info: &ScrollInfo, theme: &Theme) {
    if !scroll_info.needs_scrollbar() {
        return;
    }

    let scrollbar = CustomScrollbar::new(
        scroll_info,
        Style::default().fg(theme.border),
        Style::default().fg(theme.border_focused),
    );

    frame.render_widget(scrollbar, area);
}
