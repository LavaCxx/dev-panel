//! 事件处理模块
//! 负责处理键盘输入和其他 crossterm 事件
//!
//! 交互设计：
//! - Tab: 切换项目
//! - Dev Server: 只显示命令输出，不需要焦点，r 运行命令，s 停止
//! - Interactive Shell: 完全交互式，Enter 进入

pub mod command;
mod helpers;
mod keyboard;
mod mouse;

pub use keyboard::*;
pub use mouse::*;

use crate::app::AppState;
use crate::pty::PtyManager;
use crossterm::event::{Event, KeyEventKind};

/// 侧边栏宽度常量（增加以显示 CPU/内存信息）
pub const SIDEBAR_WIDTH: u16 = 38;

/// 处理 crossterm 事件
pub fn handle_event(
    state: &mut AppState,
    event: Event,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    match event {
        Event::Key(key) => {
            // Windows 会同时发送 Press 和 Release 事件，只处理 Press 事件
            // 避免每次按键被处理两次
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }
            handle_key_event(state, key, pty_manager)
        }
        Event::Mouse(mouse) => handle_mouse_event(state, mouse, pty_manager),
        Event::Resize(_cols, _rows) => {
            // TODO: 处理终端大小变化，通知 PTY resize
            Ok(true)
        }
        _ => Ok(false),
    }
}
