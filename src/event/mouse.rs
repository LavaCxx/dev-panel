//! 鼠标事件处理模块

use crate::app::{AppMode, AppState, FocusArea};
use crate::pty::PtyManager;
use crate::ui::centered_rect;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use super::helpers::start_shell_for_active_project;
use super::SIDEBAR_WIDTH;

/// 计算居中矩形区域（用于弹窗点击检测）
/// 使用与渲染相同的 centered_rect 函数确保一致性
fn calc_centered_rect(
    percent_x: u16,
    percent_y: u16,
    term_width: u16,
    term_height: u16,
) -> (u16, u16, u16, u16) {
    let term_rect = Rect::new(0, 0, term_width, term_height);
    let area = centered_rect(percent_x, percent_y, term_rect);
    (area.x, area.y, area.width, area.height)
}

/// 检查点击是否在矩形区域内
fn is_point_in_rect(px: u16, py: u16, rx: u16, ry: u16, rw: u16, rh: u16) -> bool {
    px >= rx && px < rx + rw && py >= ry && py < ry + rh
}

/// 处理鼠标事件
pub fn handle_mouse_event(
    state: &mut AppState,
    mouse: MouseEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));

    // 处理弹窗模式下的鼠标点击（点击空白处关闭弹窗）
    if state.mode != AppMode::Normal {
        return handle_popup_mouse_event(state, mouse, term_width, term_height);
    }

    let content_height = term_height.saturating_sub(2);
    let half_height = content_height / 2;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(state, mouse, pty_manager, half_height, term_height)
        }
        MouseEventKind::ScrollUp => {
            handle_scroll_up(state);
            Ok(true)
        }
        MouseEventKind::ScrollDown => {
            handle_scroll_down(state);
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// 处理弹窗模式下的鼠标事件
fn handle_popup_mouse_event(
    state: &mut AppState,
    mouse: MouseEvent,
    term_width: u16,
    term_height: u16,
) -> anyhow::Result<bool> {
    // 滚动步进常量（与键盘滚动保持一致）
    const MOUSE_SCROLL_STEP: u16 = 3;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            // 根据弹窗类型计算区域
            let popup_area = match &state.mode {
                AppMode::BrowseDirectory => {
                    Some(calc_centered_rect(70, 80, term_width, term_height))
                }
                AppMode::Help => Some(calc_centered_rect(55, 80, term_width, term_height)),
                AppMode::Settings => Some(calc_centered_rect(50, 40, term_width, term_height)),
                AppMode::CommandPalette => {
                    Some(calc_centered_rect(60, 50, term_width, term_height))
                }
                AppMode::AddProject | AppMode::AddCommand | AppMode::EditAlias => {
                    Some(calc_centered_rect(60, 20, term_width, term_height))
                }
                AppMode::Confirm(_) => {
                    // 固定尺寸弹窗：40x7
                    let width = 40u16.min(term_width.saturating_sub(2));
                    let height = 7u16.min(term_height.saturating_sub(2));
                    let px = (term_width.saturating_sub(width)) / 2;
                    let py = (term_height.saturating_sub(height)) / 2;
                    Some((px, py, width, height))
                }
                AppMode::Normal => None,
            };

            if let Some((rx, ry, rw, rh)) = popup_area {
                if !is_point_in_rect(x, y, rx, ry, rw, rh) {
                    // 点击在弹窗区域外，关闭弹窗
                    state.exit_mode();
                    return Ok(true);
                }

                // 目录浏览器：处理点击目录条目
                if let AppMode::BrowseDirectory = state.mode {
                    return handle_dir_browser_click(state, x, y, rx, ry, rw, rh);
                }
            }
        }
        // 弹窗内鼠标滚轮支持
        MouseEventKind::ScrollUp => match state.mode {
            AppMode::Help => {
                // 使用平滑滚动：向上滚动（内容向下移动，offset 增加）
                state.help_scroll.scroll_by(MOUSE_SCROLL_STEP as f32);
                return Ok(true);
            }
            AppMode::BrowseDirectory => {
                // 目录浏览器向上滚动（由 dir_browser 内部处理）
                state.dir_browser.scroll_up(3);
                return Ok(true);
            }
            AppMode::CommandPalette => {
                // 命令面板向上滚动
                state.command_palette_idx = state.command_palette_idx.saturating_sub(1);
                return Ok(true);
            }
            _ => {}
        },
        MouseEventKind::ScrollDown => match state.mode {
            AppMode::Help => {
                // 使用平滑滚动：向下滚动（内容向上移动，offset 减少）
                state.help_scroll.scroll_by(-(MOUSE_SCROLL_STEP as f32));
                return Ok(true);
            }
            AppMode::BrowseDirectory => {
                // 目录浏览器向下滚动
                state.dir_browser.scroll_down(3);
                return Ok(true);
            }
            AppMode::CommandPalette => {
                // 命令面板向下滚动（需要知道最大索引，这里简单 +1）
                state.command_palette_idx = state.command_palette_idx.saturating_add(1);
                return Ok(true);
            }
            _ => {}
        },
        _ => {}
    }
    Ok(false)
}

/// 处理目录浏览器中的点击事件
fn handle_dir_browser_click(
    state: &mut AppState,
    _x: u16,
    y: u16,
    rx: u16,
    ry: u16,
    rw: u16,
    rh: u16,
) -> anyhow::Result<bool> {
    // 使用与渲染相同的布局计算
    let popup_rect = Rect::new(rx, ry, rw, rh);
    // 内部区域（排除边框）
    // 注意：边框+标题占用顶部 1 行，但由于终端坐标系的差异需要额外偏移
    let inner = Rect::new(
        popup_rect.x + 1,
        popup_rect.y + 2, // +2 以匹配实际渲染位置
        popup_rect.width.saturating_sub(2),
        popup_rect.height.saturating_sub(3), // 相应调整高度
    );

    // 计算帮助提示高度（与 dir_browser.rs 相同的逻辑）
    let help_items_count = 6u16;
    let avg_item_width = 15u16;
    let total_width_needed = help_items_count * avg_item_width;
    let help_lines_needed = if inner.width > 0 {
        (total_width_needed / inner.width).max(1) + 1
    } else {
        2
    };
    let help_height = help_lines_needed.min(3);

    // 使用 Layout 分割内部区域（与渲染完全一致）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),           // 路径显示
            Constraint::Min(1),              // 目录列表
            Constraint::Length(help_height), // 帮助提示
        ])
        .split(inner);

    let list_area = chunks[1]; // 目录列表区域

    if y >= list_area.y && y < list_area.y + list_area.height {
        // 计算点击的条目索引（需要考虑滚动偏移）
        let clicked_row = (y - list_area.y) as usize;
        let visible_height = list_area.height as usize;
        let total_items = state.dir_browser.entries.len();
        let selected_idx = state.dir_browser.selected_idx;

        // 计算滚动偏移（与 dir_browser.rs 中的逻辑保持一致）
        let scroll_offset = if total_items <= visible_height {
            0
        } else {
            let half_visible = visible_height / 2;
            if selected_idx < half_visible {
                0
            } else {
                let max_offset = total_items.saturating_sub(visible_height);
                let ideal_offset = selected_idx.saturating_sub(half_visible);
                ideal_offset.min(max_offset)
            }
        };

        let clicked_idx = scroll_offset + clicked_row;

        if clicked_idx < total_items {
            if clicked_idx == state.dir_browser.selected_idx {
                // 点击已选中项：进入目录
                state.dir_browser.enter_selected();
            } else {
                // 点击其他项：选中
                state.dir_browser.selected_idx = clicked_idx;
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// 处理普通模式下的左键点击
fn handle_left_click(
    state: &mut AppState,
    mouse: MouseEvent,
    pty_manager: &PtyManager,
    half_height: u16,
    term_height: u16,
) -> anyhow::Result<bool> {
    let x = mouse.column;
    let y = mouse.row;

    if x < SIDEBAR_WIDTH {
        // 点击侧边栏
        state.focus = FocusArea::Sidebar;

        // 计算"添加项目"行的位置
        // 侧边栏布局：边框(1) + 项目列表/无项目提示 + 空行(1) + 添加项目(1)
        let list_start_y = 2u16; // 边框+标题后的起始行
        let list_len = if state.projects.is_empty() {
            1
        } else {
            state.projects.len()
        };
        let add_project_y = list_start_y + list_len as u16 + 1; // +1 是空行

        if y >= 2 {
            let clicked_idx = (y - 2) as usize;

            if y == add_project_y {
                // 点击了"添加项目"行
                state.enter_browse_mode();
            } else if !state.projects.is_empty() && clicked_idx < state.projects.len() {
                // 点击了项目列表中的项目
                state.active_project_idx = clicked_idx;
            }
        }
    } else if y > 0 && y <= half_height + 1 {
        // 点击 Dev Terminal - 聚焦（只读模式，用于滚动查看 log）
        state.focus = FocusArea::DevTerminal;
    } else if y > half_height + 1 && y < term_height - 1 {
        // 点击 Shell Terminal - 进入交互模式
        start_shell_for_active_project(state, pty_manager)?;
    }

    Ok(true)
}

/// 处理滚轮向上滚动
fn handle_scroll_up(state: &mut AppState) {
    match state.focus {
        FocusArea::Sidebar => {
            // 侧边栏不使用滚轮切换项目（幅度过大）
        }
        FocusArea::DevTerminal => {
            // Dev Terminal 向上滚动（查看更早的 log）
            if let Some(project) = state.active_project_mut() {
                project.dev_scroll_offset = project.dev_scroll_offset.saturating_add(3);
            }
        }
        FocusArea::ShellTerminal => {
            // Shell Terminal 向上滚动（查看历史）
            if let Some(project) = state.active_project_mut() {
                project.shell_scroll_offset = project.shell_scroll_offset.saturating_add(3);
            }
        }
    }
}

/// 处理滚轮向下滚动
fn handle_scroll_down(state: &mut AppState) {
    match state.focus {
        FocusArea::Sidebar => {
            // 侧边栏不使用滚轮切换项目（幅度过大）
        }
        FocusArea::DevTerminal => {
            // Dev Terminal 向下滚动（查看更新的 log）
            if let Some(project) = state.active_project_mut() {
                project.dev_scroll_offset = project.dev_scroll_offset.saturating_sub(3);
            }
        }
        FocusArea::ShellTerminal => {
            // Shell Terminal 向下滚动（回到最新）
            if let Some(project) = state.active_project_mut() {
                project.shell_scroll_offset = project.shell_scroll_offset.saturating_sub(3);
            }
        }
    }
}
