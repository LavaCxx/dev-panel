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

    /// Windows 上暂停进程
    /// 通过遍历整个进程树并暂停所有线程来实现
    /// 这样可以确保子进程（如 npm 启动的 node.js）也被暂停
    #[cfg(windows)]
    pub fn suspend(&mut self) -> anyhow::Result<bool> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        };
        use windows::Win32::System::Threading::{OpenThread, SuspendThread, THREAD_SUSPEND_RESUME};

        if let Some(pid) = self.pid {
            if !self.suspended {
                unsafe {
                    // 获取整个进程树（包括所有子进程、孙进程等）
                    let process_tree = get_process_tree(pid)?;

                    // 创建线程快照
                    let thread_snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)?;

                    let mut entry = THREADENTRY32 {
                        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                        ..Default::default()
                    };

                    let mut suspended_count = 0;

                    if Thread32First(thread_snapshot, &mut entry).is_ok() {
                        loop {
                            // 暂停属于进程树中任何进程的线程
                            if process_tree.contains(&entry.th32OwnerProcessID) {
                                if let Ok(thread_handle) =
                                    OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID)
                                {
                                    if SuspendThread(thread_handle) != u32::MAX {
                                        suspended_count += 1;
                                    }
                                    let _ = CloseHandle(thread_handle);
                                }
                            }

                            if Thread32Next(thread_snapshot, &mut entry).is_err() {
                                break;
                            }
                        }
                    }

                    let _ = CloseHandle(thread_snapshot);

                    if suspended_count > 0 {
                        self.suspended = true;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Windows 上恢复进程
    /// 恢复整个进程树中的所有线程
    #[cfg(windows)]
    pub fn resume(&mut self) -> anyhow::Result<bool> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        };
        use windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

        if let Some(pid) = self.pid {
            if self.suspended {
                unsafe {
                    // 获取整个进程树（包括所有子进程、孙进程等）
                    let process_tree = get_process_tree(pid)?;

                    // 创建线程快照
                    let thread_snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)?;

                    let mut entry = THREADENTRY32 {
                        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                        ..Default::default()
                    };

                    let mut resumed_count = 0;

                    if Thread32First(thread_snapshot, &mut entry).is_ok() {
                        loop {
                            // 恢复属于进程树中任何进程的线程
                            if process_tree.contains(&entry.th32OwnerProcessID) {
                                if let Ok(thread_handle) =
                                    OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID)
                                {
                                    if ResumeThread(thread_handle) != u32::MAX {
                                        resumed_count += 1;
                                    }
                                    let _ = CloseHandle(thread_handle);
                                }
                            }

                            if Thread32Next(thread_snapshot, &mut entry).is_err() {
                                break;
                            }
                        }
                    }

                    let _ = CloseHandle(thread_snapshot);

                    if resumed_count > 0 {
                        self.suspended = false;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Windows 上切换暂停/恢复状态
    #[cfg(windows)]
    pub fn toggle_suspend(&mut self) -> anyhow::Result<bool> {
        if self.suspended {
            self.resume()
        } else {
            self.suspend()
        }
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

/// Windows 辅助函数：获取整个进程树
/// 递归查找指定 PID 的所有子进程、孙进程等
/// 返回包含根进程及所有后代进程的 PID 集合
#[cfg(windows)]
unsafe fn get_process_tree(root_pid: u32) -> anyhow::Result<std::collections::HashSet<u32>> {
    use std::collections::HashSet;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };

    let mut result = HashSet::new();
    result.insert(root_pid);

    // 创建进程快照
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

    let mut entry = PROCESSENTRY32 {
        dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
        ..Default::default()
    };

    // 构建父子关系映射
    // Key: 父进程 PID, Value: 子进程 PID 列表
    let mut parent_to_children: std::collections::HashMap<u32, Vec<u32>> =
        std::collections::HashMap::new();

    if Process32First(snapshot, &mut entry).is_ok() {
        loop {
            parent_to_children
                .entry(entry.th32ParentProcessID)
                .or_default()
                .push(entry.th32ProcessID);

            if Process32Next(snapshot, &mut entry).is_err() {
                break;
            }
        }
    }

    let _ = CloseHandle(snapshot);

    // 使用 BFS 遍历整个进程树
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(root_pid);

    while let Some(current_pid) = queue.pop_front() {
        if let Some(children) = parent_to_children.get(&current_pid) {
            for &child_pid in children {
                if result.insert(child_pid) {
                    queue.push_back(child_pid);
                }
            }
        }
    }

    Ok(result)
}
