//! 进程树收集模块
//! 提供跨平台的进程树收集功能

use std::collections::{HashSet, VecDeque};

/// 辅助函数：使用 sysinfo 收集整个进程树
/// 递归查找指定 PID 的所有子进程、孙进程等
/// 返回包含根进程及所有后代进程的 PID 列表
pub fn collect_process_tree(system: &sysinfo::System, root_pid: sysinfo::Pid) -> Vec<sysinfo::Pid> {
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
pub unsafe fn get_process_tree(root_pid: u32) -> anyhow::Result<HashSet<u32>> {
    use std::collections::HashMap;
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
    let mut parent_to_children: HashMap<u32, Vec<u32>> = HashMap::new();

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
    let mut queue = VecDeque::new();
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
