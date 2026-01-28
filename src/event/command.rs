//! 命令执行模块
//! 负责在 Dev Terminal 和 Shell Terminal 中执行命令

use crate::app::{AppState, PendingDevCommand, PtyCleanupState};
use crate::project::{detect_package_manager, CommandType};
use crate::pty::PtyManager;

/// 请求在 Dev Terminal 执行命令
/// 如果有旧进程正在运行，会启动资源释放流程并缓存命令
/// 返回 true 表示命令已开始执行，false 表示命令已缓存等待执行
pub fn request_execute_in_dev(state: &mut AppState) -> bool {
    let command_idx = state.command_palette_idx;
    let project_idx = state.active_project_idx;

    // 检查是否已经在等待资源释放
    if state.is_waiting_for_cleanup() {
        // 更新待执行的命令（替换之前缓存的）
        state.pending_dev_command = Some(PendingDevCommand {
            command_idx,
            project_idx,
        });
        log::info!("Command queued, waiting for PTY cleanup");
        return false;
    }

    // 检查是否有旧进程需要清理
    let old_pid = state
        .active_project()
        .and_then(|p| p.dev_pty.as_ref())
        .and_then(|pty| pty.pid);

    if let Some(pid) = old_pid {
        // 有旧进程，启动清理流程
        // 先停止旧进程
        if let Some(project) = state.active_project_mut() {
            project.dev_pty = None; // 触发 Drop，调用 kill()
            project.mark_dev_stopped();
        }

        // 设置清理状态和待执行命令
        state.pty_cleanup = Some(PtyCleanupState::new(pid, project_idx));
        state.pending_dev_command = Some(PendingDevCommand {
            command_idx,
            project_idx,
        });

        log::info!("Started PTY cleanup for pid {}, command queued", pid);

        // 更新状态提示
        match state.language() {
            crate::i18n::Language::English => state.set_status("Releasing resources..."),
            crate::i18n::Language::Chinese => state.set_status("正在释放资源..."),
        }

        return false;
    }

    // 没有旧进程，可以直接执行
    true
}

/// 实际执行 Dev 命令（内部使用）
/// 在确认资源已释放后调用
pub fn do_execute_command_in_dev(
    state: &mut AppState,
    pty_manager: &PtyManager,
    command_idx: usize,
) -> anyhow::Result<()> {
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

/// 在 Dev Terminal 执行命令（覆盖现有进程）
/// 这是原有的直接执行版本，用于没有旧进程时的情况
pub fn execute_command_in_dev(
    state: &mut AppState,
    pty_manager: &PtyManager,
) -> anyhow::Result<()> {
    let command_idx = state.command_palette_idx;
    do_execute_command_in_dev(state, pty_manager, command_idx)
}

/// 执行待处理的 Dev 命令（如果有）
/// 在资源释放后由主循环调用
pub fn execute_pending_dev_command(
    state: &mut AppState,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    if let Some(pending) = state.pending_dev_command.take() {
        // 确保项目索引仍然有效
        if pending.project_idx == state.active_project_idx {
            log::info!("Executing pending command (idx: {})", pending.command_idx);
            do_execute_command_in_dev(state, pty_manager, pending.command_idx)?;
            return Ok(true);
        } else {
            log::warn!(
                "Pending command project mismatch: {} vs {}",
                pending.project_idx,
                state.active_project_idx
            );
        }
    }
    Ok(false)
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
