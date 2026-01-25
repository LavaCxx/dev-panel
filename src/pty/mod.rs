//! PTY (伪终端) 管理模块
//! 负责创建和管理伪终端会话

#![allow(dead_code)]

mod bridge;
mod manager;

pub use bridge::*;
pub use manager::*;

use std::sync::Arc;
use tokio::sync::Mutex;

/// PTY 句柄
/// 包含对 PTY 会话的引用和控制
pub struct PtyHandle {
    /// PTY 会话 ID
    pub id: String,
    /// 是否正在运行
    pub running: bool,
    /// 是否已暂停（冻结）
    pub suspended: bool,
    /// 子进程 PID（用于发送信号）
    pub pid: Option<u32>,
    /// 终端解析器状态（用于 tui-term 渲染）
    pub parser: Arc<Mutex<vt100::Parser>>,
    /// PTY writer（用于发送输入）
    pub writer: Option<Box<dyn std::io::Write + Send>>,
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyHandle")
            .field("id", &self.id)
            .field("running", &self.running)
            .field("suspended", &self.suspended)
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl PtyHandle {
    /// 创建新的 PTY 句柄
    pub fn new(id: &str, rows: u16, cols: u16) -> Self {
        Self {
            id: id.to_string(),
            running: false,
            suspended: false,
            pid: None,
            parser: Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000))),
            writer: None,
        }
    }

    /// 向 PTY 发送输入
    pub fn send_input(&mut self, data: &[u8]) -> anyhow::Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.write_all(data)?;
            writer.flush()?;
        }
        Ok(())
    }

    /// 暂停（冻结）进程
    /// 使用 SIGSTOP 信号暂停整个进程组，节省系统资源
    #[cfg(unix)]
    pub fn suspend(&mut self) -> anyhow::Result<bool> {
        if let Some(pid) = self.pid {
            if !self.suspended {
                // 使用负的 PID 发送信号给整个进程组
                // 这样可以暂停 shell 及其所有子进程（如 node dev server）
                unsafe {
                    let pgid = libc::getpgid(pid as i32);
                    let target_pid = if pgid > 0 { -pgid } else { -(pid as i32) };

                    if libc::kill(target_pid, libc::SIGSTOP) == 0 {
                        self.suspended = true;
                        return Ok(true);
                    }

                    // 如果进程组信号失败，尝试直接发送给进程
                    if libc::kill(pid as i32, libc::SIGSTOP) == 0 {
                        self.suspended = true;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// 恢复（解冻）进程
    /// 使用 SIGCONT 信号恢复整个进程组
    #[cfg(unix)]
    pub fn resume(&mut self) -> anyhow::Result<bool> {
        if let Some(pid) = self.pid {
            if self.suspended {
                // 使用负的 PID 发送信号给整个进程组
                unsafe {
                    let pgid = libc::getpgid(pid as i32);
                    let target_pid = if pgid > 0 { -pgid } else { -(pid as i32) };

                    if libc::kill(target_pid, libc::SIGCONT) == 0 {
                        self.suspended = false;
                        return Ok(true);
                    }

                    // 如果进程组信号失败，尝试直接发送给进程
                    if libc::kill(pid as i32, libc::SIGCONT) == 0 {
                        self.suspended = false;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// 切换暂停/恢复状态
    #[cfg(unix)]
    pub fn toggle_suspend(&mut self) -> anyhow::Result<bool> {
        if self.suspended {
            self.resume()
        } else {
            self.suspend()
        }
    }

    /// Windows 上暂停进程（目前不支持）
    #[cfg(windows)]
    pub fn suspend(&mut self) -> anyhow::Result<bool> {
        // Windows 上暂停进程需要使用 NtSuspendProcess，较为复杂
        // 暂不实现
        Ok(false)
    }

    #[cfg(windows)]
    pub fn resume(&mut self) -> anyhow::Result<bool> {
        Ok(false)
    }

    #[cfg(windows)]
    pub fn toggle_suspend(&mut self) -> anyhow::Result<bool> {
        Ok(false)
    }
}

/// PTY 事件
/// 用于在异步任务和主线程之间传递 PTY 相关事件
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// 收到输出数据
    Output { pty_id: String, data: Vec<u8> },
    /// PTY 进程已退出
    Exited {
        pty_id: String,
        exit_code: Option<i32>,
    },
    /// 错误发生
    Error { pty_id: String, message: String },
}
