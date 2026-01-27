//! PTY 管理器
//! 负责创建和控制伪终端进程

use super::{PtyEvent, PtyHandle};
use crate::platform::get_default_shell;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

#[cfg(windows)]
use crate::config::WindowsShell;
#[cfg(windows)]
use crate::platform::get_shell_with_config;

/// PTY 管理器
/// 负责创建 PTY 会话并管理其生命周期
pub struct PtyManager {
    // 不存储 pty_system，每次创建时获取
}

impl PtyManager {
    /// 创建新的 PTY 管理器
    pub fn new() -> Self {
        Self {}
    }

    /// 创建交互式 Shell PTY
    /// 使用 login shell 模式以加载完整的 shell 配置（如 starship）
    /// shell_config: Windows 上的 Shell 类型配置
    #[allow(unused_variables)]
    pub fn create_shell(
        &self,
        id: &str,
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
        #[cfg(windows)] shell_config: WindowsShell,
    ) -> anyhow::Result<PtyHandle> {
        #[cfg(unix)]
        let shell = get_default_shell();
        #[cfg(windows)]
        let shell = get_shell_with_config(shell_config);

        // 使用 -l (login shell) 和 -i (interactive) 参数
        // 确保加载完整的 shell 配置（.zshrc, starship 等）
        #[cfg(unix)]
        let args = vec!["-l", "-i"];
        #[cfg(windows)]
        let args = vec![];

        self.create_pty(id, &shell, &args, working_dir, rows, cols, event_tx)
    }

    /// 创建执行指定命令的 PTY
    #[allow(clippy::too_many_arguments)]
    pub fn create_command(
        &self,
        id: &str,
        command: &str,
        args: &[&str],
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
    ) -> anyhow::Result<PtyHandle> {
        self.create_pty(id, command, args, working_dir, rows, cols, event_tx)
    }

    /// 通过 Shell 执行命令字符串
    /// shell_config: Windows 上的 Shell 类型配置
    #[allow(unused_variables)]
    pub fn run_shell_command(
        &self,
        id: &str,
        command: &str,
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
        #[cfg(windows)] shell_config: WindowsShell,
    ) -> anyhow::Result<PtyHandle> {
        #[cfg(unix)]
        let shell = get_default_shell();
        #[cfg(windows)]
        let shell = get_shell_with_config(shell_config);

        #[cfg(unix)]
        let args = vec!["-c", command];
        #[cfg(windows)]
        let args = {
            let shell_lower = shell.to_lowercase();
            if shell_lower.contains("powershell") || shell_lower.contains("pwsh") {
                vec!["-Command", command]
            } else {
                // cmd.exe
                vec!["/C", command]
            }
        };

        self.create_pty(id, &shell, &args, working_dir, rows, cols, event_tx)
    }

    /// 内部方法：创建 PTY
    ///
    /// 注意关于异步/同步边界：
    /// 虽然此方法可能在 tokio 运行时上下文中被调用，但它执行的是短暂的阻塞操作
    /// （PTY 创建通常在几十毫秒内完成）。完全的 spawn_blocking 隔离需要大量重构，
    /// 而当前的方案通过以下措施已足够稳健：
    /// 1. 确保关键环境变量存在
    /// 2. 针对 ConPTY 错误的重试逻辑
    /// 3. 创建前的短暂延迟（给系统资源准备时间）
    #[allow(clippy::too_many_arguments)]
    fn create_pty(
        &self,
        id: &str,
        program: &str,
        args: &[&str],
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: tokio::sync::mpsc::UnboundedSender<PtyEvent>,
    ) -> anyhow::Result<PtyHandle> {
        // Windows: 在创建 PTY 前添加短暂延迟
        // 这给 ConPTY 子系统更多时间准备资源，有助于避免快速连续创建时的竞态条件
        #[cfg(windows)]
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let pty_system = native_pty_system();

        // 创建 PTY 对（Windows 上添加重试逻辑）
        #[cfg(windows)]
        let pair = {
            let mut last_error = None;
            let mut pair_result = None;
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };

            // PTY 创建也可能受到 ConPTY 资源竞争影响，添加重试
            for attempt in 0..3 {
                match pty_system.openpty(size.clone()) {
                    Ok(p) => {
                        pair_result = Some(p);
                        break;
                    }
                    Err(e) => {
                        log::warn!(
                            "PTY openpty attempt {} failed: {} (program: {})",
                            attempt + 1,
                            e,
                            program
                        );
                        last_error = Some(e);
                        // ConPTY 初始化失败时增加等待时间
                        let delay = std::time::Duration::from_millis(50 * (attempt + 1) as u64);
                        std::thread::sleep(delay);
                    }
                }
            }

            match pair_result {
                Some(p) => p,
                None => {
                    let err = last_error.unwrap();
                    log::error!("PTY openpty failed after 3 attempts: {}", err);
                    return Err(err.into());
                }
            }
        };

        #[cfg(unix)]
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // 构建命令
        let mut cmd = CommandBuilder::new(program);
        cmd.args(args);
        cmd.cwd(working_dir);

        // Windows: 继承所有环境变量，避免 0xc0000142 错误
        // 当调用 cmd.env() 时，portable-pty 会从"继承所有"模式切换到"只使用指定"模式
        // 所以我们需要显式继承所有环境变量，然后再覆盖需要的
        //
        // 重要：0xc0000142 (STATUS_DLL_INIT_FAILED) 错误通常是因为子进程无法加载系统 DLL
        // 这要求 SystemRoot、SystemDrive 等关键环境变量必须存在
        #[cfg(windows)]
        {
            // 先继承当前进程的所有环境变量
            for (key, val) in std::env::vars() {
                cmd.env(key, val);
            }

            // 确保关键的 Windows 系统环境变量存在
            // 这些变量对于 conhost.exe 和子进程加载系统 DLL 至关重要
            Self::ensure_critical_windows_env(&mut cmd);
        }

        // 设置关键环境变量以支持彩色输出和 prompt 美化
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        // Unix: 继承重要的环境变量
        #[cfg(unix)]
        {
            for key in &[
                "HOME",
                "USER",
                "SHELL",
                "PATH",
                "LANG",
                "LC_ALL",
                "STARSHIP_SHELL",
            ] {
                if let Ok(val) = std::env::var(key) {
                    cmd.env(key, val);
                }
            }
        }

        // 启动子进程（Windows 上添加重试逻辑，专门处理 0xc0000142 错误）
        #[cfg(windows)]
        let child = {
            // 0xc0000142 作为有符号 32 位整数是 -1073741502
            const STATUS_DLL_INIT_FAILED: i32 = -1073741502;
            let max_retries = 5; // 增加重试次数
            let mut last_error = None;
            let mut child_result = None;

            for attempt in 0..max_retries {
                match pair.slave.spawn_command(cmd.clone()) {
                    Ok(c) => {
                        if attempt > 0 {
                            log::info!(
                                "PTY spawn succeeded on attempt {} (program: {})",
                                attempt + 1,
                                program
                            );
                        }
                        child_result = Some(c);
                        break;
                    }
                    Err(e) => {
                        // 检查是否是 0xc0000142 错误
                        let is_dll_init_failed = e
                            .source()
                            .and_then(|s| s.downcast_ref::<std::io::Error>())
                            .and_then(|io_err| io_err.raw_os_error())
                            .map(|code| code == STATUS_DLL_INIT_FAILED)
                            .unwrap_or(false);

                        log::warn!(
                            "PTY spawn attempt {} failed: {} (program: {}, is_dll_init_failed: {})",
                            attempt + 1,
                            e,
                            program,
                            is_dll_init_failed
                        );
                        last_error = Some(e);

                        // 如果是 DLL 初始化失败，使用更长的退避时间
                        // 这给 ConPTY 和 conhost.exe 更多时间来释放资源
                        let base_delay = if is_dll_init_failed { 100 } else { 50 };
                        let delay =
                            std::time::Duration::from_millis(base_delay * (attempt + 1) as u64);
                        std::thread::sleep(delay);

                        // 如果不是 DLL 初始化错误，不继续重试（可能是其他问题）
                        if !is_dll_init_failed && attempt >= 2 {
                            break;
                        }
                    }
                }
            }

            match child_result {
                Some(c) => c,
                None => {
                    let err = last_error.unwrap();
                    log::error!(
                        "PTY spawn failed after {} attempts: {} (program: {})",
                        max_retries,
                        err,
                        program
                    );
                    return Err(err.into());
                }
            }
        };

        #[cfg(unix)]
        let child = pair.slave.spawn_command(cmd)?;

        // 获取子进程 PID（用于后续发送信号）
        let pid = child.process_id();

        // 获取 reader 和 writer
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        // 创建 PTY 句柄
        let mut handle = PtyHandle::new(id, rows, cols);
        handle.running = true;
        handle.pid = pid;
        handle.writer = Some(writer);

        let parser = Arc::clone(&handle.parser);
        let pty_id = id.to_string();

        // 在独立的阻塞任务中读取 PTY 输出
        // 使用 spawn_blocking 因为 PTY 读取是阻塞 IO
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - 进程已退出
                        let _ = event_tx.send(PtyEvent::Exited {
                            pty_id: pty_id.clone(),
                            exit_code: None,
                        });
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();

                        // 更新解析器状态（使用 blocking_lock 因为在同步线程中）
                        {
                            let mut parser = parser.blocking_lock();
                            parser.process(&data);
                        }

                        // 发送输出事件
                        let _ = event_tx.send(PtyEvent::Output {
                            pty_id: pty_id.clone(),
                            data,
                        });
                    }
                    Err(e) => {
                        let _ = event_tx.send(PtyEvent::Error {
                            pty_id: pty_id.clone(),
                            message: e.to_string(),
                        });
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    /// 调整 PTY 大小
    pub fn resize_pty(
        &self,
        _handle: &mut PtyHandle,
        _rows: u16,
        _cols: u16,
    ) -> anyhow::Result<()> {
        // TODO: 实现 resize 功能
        // 需要保存 master 引用来调用 resize
        Ok(())
    }

    /// Windows 专用：确保关键系统环境变量存在
    /// 这些变量对于 conhost.exe 和子进程加载系统 DLL 至关重要
    /// 如果缺失会导致 0xc0000142 (STATUS_DLL_INIT_FAILED) 错误
    #[cfg(windows)]
    fn ensure_critical_windows_env(cmd: &mut CommandBuilder) {
        // SystemRoot (通常是 C:\Windows) - 最关键的变量
        // 如果缺失，子进程无法找到 system32 目录中的 DLL
        if std::env::var("SystemRoot").is_err() {
            // 尝试从注册表或常见位置获取
            let default_system_root = std::env::var("SYSTEMROOT")
                .or_else(|_| std::env::var("windir"))
                .unwrap_or_else(|_| "C:\\Windows".to_string());
            cmd.env("SystemRoot", &default_system_root);
            log::warn!(
                "SystemRoot not found in env, using fallback: {}",
                default_system_root
            );
        }

        // SystemDrive (通常是 C:) - 用于路径解析
        if std::env::var("SystemDrive").is_err() {
            let default_system_drive = std::env::var("SYSTEMDRIVE")
                .or_else(|_| {
                    std::env::var("SystemRoot")
                        .or_else(|_| std::env::var("SYSTEMROOT"))
                        .map(|sr| sr.chars().take(2).collect())
                })
                .unwrap_or_else(|_| "C:".to_string());
            cmd.env("SystemDrive", &default_system_drive);
            log::warn!(
                "SystemDrive not found in env, using fallback: {}",
                default_system_drive
            );
        }

        // COMSPEC (通常是 C:\Windows\System32\cmd.exe) - 用于 shell 操作
        if std::env::var("COMSPEC").is_err() {
            let system_root = std::env::var("SystemRoot")
                .or_else(|_| std::env::var("SYSTEMROOT"))
                .unwrap_or_else(|_| "C:\\Windows".to_string());
            let default_comspec = format!("{}\\System32\\cmd.exe", system_root);
            cmd.env("COMSPEC", &default_comspec);
            log::warn!(
                "COMSPEC not found in env, using fallback: {}",
                default_comspec
            );
        }

        // PATH - 确保系统路径在 PATH 中
        // 如果 PATH 不包含 System32，某些 DLL 可能无法加载
        if let Ok(path) = std::env::var("PATH") {
            let system_root = std::env::var("SystemRoot")
                .or_else(|_| std::env::var("SYSTEMROOT"))
                .unwrap_or_else(|_| "C:\\Windows".to_string());
            let system32_path = format!("{}\\System32", system_root);

            // 检查 PATH 是否包含 System32（不区分大小写）
            let path_lower = path.to_lowercase();
            let system32_lower = system32_path.to_lowercase();
            if !path_lower.contains(&system32_lower) {
                // 将 System32 添加到 PATH 前面
                let new_path = format!("{};{}", system32_path, path);
                cmd.env("PATH", new_path);
                log::warn!("System32 not found in PATH, prepending it");
            }
        }

        // WINDIR - 某些程序使用这个变量
        if std::env::var("WINDIR").is_err() && std::env::var("windir").is_err() {
            let system_root = std::env::var("SystemRoot")
                .or_else(|_| std::env::var("SYSTEMROOT"))
                .unwrap_or_else(|_| "C:\\Windows".to_string());
            cmd.env("WINDIR", &system_root);
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
