//! DevPanel - 多项目终端管理 TUI 工具
//!
//! 在单窗口内管理多个前端项目，支持切割视图（Dev Server 日志 + 交互式 Shell）
//! 完美兼容 macOS 和 Windows

mod app;
mod config;
mod event;
mod i18n;
mod platform;
mod project;
mod pty;
mod ui;

use app::AppState;
use config::{get_config_path, load_config, save_config, AppConfig};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use project::Project;
use pty::{handle_pty_events, PtyManager};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use ui::{draw_ui, Theme};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TUI 应用中禁用控制台日志（会干扰界面）
    // 如需调试，可以设置 RUST_LOG=off 或输出到文件
    
    // 加载配置
    let config_path = get_config_path();
    let config = load_config(&config_path).unwrap_or_default();

    // 运行应用
    let result = run_app(config).await;

    // 应用退出后保存配置会在 run_app 内部处理
    result
}

/// 运行主应用
async fn run_app(config: AppConfig) -> anyhow::Result<()> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 初始化应用状态
    let mut state = AppState::new(config.clone());

    // 从配置加载项目
    for project_config in &config.projects {
        let path = PathBuf::from(&project_config.path);
        if path.exists() {
            match Project::load(path) {
                Ok(mut project) => {
                    project.alias = project_config.alias.clone();
                    project.custom_commands = project_config.custom_commands.clone();
                    state.add_project(project);
                }
                Err(e) => {
                    log::warn!("Failed to load project {}: {}", project_config.path, e);
                }
            }
        }
    }

    // 初始化主题
    let theme = Theme::default();

    // 初始化 PTY 管理器
    let pty_manager = PtyManager::new();

    // 创建事件流
    let mut event_stream = EventStream::new();

    // 主循环
    loop {
        // 处理 PTY 事件
        handle_pty_events(&mut state);

        // 渲染 UI
        terminal.draw(|frame| {
            draw_ui(frame, &state, &theme);
        })?;

        // 检查是否应该退出
        if state.should_quit {
            break;
        }

        // 等待事件（带超时，以便定期刷新 PTY 输出）
        tokio::select! {
            maybe_event = event_stream.next() => {
                if let Some(Ok(evt)) = maybe_event {
                    event::handle_event(&mut state, evt, &pty_manager)?;
                }
            }
            // 每 50ms 刷新一次，确保 PTY 输出及时更新
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {}
        }
    }

    // 保存配置
    let mut new_config = state.config.clone();
    new_config.projects = state
        .projects
        .iter()
        .map(|p| p.into())
        .collect();
    let config_path = get_config_path();
    let _ = save_config(&new_config, &config_path);

    // 恢复终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}
