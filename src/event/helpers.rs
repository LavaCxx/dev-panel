//! 事件处理辅助函数模块

use crate::app::{AppState, FocusArea};
use crate::pty::PtyManager;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 为当前项目启动交互式 Shell
pub fn start_shell_for_active_project(
    state: &mut AppState,
    pty_manager: &PtyManager,
) -> anyhow::Result<()> {
    // 检查是否有项目选中
    if state.projects.is_empty() {
        let msg = state.i18n().no_project().to_string();
        state.set_status(&msg);
        return Ok(());
    }

    let (needs_shell, project_path) = {
        if let Some(project) = state.active_project() {
            (project.shell_pty.is_none(), Some(project.path.clone()))
        } else {
            (false, None)
        }
    };

    if let Some(path) = project_path {
        if needs_shell {
            let pty_id = format!("shell-{}", uuid::Uuid::new_v4());
            let pty_tx = state.pty_tx.clone();

            #[cfg(windows)]
            let shell_config = state.config.settings.windows_shell;

            let result = pty_manager.create_shell(
                &pty_id,
                &path,
                24,
                80,
                pty_tx,
                #[cfg(windows)]
                shell_config,
            );

            match result {
                Ok(handle) => {
                    if let Some(project) = state.active_project_mut() {
                        project.shell_pty = Some(handle);
                    }
                    let msg = state.i18n().shell_started().to_string();
                    state.set_status(&msg);
                }
                Err(e) => {
                    state.set_status(&format!("Failed to start shell: {}", e));
                    return Ok(());
                }
            }
        }
        state.focus = FocusArea::ShellTerminal;
    }
    Ok(())
}

/// 将按键事件转换为终端字节序列
pub fn key_to_bytes(key: &KeyEvent) -> Vec<u8> {
    let mut bytes = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let ctrl_char = (c.to_ascii_lowercase() as u8)
                .wrapping_sub(b'a')
                .wrapping_add(1);
            bytes.push(ctrl_char);
            return bytes;
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(s.as_bytes());
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Backspace => bytes.push(0x7f),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        KeyCode::F(n) => {
            let seq = match n {
                1 => b"\x1bOP".as_slice(),
                2 => b"\x1bOQ",
                3 => b"\x1bOR",
                4 => b"\x1bOS",
                5 => b"\x1b[15~",
                6 => b"\x1b[17~",
                7 => b"\x1b[18~",
                8 => b"\x1b[19~",
                9 => b"\x1b[20~",
                10 => b"\x1b[21~",
                11 => b"\x1b[23~",
                12 => b"\x1b[24~",
                _ => &[],
            };
            bytes.extend_from_slice(seq);
        }
        _ => {}
    }

    bytes
}
