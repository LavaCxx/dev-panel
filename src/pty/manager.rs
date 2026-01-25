//! PTY 管理器
//! 负责创建和控制伪终端进程

use super::{PtyEvent, PtyHandle};
use crate::platform::get_default_shell;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

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
    pub fn create_shell(
        &self,
        id: &str,
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
    ) -> anyhow::Result<PtyHandle> {
        let shell = get_default_shell();
        
        // 使用 -l (login shell) 和 -i (interactive) 参数
        // 确保加载完整的 shell 配置（.zshrc, starship 等）
        #[cfg(unix)]
        let args = vec!["-l", "-i"];
        #[cfg(windows)]
        let args = vec![];
        
        self.create_pty(id, &shell, &args, working_dir, rows, cols, event_tx)
    }

    /// 创建执行指定命令的 PTY
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
    pub fn run_shell_command(
        &self,
        id: &str,
        command: &str,
        working_dir: &Path,
        rows: u16,
        cols: u16,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
    ) -> anyhow::Result<PtyHandle> {
        let shell = get_default_shell();

        #[cfg(unix)]
        let args = vec!["-c", command];
        #[cfg(windows)]
        let args = if shell.contains("powershell") {
            vec!["-Command", command]
        } else {
            vec!["/C", command]
        };

        self.create_pty(id, &shell, &args, working_dir, rows, cols, event_tx)
    }

    /// 内部方法：创建 PTY
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
        let pty_system = native_pty_system();
        
        // 创建 PTY 对
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
        
        // 设置关键环境变量以支持彩色输出和 prompt 美化
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        
        // 继承重要的环境变量
        for key in &["HOME", "USER", "SHELL", "PATH", "LANG", "LC_ALL", "STARSHIP_SHELL"] {
            if let Ok(val) = std::env::var(key) {
                cmd.env(key, val);
            }
        }

        // 启动子进程
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
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
