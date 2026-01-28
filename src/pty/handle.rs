//! PTY 句柄模块
//! 包含 PTY 句柄的定义和操作

use super::process_tree::collect_process_tree;
use super::resource::ProcessResourceUsage;
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
    /// CPU 使用率会归一化到 0-100% 范围（按 CPU 核心数计算）
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

            // 归一化 CPU 使用率：sysinfo 的 cpu_usage() 返回的是单核百分比
            // 多进程累加后可能超过 100%，这里除以 CPU 核心数得到系统整体使用率
            let cpu_count = system.cpus().len().max(1) as f32;
            let normalized_cpu = total_cpu / cpu_count;

            self.resource_usage.cpu_percent = normalized_cpu;
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
        use super::process_tree::get_process_tree;
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
        use super::process_tree::get_process_tree;
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

    /// 终止进程及其所有子进程
    /// 在 Unix 上使用 SIGTERM/SIGKILL 发送给进程组
    /// 在 Windows 上使用 TerminateProcess 终止进程树
    pub fn kill(&mut self) {
        if let Some(pid) = self.pid.take() {
            log::info!("Killing PTY process tree: {}", pid);

            #[cfg(unix)]
            {
                unsafe {
                    // 如果进程被暂停，先恢复它（否则可能无法正常终止）
                    if self.suspended {
                        let pgid = libc::getpgid(pid as i32);
                        let target_pid = if pgid > 0 { -pgid } else { -(pid as i32) };
                        libc::kill(target_pid, libc::SIGCONT);
                    }

                    // 尝试获取进程组 ID
                    let pgid = libc::getpgid(pid as i32);
                    let target_pid = if pgid > 0 { -pgid } else { -(pid as i32) };

                    // 先发送 SIGTERM 给进程组，给进程一个优雅退出的机会
                    libc::kill(target_pid, libc::SIGTERM);

                    // 给进程一点时间来响应 SIGTERM
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // 发送 SIGKILL 确保进程被终止
                    libc::kill(target_pid, libc::SIGKILL);

                    // 同时直接向主进程发送信号，以防进程组信号不起作用
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }

            #[cfg(windows)]
            {
                use super::process_tree::get_process_tree;
                use windows::Win32::Foundation::CloseHandle;
                use windows::Win32::System::Threading::{
                    OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
                    PROCESS_TERMINATE,
                };

                unsafe {
                    // 获取整个进程树
                    if let Ok(process_tree) = get_process_tree(pid) {
                        // 收集所有需要等待的进程句柄
                        let mut handles_to_wait = Vec::new();

                        // 终止进程树中的所有进程
                        for child_pid in process_tree {
                            if let Ok(handle) = OpenProcess(
                                PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
                                false,
                                child_pid,
                            ) {
                                let _ = TerminateProcess(handle, 1);
                                handles_to_wait.push(handle);
                            }
                        }

                        // 等待所有进程终止（最多 1000ms）
                        // 增加等待时间以确保 ConPTY 资源有足够时间释放
                        for handle in &handles_to_wait {
                            let _ = WaitForSingleObject(*handle, 1000);
                        }

                        // 关闭所有句柄
                        for handle in handles_to_wait {
                            let _ = CloseHandle(handle);
                        }
                    }

                    // 额外等待一段时间，确保 ConPTY 子系统完全释放资源
                    // 这对于避免快速连续创建 PTY 时的 0xc0000142 错误很重要
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        self.running = false;
        self.suspended = false;
    }
}

/// Drop 实现：确保 PTY 进程在句柄销毁时被正确终止
impl Drop for PtyHandle {
    fn drop(&mut self) {
        // 如果进程还在运行，终止它
        if self.pid.is_some() {
            self.kill();
        }
    }
}
