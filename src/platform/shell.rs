//! 跨平台 Shell 处理模块
//! 负责检测系统默认 Shell 和处理平台差异

#![allow(dead_code)]

/// 获取系统默认 Shell
pub fn get_default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    }
    #[cfg(windows)]
    {
        // Windows 优先使用 PowerShell
        "powershell.exe".to_string()
    }
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
        if shell.contains("powershell") {
            (shell, vec!["-Command".to_string(), command.to_string()])
        } else {
            // cmd.exe
            (shell, vec!["/C".to_string(), command.to_string()])
        }
    }
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
