//! PTY (伪终端) 管理模块
//! 负责创建和管理伪终端会话

#![allow(dead_code)]

mod bridge;
mod manager;

pub use bridge::*;
pub use manager::*;

use std::sync::Arc;
use tokio::sync::Mutex;

/// 进程资源使用信息
#[derive(Debug, Clone, Default)]
pub struct ProcessResourceUsage {
    /// CPU 使用率（百分比，0.0-100.0+）
    pub cpu_percent: f32,
    /// 内存使用量（字节）
    pub memory_bytes: u64,
}

impl ProcessResourceUsage {
    /// 格式化内存大小为人类可读格式
    pub fn format_memory(&self) -> String {
        let bytes = self.memory_bytes;
        if bytes == 0 {
            "--".to_string() // 数据未就绪
        } else if bytes < 1024 {
            format!("{}B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1}K", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// 格式化 CPU 使用率
    /// 注意：多核系统上进程树的 CPU 总和可能超过 100%
    pub fn format_cpu(&self) -> String {
        if self.cpu_percent == 0.0 && self.memory_bytes == 0 {
            "--".to_string() // 数据未就绪
        } else if self.cpu_percent >= 1000.0 {
            // 极端情况：超过 1000%（很多核心满载）
            format!("{:.0}%", self.cpu_percent)
        } else if self.cpu_percent >= 100.0 {
            // 超过 100%（多个子进程或多核满载）
            format!("{:.0}%", self.cpu_percent)
        } else if self.cpu_percent >= 10.0 {
            // 两位数
            format!("{:.0}%", self.cpu_percent)
        } else {
            // 个位数，保留一位小数
            format!("{:.1}%", self.cpu_percent)
        }
    }
}

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
    /// 进程资源使用信息（包括所有子进程的总和）
    pub resource_usage: ProcessResourceUsage,
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyHandle")
            .field("id", &self.id)
            .field("running", &self.running)
            .field("suspended", &self.suspended)
            .field("pid", &self.pid)
            .field("resource_usage", &self.resource_usage)
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
            resource_usage: ProcessResourceUsage::default(),
        }
    }

    /// 更新进程资源使用信息
    /// 会统计整个进程树（包括所有子进程）的资源使用
    pub fn update_resource_usage(&mut self, system: &sysinfo::System) {
        if let Some(pid) = self.pid {
            let pid = sysinfo::Pid::from_u32(pid);

            // 收集整个进程树的资源使用
            let mut total_cpu: f32 = 0.0;
            let mut total_memory: u64 = 0;

            // 获取所有子进程的 PID
            let child_pids = collect_process_tree(system, pid);

            // 如果找到了进程，统计资源使用
            if !child_pids.is_empty() {
                for child_pid in child_pids {
                    if let Some(process) = system.process(child_pid) {
                        total_cpu += process.cpu_usage();
                        total_memory += process.memory();
                    }
                }
            } else {
                // 如果进程树为空，尝试直接获取根进程的信息
                // 这可以处理某些 PTY 实现中 PID 跟踪不准确的情况
                if let Some(process) = system.process(pid) {
                    total_cpu = process.cpu_usage();
                    total_memory = process.memory();
                }
            }

            self.resource_usage.cpu_percent = total_cpu;
            self.resource_usage.memory_bytes = total_memory;
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

/// 辅助函数：使用 sysinfo 收集整个进程树
/// 递归查找指定 PID 的所有子进程、孙进程等
/// 返回包含根进程及所有后代进程的 PID 列表
fn collect_process_tree(system: &sysinfo::System, root_pid: sysinfo::Pid) -> Vec<sysinfo::Pid> {
    use std::collections::{HashSet, VecDeque};

    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // 首先检查根进程是否存在
    let root_exists = system.process(root_pid).is_some();

    if root_exists {
        queue.push_back(root_pid);
        visited.insert(root_pid);
    }

    // BFS 遍历进程树
    while let Some(current_pid) = queue.pop_front() {
        result.push(current_pid);

        // 查找所有以 current_pid 为父进程的子进程
        for (pid, process) in system.processes() {
            if process.parent() == Some(current_pid) && !visited.contains(pid) {
                visited.insert(*pid);
                queue.push_back(*pid);
            }
        }
    }

    // 如果根进程不存在（可能被 exec 替换了），直接查找以 root_pid 为父进程的子进程
    // 这可以处理 shell -c "command" 场景中 shell 被 exec 替换的情况
    if !root_exists {
        for (pid, process) in system.processes() {
            if process.parent() == Some(root_pid) && !visited.contains(pid) {
                visited.insert(*pid);
                result.push(*pid);
                // 递归查找这些子进程的后代
                let mut sub_queue = VecDeque::new();
                sub_queue.push_back(*pid);
                while let Some(current) = sub_queue.pop_front() {
                    for (child_pid, child_process) in system.processes() {
                        if child_process.parent() == Some(current) && !visited.contains(child_pid) {
                            visited.insert(*child_pid);
                            result.push(*child_pid);
                            sub_queue.push_back(*child_pid);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Windows 辅助函数：获取整个进程树（用于暂停/恢复）
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
