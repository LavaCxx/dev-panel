//! 键盘事件处理模块

use crate::app::{AppMode, AppState, CommandTarget, FocusArea};
use crate::project::Project;
use crate::pty::PtyManager;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use super::command::{execute_command_in_dev, execute_command_in_shell};
use super::helpers::{key_to_bytes, start_shell_for_active_project};

/// 处理键盘事件
pub fn handle_key_event(
    state: &mut AppState,
    key: KeyEvent,
    pty_manager: &PtyManager,
) -> anyhow::Result<bool> {
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
                        // 用户输入时自动回到底部（重置滚动偏移）
                        project.shell_scroll_offset = 0;
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
            // z 切换面板布局（最大化/平分）
            KeyCode::Char('z') => {
                state.toggle_panel_layout();
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
        // z 切换面板布局（最大化/平分）
        KeyCode::Char('z') => {
            state.toggle_panel_layout();
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
            handle_dir_browser_select(state)?;
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

/// 处理目录浏览器中的选择操作
fn handle_dir_browser_select(state: &mut AppState) -> anyhow::Result<()> {
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
        match Project::load(path.clone()) {
            Ok(project) => {
                let msg = match state.language() {
                    crate::i18n::Language::English => {
                        format!("Added project: {}", project.name)
                    }
                    crate::i18n::Language::Chinese => {
                        format!("已添加项目: {}", project.name)
                    }
                };

                // 保存当前浏览目录到配置（用于下次打开时记住位置）
                // 保存项目所在目录而不是当前浏览目录，这样更符合用户预期
                state.config.settings.last_browse_dir = Some(path.to_string_lossy().to_string());

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
    Ok(())
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
    // 滚动步进常量
    const SCROLL_STEP: f32 = 1.0; // 普通滚动步进（每次 1 行）
    const PAGE_STEP: f32 = 10.0; // 翻页步进

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            state.help_scroll.reset(); // 退出时重置滚动位置
            state.exit_mode();
        }
        // j/k 或方向键滚动（使用平滑滚动动画）
        KeyCode::Char('j') | KeyCode::Down => {
            state.help_scroll.scroll_by(SCROLL_STEP);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.help_scroll.scroll_by(-SCROLL_STEP);
        }
        // PageUp/PageDown 快速滚动
        KeyCode::PageDown | KeyCode::Char('d') => {
            state.help_scroll.scroll_by(PAGE_STEP);
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            state.help_scroll.scroll_by(-PAGE_STEP);
        }
        // Home/End 跳到顶部/底部（使用 g/G vim 风格）
        KeyCode::Home | KeyCode::Char('g') => {
            state.help_scroll.set_target(0.0);
        }
        KeyCode::End | KeyCode::Char('G') => {
            // 设置一个足够大的值，渲染时会被 max_scroll 限制
            state.help_scroll.set_target(10000.0);
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
            // 首次启动引导完成后标记为已显示
            if !state.config.settings.first_run_shown {
                state.config.settings.first_run_shown = true;
            }
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
        // y/Y 或 Enter 确认删除
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let idx = state.active_project_idx;
            let msg = state.i18n().project_removed().to_string();
            state.remove_project(idx);
            state.set_status(&msg);
            state.exit_mode();
        }
        // n/N 或 Esc 取消
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            state.exit_mode();
        }
        _ => {}
    }
    Ok(true)
}
