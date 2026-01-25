//! 国际化模块
//! 支持中英文切换

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 语言枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    /// 获取语言显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }

    /// 切换语言
    pub fn toggle(&self) -> Self {
        match self {
            Language::English => Language::Chinese,
            Language::Chinese => Language::English,
        }
    }
}

/// 国际化文本
pub struct I18n {
    lang: Language,
}

impl I18n {
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    // === 标题 ===
    pub fn app_title(&self) -> &'static str {
        match self.lang {
            Language::English => " DevPanel ",
            Language::Chinese => " DevPanel ",
        }
    }

    pub fn projects(&self) -> &'static str {
        match self.lang {
            Language::English => " Projects ",
            Language::Chinese => " 项目列表 ",
        }
    }

    pub fn dev_server(&self) -> &'static str {
        match self.lang {
            Language::English => "Dev Server",
            Language::Chinese => "开发服务",
        }
    }

    pub fn interactive_shell(&self) -> &'static str {
        match self.lang {
            Language::English => "Interactive Shell",
            Language::Chinese => "交互终端",
        }
    }

    pub fn settings(&self) -> &'static str {
        match self.lang {
            Language::English => " Settings ",
            Language::Chinese => " 设置 ",
        }
    }

    pub fn help(&self) -> &'static str {
        match self.lang {
            Language::English => " Help ",
            Language::Chinese => " 帮助 ",
        }
    }

    // === 状态栏提示 ===
    pub fn status_hint_sidebar(&self) -> &'static str {
        match self.lang {
            Language::English => {
                " Tab: Project | Enter: Shell | r: Run | s: Stop | ,: Settings | ?: Help"
            }
            Language::Chinese => " Tab: 切换 | Enter: 终端 | r: 运行 | s: 停止 | ,: 设置 | ?: 帮助",
        }
    }

    pub fn status_hint_shell(&self) -> &'static str {
        match self.lang {
            Language::English => " Interactive Shell - type freely (Esc: back)",
            Language::Chinese => " 交互终端 - 自由输入 (Esc: 返回)",
        }
    }

    // === 设置页面 ===
    pub fn language(&self) -> &'static str {
        match self.lang {
            Language::English => "Language",
            Language::Chinese => "语言",
        }
    }

    pub fn settings_hint(&self) -> &'static str {
        match self.lang {
            Language::English => "↑/↓: Navigate | Enter: Toggle | Esc: Close",
            Language::Chinese => "↑/↓: 导航 | Enter: 切换 | Esc: 关闭",
        }
    }

    // === 弹窗 ===
    pub fn add_project(&self) -> &'static str {
        match self.lang {
            Language::English => "Add Project",
            Language::Chinese => "添加项目",
        }
    }

    pub fn enter_project_path(&self) -> &'static str {
        match self.lang {
            Language::English => "Enter project path:",
            Language::Chinese => "输入项目路径：",
        }
    }

    pub fn add_command(&self) -> &'static str {
        match self.lang {
            Language::English => "Add Custom Command",
            Language::Chinese => "添加自定义命令",
        }
    }

    pub fn command_format_hint(&self) -> &'static str {
        match self.lang {
            Language::English => "Format: name:command",
            Language::Chinese => "格式：名称:命令",
        }
    }

    pub fn edit_alias(&self) -> &'static str {
        match self.lang {
            Language::English => "Edit Alias",
            Language::Chinese => "编辑别名",
        }
    }

    pub fn alias_hint(&self) -> &'static str {
        match self.lang {
            Language::English => "Enter alias (empty to clear):",
            Language::Chinese => "输入别名（留空清除）：",
        }
    }

    pub fn alias_set(&self) -> &'static str {
        match self.lang {
            Language::English => "Alias updated",
            Language::Chinese => "别名已更新",
        }
    }

    pub fn process_suspended(&self) -> &'static str {
        match self.lang {
            Language::English => "Process suspended (frozen)",
            Language::Chinese => "进程已暂停（冻结）",
        }
    }

    pub fn process_resumed(&self) -> &'static str {
        match self.lang {
            Language::English => "Process resumed",
            Language::Chinese => "进程已恢复",
        }
    }

    pub fn suspend_not_supported(&self) -> &'static str {
        match self.lang {
            Language::English => "Suspend not supported on this platform",
            Language::Chinese => "当前平台不支持暂停功能",
        }
    }

    pub fn paused(&self) -> &'static str {
        match self.lang {
            Language::English => "PAUSED",
            Language::Chinese => "已暂停",
        }
    }

    pub fn confirm(&self) -> &'static str {
        match self.lang {
            Language::English => " Confirm ",
            Language::Chinese => " 确认 ",
        }
    }

    pub fn yes_no(&self) -> &'static str {
        match self.lang {
            Language::English => "[y] Yes  [n] No",
            Language::Chinese => "[y] 是  [n] 否",
        }
    }

    pub fn run_command(&self) -> &'static str {
        match self.lang {
            Language::English => " Run Command ",
            Language::Chinese => " 运行命令 ",
        }
    }

    // === 提示消息 ===
    pub fn no_project(&self) -> &'static str {
        match self.lang {
            Language::English => "No project selected. Press 'a' to add one.",
            Language::Chinese => "未选择项目。按 'a' 添加项目。",
        }
    }

    pub fn no_projects(&self) -> &'static str {
        match self.lang {
            Language::English => "  No projects",
            Language::Chinese => "  暂无项目",
        }
    }

    pub fn add_project_hint(&self) -> &'static str {
        match self.lang {
            Language::English => "  [+] Add Project (a)",
            Language::Chinese => "  [+] 添加项目 (a)",
        }
    }

    pub fn shell_waiting(&self) -> &'static str {
        match self.lang {
            Language::English => "Shell running, waiting for output...",
            Language::Chinese => "Shell 运行中，等待输出...",
        }
    }

    pub fn shell_ended(&self) -> &'static str {
        match self.lang {
            Language::English => "Shell process ended",
            Language::Chinese => "Shell 进程已结束",
        }
    }

    pub fn press_r_to_run(&self) -> &'static str {
        match self.lang {
            Language::English => "Press 'r' to run a command",
            Language::Chinese => "按 'r' 运行命令",
        }
    }

    pub fn press_enter_for_shell(&self) -> &'static str {
        match self.lang {
            Language::English => "Press Enter to start shell",
            Language::Chinese => "按 Enter 启动终端",
        }
    }

    pub fn dev_stopped(&self) -> &'static str {
        match self.lang {
            Language::English => "Dev server stopped",
            Language::Chinese => "开发服务已停止",
        }
    }

    pub fn sent_interrupt(&self) -> &'static str {
        match self.lang {
            Language::English => "Sent interrupt to dev server",
            Language::Chinese => "已发送中断信号",
        }
    }

    pub fn shell_started(&self) -> &'static str {
        match self.lang {
            Language::English => "Shell started - type commands here",
            Language::Chinese => "终端已启动 - 可输入命令",
        }
    }

    pub fn project_removed(&self) -> &'static str {
        match self.lang {
            Language::English => "Project removed",
            Language::Chinese => "项目已移除",
        }
    }

    pub fn delete_project(&self) -> &'static str {
        match self.lang {
            Language::English => "Delete this project?",
            Language::Chinese => "删除此项目？",
        }
    }

    pub fn loading(&self) -> &'static str {
        match self.lang {
            Language::English => "Loading...",
            Language::Chinese => "加载中...",
        }
    }
}
