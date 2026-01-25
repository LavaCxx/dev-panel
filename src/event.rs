//! 事件处理模块
//! 负责处理键盘输入和其他 crossterm 事件
//!
//! 交互设计：
//! - Tab: 切换项目
//! - Dev Server: 只显示命令输出，不需要焦点，r 运行命令，s 停止
//! - Interactive Shell: 完全交互式，Enter 进入

use crate::app::{AppMode, AppState, CommandTarget, FocusArea};
use crate::project::Project;
use crate::pty::PtyManager;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::path::PathBuf;

/// 侧边栏宽度常量（增加以显示 CPU/内存信息）
const SIDEBAR_WIDTH: u16 = 38;

/// 处理 crossterm 事件
pub fn handle_event(
    state: &mut AppState,
    event: Event,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    match event {
        Event::Key(key) => handle_key_event(state, key, pty_manager),
        Event::Mouse(mouse) => handle_mouse_event(state, mouse, pty_manager),
        Event::Resize(_cols, _rows) => {
            // TODO: 处理终端大小变化，通知 PTY resize
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// 处理鼠标事件
fn handle_mouse_event(
    state: &mut AppState,
    mouse: MouseEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    // 只处理普通模式下的鼠标事件
    if state.mode != AppMode::Normal {
        return Ok(false);
    }

    let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
    let content_height = term_height.saturating_sub(2);
    let half_height = content_height / 2;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            if x < SIDEBAR_WIDTH {
                // 点击侧边栏 - 选择项目，返回侧边栏焦点
                state.focus = FocusArea::Sidebar;
                if y >= 2 && !state.projects.is_empty() {
                    let clicked_idx = (y - 2) as usize;
                    if clicked_idx < state.projects.len() {
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
        MouseEventKind::ScrollUp => {
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
                    // Shell Terminal 暂不支持滚动
                }
            }
            Ok(true)
        }
        MouseEventKind::ScrollDown => {
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
                    // Shell Terminal 暂不支持滚动
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// 处理键盘事件
fn handle_key_event(
    state: &mut AppState,
    key: KeyEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    // Windows 会同时发送 Press 和 Release 事件，只处理 Press 事件
    // 避免每次按键被处理两次
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    match &state.mode.clone() {
        AppMode::Normal => handle_normal_mode(state, key, pty_manager),
        AppMode::CommandPalette => handle_command_palette_mode(state, key, pty_manager),
        AppMode::AddProject => handle_add_project_mode(state, key),
        AppMode::BrowseDirectory => handle_browse_directory_mode(state, key),
        AppMode::AddCommand => handle_add_command_mode(state, key),
        AppMode::EditAlias => handle_edit_alias_mode(state, key),
        AppMode::Help => handle_help_mode(state, key),
        AppMode::Settings => handle_settings_mode(state, key),
        AppMode::Confirm(_) => handle_confirm_mode(state, key),
    }
}

/// 处理普通模式下的键盘事件
fn handle_normal_mode(
    state: &mut AppState,
    key: KeyEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    // === Shell Terminal 交互模式 ===
    // 只有 Shell Terminal 是完全交互式的
    if state.focus == FocusArea::ShellTerminal {
        match key.code {
            // Esc 返回侧边栏（不关闭 shell）
            KeyCode::Esc => {
                state.focus = FocusArea::Sidebar;
                return Ok(true);
            }
            // 其他所有按键转发给 Shell PTY
            _ => {
                let data = key_to_bytes(&key);
                if !data.is_empty() {
                    if let Some(project) = state.active_project_mut() {
                        if let Some(ref mut pty) = project.shell_pty {
                            pty.send_input(&data)?;
                        }
                    }
                }
                return Ok(true);
            }
        }
    }

    // === Dev Terminal 只读模式（查看 log）===
    if state.focus == FocusArea::DevTerminal {
        match key.code {
            // Esc 返回侧边栏
            KeyCode::Esc => {
                // 重置滚动位置
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = 0;
                }
                state.focus = FocusArea::Sidebar;
                return Ok(true);
            }
            // j/k 或方向键滚动
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = project.dev_scroll_offset.saturating_sub(1);
                }
                return Ok(true);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = project.dev_scroll_offset.saturating_add(1);
                }
                return Ok(true);
            }
            // Page Up/Down 快速滚动
            KeyCode::PageUp => {
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = project.dev_scroll_offset.saturating_add(10);
                }
                return Ok(true);
            }
            KeyCode::PageDown => {
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = project.dev_scroll_offset.saturating_sub(10);
                }
                return Ok(true);
            }
            // Home 跳到最新
            KeyCode::Home => {
                if let Some(project) = state.active_project_mut() {
                    project.dev_scroll_offset = 0;
                }
                return Ok(true);
            }
            _ => {
                // 其他按键返回侧边栏
                state.focus = FocusArea::Sidebar;
                return Ok(true);
            }
        }
    }

    // === 侧边栏模式（全局快捷键）===
    // Dev Terminal 不需要焦点，所有操作都在侧边栏完成

    // Ctrl+C 或 Ctrl+Q 退出程序
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => {
                state.should_quit = true;
                return Ok(true);
            }
            _ => {}
        }
    }

    match key.code {
        // 退出
        KeyCode::Char('q') => {
            state.should_quit = true;
        }
        // 帮助
        KeyCode::Char('?') => {
            state.mode = AppMode::Help;
        }
        // Tab 切换项目
        KeyCode::Tab => {
            state.select_next_project();
        }
        KeyCode::BackTab => {
            state.select_prev_project();
        }
        // j/k 或方向键切换项目
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next_project();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_prev_project();
        }
        // r 打开命令面板（在 Dev Terminal 运行）
        KeyCode::Char('r') => {
            if state.active_project().is_some() {
                state.enter_command_palette(CommandTarget::DevTerminal);
            } else {
                let msg = state.i18n().no_project().to_string();
                state.set_status(&msg);
            }
        }
        // R (Shift+r) 打开命令面板（在 Interactive Shell 运行）
        KeyCode::Char('R') => {
            if state.active_project().is_some() {
                state.enter_command_palette(CommandTarget::ShellTerminal);
            } else {
                let msg = state.i18n().no_project().to_string();
                state.set_status(&msg);
            }
        }
        // a 添加项目（进入目录浏览器）
        KeyCode::Char('a') => {
            state.enter_browse_mode();
        }
        // c 添加自定义命令
        KeyCode::Char('c') => {
            if state.active_project().is_some() {
                state.mode = AppMode::AddCommand;
                state.input_buffer.clear();
            } else {
                let msg = state.i18n().no_project().to_string();
                state.set_status(&msg);
            }
        }
        // s 停止 Dev Server
        KeyCode::Char('s') => {
            if let Some(project) = state.active_project_mut() {
                if project.dev_pty.is_some() {
                    project.dev_pty = None;
                    project.mark_dev_stopped();
                    let msg = state.i18n().dev_stopped().to_string();
                    state.set_status(&msg);
                }
            }
        }
        // Enter 进入 Shell Terminal
        KeyCode::Enter => {
            start_shell_for_active_project(state, pty_manager)?;
        }
        // d 删除项目
        KeyCode::Char('d') => {
            if state.active_project().is_some() {
                let msg = state.i18n().delete_project().to_string();
                state.mode = AppMode::Confirm(msg);
            }
        }
        // x 发送 Ctrl+C 给 Dev Server（无需切换焦点）
        KeyCode::Char('x') => {
            if let Some(project) = state.active_project_mut() {
                if let Some(ref mut pty) = project.dev_pty {
                    pty.send_input(&[0x03])?; // Ctrl+C
                    let msg = state.i18n().sent_interrupt().to_string();
                    state.set_status(&msg);
                }
            }
        }
        // p 暂停/恢复 Dev Server（冻结进程节省资源）
        KeyCode::Char('p') => {
            if let Some(project) = state.active_project_mut() {
                if let Some(ref mut pty) = project.dev_pty {
                    let was_suspended = pty.suspended;
                    match pty.toggle_suspend() {
                        Ok(true) => {
                            let msg = if was_suspended {
                                state.i18n().process_resumed().to_string()
                            } else {
                                state.i18n().process_suspended().to_string()
                            };
                            state.set_status(&msg);
                        }
                        Ok(false) => {
                            let msg = state.i18n().suspend_not_supported().to_string();
                            state.set_status(&msg);
                        }
                        Err(e) => {
                            state.set_status(&format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        // 数字键 1-9 快速切换项目
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            if idx < state.projects.len() {
                state.active_project_idx = idx;
            }
        }
        // , 打开设置
        KeyCode::Char(',') => {
            state.mode = AppMode::Settings;
            state.settings_idx = 0;
        }
        // e 编辑项目别名
        KeyCode::Char('e') => {
            if let Some(project) = state.active_project() {
                // 预填充当前别名
                state.input_buffer = project.alias.clone().unwrap_or_default();
                state.mode = AppMode::EditAlias;
            } else {
                let msg = state.i18n().no_project().to_string();
                state.set_status(&msg);
            }
        }
        _ => {}
    }

    Ok(true)
}

/// 处理命令面板模式
fn handle_command_palette_mode(
    state: &mut AppState,
    key: KeyEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.exit_mode();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.command_palette_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.command_palette_prev();
        }
        KeyCode::Enter => {
            // 根据 command_target 决定执行位置
            match state.command_target {
                CommandTarget::DevTerminal => {
                    execute_command_in_dev(state, pty_manager)?;
                }
                CommandTarget::ShellTerminal => {
                    execute_command_in_shell(state, pty_manager)?;
                }
            }
            state.exit_mode();
        }
        _ => {}
    }
    Ok(true)
}

/// 处理添加项目模式
fn handle_add_project_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.exit_mode();
        }
        KeyCode::Enter => {
            let path = PathBuf::from(&state.input_buffer);
            if path.exists() && path.join("package.json").exists() {
                match Project::load(path) {
                    Ok(project) => {
                        state.set_status(&format!("Added project: {}", project.name));
                        state.add_project(project);
                    }
                    Err(e) => {
                        state.set_status(&format!("Failed to load project: {}", e));
                    }
                }
            } else {
                state.set_status("Invalid project path or missing package.json");
            }
            state.exit_mode();
        }
        KeyCode::Char(c) => {
            state.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            state.input_buffer.pop();
        }
        _ => {}
    }
    Ok(true)
}

/// 处理目录浏览模式
fn handle_browse_directory_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        // Esc 取消
        KeyCode::Esc => {
            state.exit_mode();
        }
        // 上下导航
        KeyCode::Char('j') | KeyCode::Down => {
            state.dir_browser.select_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.dir_browser.select_prev();
        }
        // Enter 进入目录
        KeyCode::Enter => {
            state.dir_browser.enter_selected();
        }
        // Backspace 返回上级目录
        KeyCode::Backspace => {
            state.dir_browser.go_up();
        }
        // 空格：选择目录作为项目
        KeyCode::Char(' ') => {
            // 优先检查选中的目录，否则检查当前目录
            let path_to_add = if let Some(entry) = state.dir_browser.selected_entry() {
                if entry.has_package_json {
                    // 选中的目录有 package.json
                    Some(entry.path.clone())
                } else {
                    // 选中的目录没有 package.json，检查当前目录
                    let current = state.dir_browser.current_dir.clone();
                    if current.join("package.json").exists() {
                        Some(current)
                    } else {
                        None
                    }
                }
            } else {
                // 没有选中任何条目，检查当前目录
                let current = state.dir_browser.current_dir.clone();
                if current.join("package.json").exists() {
                    Some(current)
                } else {
                    None
                }
            };

            if let Some(path) = path_to_add {
                match Project::load(path) {
                    Ok(project) => {
                        let msg = match state.language() {
                            crate::i18n::Language::English => {
                                format!("Added project: {}", project.name)
                            }
                            crate::i18n::Language::Chinese => {
                                format!("已添加项目: {}", project.name)
                            }
                        };
                        state.add_project(project);
                        state.set_status(&msg);
                        state.exit_mode();
                    }
                    Err(e) => {
                        state.set_status(&format!("Error: {}", e));
                    }
                }
            } else {
                let msg = match state.language() {
                    crate::i18n::Language::English => "No package.json found",
                    crate::i18n::Language::Chinese => "未找到 package.json",
                };
                state.set_status(msg);
            }
        }
        // . 切换隐藏文件
        KeyCode::Char('.') => {
            state.dir_browser.toggle_hidden();
        }
        // ~ 跳转到主目录
        KeyCode::Char('~') => {
            if let Some(home) = dirs::home_dir() {
                state.dir_browser.current_dir = home;
                state.dir_browser.refresh();
            }
        }
        _ => {}
    }
    Ok(true)
}

/// 处理添加命令模式
fn handle_add_command_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.exit_mode();
        }
        KeyCode::Enter => {
            let input = state.input_buffer.clone();
            if let Some((name, command)) = input.split_once(':') {
                let name = name.trim().to_string();
                let command = command.trim().to_string();
                if let Some(project) = state.active_project_mut() {
                    project.add_custom_command(&name, &command);
                }
                state.set_status(&format!("Added command: {}", name));
            } else {
                state.set_status("Format: name:command");
            }
            state.exit_mode();
        }
        KeyCode::Char(c) => {
            state.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            state.input_buffer.pop();
        }
        _ => {}
    }
    Ok(true)
}

/// 处理编辑别名模式
fn handle_edit_alias_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.exit_mode();
        }
        KeyCode::Enter => {
            let alias = state.input_buffer.clone();
            if let Some(project) = state.active_project_mut() {
                project.set_alias(if alias.is_empty() { None } else { Some(alias) });
            }
            let msg = state.i18n().alias_set().to_string();
            state.set_status(&msg);
            state.exit_mode();
        }
        KeyCode::Char(c) => {
            state.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            state.input_buffer.pop();
        }
        _ => {}
    }
    Ok(true)
}

/// 处理帮助模式
fn handle_help_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            state.exit_mode();
        }
        _ => {}
    }
    Ok(true)
}

/// 处理设置模式
fn handle_settings_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    use crate::ui::SettingItem;

    match key.code {
        KeyCode::Esc | KeyCode::Char(',') => {
            state.exit_mode();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.settings_idx = (state.settings_idx + 1) % SettingItem::count();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.settings_idx == 0 {
                state.settings_idx = SettingItem::count() - 1;
            } else {
                state.settings_idx -= 1;
            }
        }
        KeyCode::Enter => {
            // 切换当前设置项
            let items = SettingItem::all();
            if let Some(item) = items.get(state.settings_idx) {
                match item {
                    SettingItem::Language => {
                        state.toggle_language();
                    }
                    #[cfg(windows)]
                    SettingItem::WindowsShell => {
                        state.toggle_windows_shell();
                    }
                }
            }
        }
        _ => {}
    }
    Ok(true)
}

/// 处理确认对话框模式
fn handle_confirm_mode(state: &mut AppState, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let idx = state.active_project_idx;
            let msg = state.i18n().project_removed().to_string();
            state.remove_project(idx);
            state.set_status(&msg);
            state.exit_mode();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            state.exit_mode();
        }
        _ => {}
    }
    Ok(true)
}

/// 为当前项目启动交互式 Shell
fn start_shell_for_active_project(
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

/// 在 Dev Terminal 执行命令（覆盖现有进程）
fn execute_command_in_dev(state: &mut AppState, pty_manager: &PtyManager) -> anyhow::Result<()> {
    use crate::project::{detect_package_manager, CommandType};

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
fn execute_command_in_shell(state: &mut AppState, pty_manager: &PtyManager) -> anyhow::Result<()> {
    use crate::project::{detect_package_manager, CommandType};

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

/// 将按键事件转换为终端字节序列
fn key_to_bytes(key: &KeyEvent) -> Vec<u8> {
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
