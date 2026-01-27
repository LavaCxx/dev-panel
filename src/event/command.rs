//! 命令执行模块
//! 负责在 Dev Terminal 和 Shell Terminal 中执行命令

use crate::app::AppState;
use crate::project::{detect_package_manager, CommandType};
use crate::pty::PtyManager;

/// 在 Dev Terminal 执行命令（覆盖现有进程）
pub fn execute_command_in_dev(
    state: &mut AppState,
    pty_manager: &PtyManager,
) -> anyhow::Result<()> {
    let command_idx = state.command_palette_idx;

    let command_info = {
        if let Some(project) = state.active_project() {
            let commands = project.get_all_commands();
            commands.get(command_idx).map(|cmd| {
                let working_dir = project.path.clone();
                let full_command = match cmd.cmd_type {
                    CommandType::NpmScript => {
                        let pm = detect_package_manager(&working_dir);
                        format!("{} {}", pm.run_prefix(), cmd.name)
                    }
                    CommandType::RawShell => cmd.command.clone(),
                };
                (working_dir, full_command, cmd.name.clone())
            })
        } else {
            None
        }
    };

    if let Some((working_dir, full_command, cmd_name)) = command_info {
        // 先停止现有的 Dev 进程
        if let Some(project) = state.active_project_mut() {
            project.dev_pty = None;
        }

        let pty_id = format!("dev-{}", uuid::Uuid::new_v4());
        let pty_tx = state.pty_tx.clone();

        #[cfg(windows)]
        let shell_config = state.config.settings.windows_shell;

        let handle = pty_manager.run_shell_command(
            &pty_id,
            &full_command,
            &working_dir,
            24,
            80,
            pty_tx,
            #[cfg(windows)]
            shell_config,
        )?;

        if let Some(project) = state.active_project_mut() {
            project.dev_pty = Some(handle);
            project.mark_dev_started();
        }
        state.set_status(&format!("Running: {}", cmd_name));
    }
    Ok(())
}

/// 在 Interactive Shell 执行命令
pub fn execute_command_in_shell(
    state: &mut AppState,
    pty_manager: &PtyManager,
) -> anyhow::Result<()> {
    use crate::app::FocusArea;

    let command_idx = state.command_palette_idx;

    // 获取命令信息
    let command_info = {
        if let Some(project) = state.active_project() {
            let commands = project.get_all_commands();
            commands.get(command_idx).map(|cmd| {
                let working_dir = project.path.clone();
                let full_command = match cmd.cmd_type {
                    CommandType::NpmScript => {
                        let pm = detect_package_manager(&working_dir);
                        format!("{} {}", pm.run_prefix(), cmd.name)
                    }
                    CommandType::RawShell => cmd.command.clone(),
                };
                (working_dir, full_command, cmd.name.clone())
            })
        } else {
            None
        }
    };

    if let Some((project_path, full_command, cmd_name)) = command_info {
        // 检查是否需要先启动 Shell
        let needs_shell = {
            if let Some(project) = state.active_project() {
                project.shell_pty.is_none()
            } else {
                false
            }
        };

        // 如果 Shell 不存在，先启动
        if needs_shell {
            let pty_id = format!("shell-{}", uuid::Uuid::new_v4());
            let pty_tx = state.pty_tx.clone();

            #[cfg(windows)]
            let shell_config = state.config.settings.windows_shell;

            let result = pty_manager.create_shell(
                &pty_id,
                &project_path,
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
                }
                Err(e) => {
                    state.set_status(&format!("Failed to start shell: {}", e));
                    return Ok(());
                }
            }
        }

        // 向 Shell 发送命令（加上回车执行）
        if let Some(project) = state.active_project_mut() {
            if let Some(ref mut pty) = project.shell_pty {
                // 发送命令文本 + 回车
                let command_with_newline = format!("{}\r", full_command);
                pty.send_input(command_with_newline.as_bytes())?;
            }
        }

        // 切换焦点到 Shell Terminal
        state.focus = FocusArea::ShellTerminal;
        state.set_status(&format!("Shell: {}", cmd_name));
    }
    Ok(())
}
