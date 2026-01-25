//! PTY-UI 桥接模块
//! 负责处理 PTY 事件并更新 UI 状态

use super::PtyEvent;
use crate::app::AppState;

/// 处理 PTY 事件
/// 在主循环中调用，处理来自 PTY 任务的事件
pub fn handle_pty_events(state: &mut AppState) {
    // 收集所有待处理的事件
    let mut events = Vec::new();
    while let Ok(event) = state.pty_rx.try_recv() {
        events.push(event);
    }

    // 处理收集到的事件
    for event in events {
        match event {
            PtyEvent::Output { pty_id, data: _ } => {
                // 输出数据已经在 PTY reader 任务中更新到 parser 了
                // 这里可以用于其他处理，如日志记录
                log::trace!("PTY {} output received", pty_id);
            }
            PtyEvent::Exited { pty_id, exit_code } => {
                log::info!("PTY {} exited with code {:?}", pty_id, exit_code);

                // 查找并更新对应的项目状态
                let mut status_msg = None;
                for project in &mut state.projects {
                    if let Some(ref dev_pty) = project.dev_pty {
                        if dev_pty.id == pty_id {
                            project.dev_pty = None;
                            status_msg =
                                Some(format!("Dev server stopped (exit: {:?})", exit_code));
                        }
                    }
                    if let Some(ref shell_pty) = project.shell_pty {
                        if shell_pty.id == pty_id {
                            project.shell_pty = None;
                        }
                    }
                }
                if let Some(msg) = status_msg {
                    state.set_status(&msg);
                }
            }
            PtyEvent::Error { pty_id, message } => {
                log::error!("PTY {} error: {}", pty_id, message);
                state.set_status(&format!("PTY error: {}", message));
            }
        }
    }
}

/// 向当前激活项目的终端发送输入
pub fn send_to_active_terminal(state: &mut AppState, data: &[u8]) -> anyhow::Result<()> {
    use crate::app::FocusArea;

    let focus = state.focus;
    if let Some(project) = state.active_project_mut() {
        match focus {
            FocusArea::DevTerminal => {
                if let Some(ref mut pty) = project.dev_pty {
                    pty.send_input(data)?;
                }
            }
            FocusArea::ShellTerminal => {
                if let Some(ref mut pty) = project.shell_pty {
                    pty.send_input(data)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
