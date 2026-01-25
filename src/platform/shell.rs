//! 跨平台 Shell 处理模块
//! 负责检测系统默认 Shell 和处理平台差异

#![allow(dead_code)]

#[cfg(windows)]
use crate::config::WindowsShell;

/// 获取系统默认 Shell（不带配置参数的版本，用于兼容现有代码）
pub fn get_default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    }
    #[cfg(windows)]
    {
        // 默认使用 PowerShell
        get_windows_shell(WindowsShell::PowerShell)
    }
}

/// 根据配置获取 Shell（Windows 专用）
#[cfg(windows)]
pub fn get_shell_with_config(shell_type: WindowsShell) -> String {
    get_windows_shell(shell_type)
}

/// 根据配置获取 Shell（非 Windows 平台，忽略配置）
#[cfg(not(windows))]
pub fn get_shell_with_config(_shell_type: ()) -> String {
    get_default_shell()
}

/// Windows 专用：根据配置获取 Shell
#[cfg(windows)]
fn get_windows_shell(shell_type: WindowsShell) -> String {
    match shell_type {
        WindowsShell::PowerShell => {
            get_powershell_path().unwrap_or_else(|| "powershell.exe".to_string())
        }
        WindowsShell::Cmd => get_cmd_path(),
    }
}

/// Windows 专用：获取 cmd.exe 路径
#[cfg(windows)]
fn get_cmd_path() -> String {
    // 检查环境变量中是否指定了 Shell
    if let Ok(shell) = std::env::var("COMSPEC") {
        return shell;
    }

    // 尝试找到 cmd.exe 的完整路径
    if let Ok(system_root) = std::env::var("SYSTEMROOT") {
        let cmd_path = format!("{}\\System32\\cmd.exe", system_root);
        if std::path::Path::new(&cmd_path).exists() {
            return cmd_path;
        }
    }

    // 回退到简单的 cmd.exe
    "cmd.exe".to_string()
}

/// Windows 专用：获取 PowerShell 路径
#[cfg(windows)]
pub fn get_powershell_path() -> Option<String> {
    // 尝试 PowerShell Core (pwsh)
    if which::which("pwsh").is_ok() {
        return Some("pwsh".to_string());
    }

    // 尝试 Windows PowerShell 的完整路径
    if let Ok(system_root) = std::env::var("SYSTEMROOT") {
        let ps_path = format!(
            "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
            system_root
        );
        if std::path::Path::new(&ps_path).exists() {
            return Some(ps_path);
        }
    }

    // 尝试简单的 powershell.exe
    if which::which("powershell.exe").is_ok() {
        return Some("powershell.exe".to_string());
    }

    None
}

/// 获取 Shell 的名称（不含路径）
pub fn get_shell_name(shell: &str) -> &str {
    std::path::Path::new(shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(shell)
}

/// 获取 npm 命令（Windows 需要 .cmd 扩展名）
pub fn get_npm_command() -> &'static str {
    #[cfg(windows)]
    {
        "npm.cmd"
    }
    #[cfg(unix)]
    {
        "npm"
    }
}

/// 获取 pnpm 命令
pub fn get_pnpm_command() -> &'static str {
    #[cfg(windows)]
    {
        "pnpm.cmd"
    }
    #[cfg(unix)]
    {
        "pnpm"
    }
}

/// 获取 yarn 命令
pub fn get_yarn_command() -> &'static str {
    #[cfg(windows)]
    {
        "yarn.cmd"
    }
    #[cfg(unix)]
    {
        "yarn"
    }
}

/// 检查命令是否存在
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// 构建在指定 Shell 中执行命令的参数
/// 返回 (shell_path, shell_args)
pub fn build_shell_command(command: &str) -> (String, Vec<String>) {
    let shell = get_default_shell();

    #[cfg(unix)]
    {
        (shell, vec!["-c".to_string(), command.to_string()])
    }
    #[cfg(windows)]
    {
        if shell.to_lowercase().contains("powershell") || shell.to_lowercase().contains("pwsh") {
            (shell, vec!["-Command".to_string(), command.to_string()])
        } else {
            // cmd.exe - 使用 /C 参数
            (shell, vec!["/C".to_string(), command.to_string()])
        }
    }
}

/// 检查 Shell 是否是 PowerShell
pub fn is_powershell(shell: &str) -> bool {
    let lower = shell.to_lowercase();
    lower.contains("powershell") || lower.contains("pwsh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shell_name() {
        assert_eq!(get_shell_name("/bin/zsh"), "zsh");
        assert_eq!(get_shell_name("/usr/bin/bash"), "bash");
        assert_eq!(get_shell_name("powershell.exe"), "powershell.exe");
    }

    #[test]
    fn test_build_shell_command() {
        let (shell, args) = build_shell_command("echo hello");
        assert!(!shell.is_empty());
        assert!(!args.is_empty());
    }
}
