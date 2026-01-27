//! 项目管理模块
//! 负责项目数据结构、package.json 解析和项目扫描

mod package;
#[allow(dead_code)]
mod scanner;

pub use package::*;
#[allow(unused_imports)]
pub use scanner::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::pty::PtyHandle;

/// 命令类型枚举
/// - NpmScript: 通过包管理器执行的 npm scripts
/// - RawShell: 直接在 Shell 中执行的原始命令
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandType {
    NpmScript,
    RawShell,
}

/// 命令条目
/// 用于存储 npm scripts 或用户自定义命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub id: String,
    pub name: String,
    pub command: String,
    pub cmd_type: CommandType,
}

impl CommandEntry {
    /// 创建新的 NpmScript 类型命令
    pub fn new_npm_script(name: &str, command: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            cmd_type: CommandType::NpmScript,
        }
    }

    /// 创建新的 RawShell 类型命令
    pub fn new_raw_shell(name: &str, command: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            cmd_type: CommandType::RawShell,
        }
    }
}

/// 项目结构体
/// 代表一个被管理的前端项目
#[derive(Debug)]
pub struct Project {
    /// 项目路径
    pub path: PathBuf,
    /// 项目名称（从目录名或 package.json 获取）
    pub name: String,
    /// 项目别名（用户自定义显示名称）
    pub alias: Option<String>,
    /// 从 package.json 解析的 scripts
    pub scripts: HashMap<String, String>,
    /// 用户自定义命令
    pub custom_commands: Vec<CommandEntry>,
    /// Dev Server PTY 句柄
    pub dev_pty: Option<PtyHandle>,
    /// 交互式 Shell PTY 句柄
    pub shell_pty: Option<PtyHandle>,
    /// Dev Terminal 滚动偏移量（用于查看历史 log）
    pub dev_scroll_offset: usize,
    /// Shell Terminal 滚动偏移量（用于查看历史）
    pub shell_scroll_offset: usize,
    /// Dev Server 启动时间
    pub dev_started_at: Option<Instant>,
}

impl Project {
    /// 创建新项目
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            name,
            alias: None,
            scripts: HashMap::new(),
            custom_commands: Vec::new(),
            dev_pty: None,
            shell_pty: None,
            dev_scroll_offset: 0,
            shell_scroll_offset: 0,
            dev_started_at: None,
        }
    }

    /// 记录 Dev Server 启动时间
    pub fn mark_dev_started(&mut self) {
        self.dev_started_at = Some(Instant::now());
    }

    /// 清除 Dev Server 启动时间
    pub fn mark_dev_stopped(&mut self) {
        self.dev_started_at = None;
    }

    /// 获取显示名称（优先使用别名）
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }

    /// 设置别名
    pub fn set_alias(&mut self, alias: Option<String>) {
        self.alias = alias.filter(|s| !s.trim().is_empty());
    }

    /// 从路径加载项目，自动解析 package.json
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let mut project = Self::new(path.clone());

        // 尝试解析 package.json
        let package_json_path = path.join("package.json");
        if package_json_path.exists() {
            if let Ok(pkg) = parse_package_json(&package_json_path) {
                project.name = pkg.name.unwrap_or(project.name);
                project.scripts = pkg.scripts.unwrap_or_default();
            }
        }

        Ok(project)
    }

    /// 获取所有可执行命令（npm scripts + 自定义命令）
    /// npm scripts 按名称字母顺序排序，自定义命令按添加顺序排在后面
    pub fn get_all_commands(&self) -> Vec<CommandEntry> {
        // 收集并排序 npm scripts（按名称字母顺序）
        let mut script_names: Vec<_> = self.scripts.keys().collect();
        script_names.sort();

        let mut commands: Vec<CommandEntry> = script_names
            .into_iter()
            .filter_map(|name| {
                self.scripts
                    .get(name)
                    .map(|cmd| CommandEntry::new_npm_script(name, cmd))
            })
            .collect();

        // 自定义命令按添加顺序追加
        commands.extend(self.custom_commands.clone());
        commands
    }

    /// 添加自定义命令
    pub fn add_custom_command(&mut self, name: &str, command: &str) {
        self.custom_commands
            .push(CommandEntry::new_raw_shell(name, command));
    }

    /// 检查 Dev Server 是否正在运行
    pub fn is_dev_running(&self) -> bool {
        self.dev_pty.is_some()
    }
}

/// 用于序列化的项目配置（不包含运行时状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub custom_commands: Vec<CommandEntry>,
}

impl From<&Project> for ProjectConfig {
    fn from(project: &Project) -> Self {
        Self {
            path: project.path.to_string_lossy().to_string(),
            alias: project.alias.clone(),
            custom_commands: project.custom_commands.clone(),
        }
    }
}
