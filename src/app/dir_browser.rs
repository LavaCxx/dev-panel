//! 目录浏览器模块

use std::path::PathBuf;

/// 目录浏览器状态
#[derive(Debug, Clone)]
pub struct DirectoryBrowser {
    /// 当前浏览的目录
    pub current_dir: PathBuf,
    /// 目录中的条目列表（只包含文件夹）
    pub entries: Vec<DirEntry>,
    /// 当前选中的索引
    pub selected_idx: usize,
    /// 是否显示隐藏文件
    pub show_hidden: bool,
    /// 是否在驱动器选择模式（仅 Windows）
    pub in_drive_selection: bool,
}

/// 目录条目
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub has_package_json: bool,
}

impl DirectoryBrowser {
    /// 创建新的目录浏览器，从用户主目录开始
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut browser = Self {
            current_dir: home,
            entries: Vec::new(),
            selected_idx: 0,
            show_hidden: false,
            in_drive_selection: false,
        };
        browser.refresh();
        browser
    }

    /// 创建目录浏览器，从指定目录开始
    /// 在 Windows 上，如果提供了上次的目录，会使用其盘符根目录作为起始位置
    pub fn with_initial_dir(last_dir: Option<&str>) -> Self {
        let start_dir = if let Some(dir) = last_dir {
            let path = PathBuf::from(dir);

            #[cfg(windows)]
            {
                // Windows: 提取盘符根目录（如 "C:\" 或 "D:\"）
                if let Some(prefix) = path.components().next() {
                    use std::path::Component;
                    if let Component::Prefix(p) = prefix {
                        let drive_root =
                            PathBuf::from(format!("{}\\", p.as_os_str().to_string_lossy()));
                        if drive_root.exists() {
                            drive_root
                        } else {
                            dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"))
                        }
                    } else {
                        dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"))
                    }
                } else {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"))
                }
            }

            #[cfg(not(windows))]
            {
                // 非 Windows: 如果路径存在，使用该路径的父目录；否则使用主目录
                if path.exists() {
                    path.parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")))
                } else {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                }
            }
        } else {
            dirs::home_dir().unwrap_or_else(|| {
                #[cfg(windows)]
                {
                    PathBuf::from("C:\\")
                }
                #[cfg(not(windows))]
                {
                    PathBuf::from("/")
                }
            })
        };

        let mut browser = Self {
            current_dir: start_dir,
            entries: Vec::new(),
            selected_idx: 0,
            show_hidden: false,
            in_drive_selection: false,
        };
        browser.refresh();
        browser
    }

    /// 刷新当前目录的内容
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_idx = 0;

        // Windows 驱动器选择模式
        if self.in_drive_selection {
            #[cfg(windows)]
            {
                self.entries = Self::get_windows_drives();
            }
            return;
        }

        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            let mut entries: Vec<DirEntry> = read_dir
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    // 过滤隐藏文件（除非 show_hidden）
                    if !self.show_hidden && name.starts_with('.') {
                        return false;
                    }
                    // 只显示目录
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                })
                .map(|e| {
                    let path = e.path();
                    let has_package_json = path.join("package.json").exists();
                    DirEntry {
                        name: e.file_name().to_string_lossy().to_string(),
                        path,
                        is_dir: true,
                        has_package_json,
                    }
                })
                .collect();

            // 按名称排序
            entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // 在顶部添加返回上级目录的选项（如果有上级目录）
            let has_parent = self.current_dir.parent().is_some();
            #[cfg(windows)]
            let has_parent = has_parent || self.is_at_drive_root(); // Windows 驱动器根目录可以返回驱动器选择

            if has_parent {
                let parent_path = self
                    .current_dir
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| self.current_dir.clone());
                entries.insert(
                    0,
                    DirEntry {
                        name: "..".to_string(),
                        path: parent_path,
                        is_dir: true,
                        has_package_json: false,
                    },
                );
            }

            self.entries = entries;
        }
    }

    /// 获取 Windows 驱动器列表
    #[cfg(windows)]
    fn get_windows_drives() -> Vec<DirEntry> {
        let mut drives = Vec::new();
        // 检查 A-Z 驱动器
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);
            if path.exists() {
                drives.push(DirEntry {
                    name: format!("{}: Drive", letter as char),
                    path,
                    is_dir: true,
                    has_package_json: false,
                });
            }
        }
        drives
    }

    /// 检查当前是否在驱动器根目录（Windows）
    #[cfg(windows)]
    fn is_at_drive_root(&self) -> bool {
        // Windows 驱动器根目录形如 "C:\" 或 "C:"
        let path_str = self.current_dir.to_string_lossy();
        path_str.len() <= 3 && path_str.chars().nth(1) == Some(':')
    }

    #[cfg(not(windows))]
    fn is_at_drive_root(&self) -> bool {
        false
    }

    /// 进入选中的目录
    pub fn enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_idx) {
            if entry.is_dir {
                // 特殊处理 ".." 返回上级目录
                if entry.name == ".." {
                    self.go_up();
                } else {
                    self.current_dir = entry.path.clone();
                    self.in_drive_selection = false;
                    self.refresh();
                }
            }
        }
    }

    /// 返回上级目录
    pub fn go_up(&mut self) {
        // 如果在驱动器选择模式，按返回无效
        if self.in_drive_selection {
            return;
        }

        // Windows: 如果已经在驱动器根目录，进入驱动器选择模式
        #[cfg(windows)]
        if self.is_at_drive_root() {
            self.in_drive_selection = true;
            self.refresh();
            return;
        }

        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh();
        }
    }

    /// 选择下一项
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.entries.len();
        }
    }

    /// 选择上一项
    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            if self.selected_idx == 0 {
                self.selected_idx = self.entries.len() - 1;
            } else {
                self.selected_idx -= 1;
            }
        }
    }

    /// 向上滚动多行（用于鼠标滚轮）
    pub fn scroll_up(&mut self, lines: usize) {
        if !self.entries.is_empty() {
            self.selected_idx = self.selected_idx.saturating_sub(lines);
        }
    }

    /// 向下滚动多行（用于鼠标滚轮）
    pub fn scroll_down(&mut self, lines: usize) {
        if !self.entries.is_empty() {
            let max_idx = self.entries.len().saturating_sub(1);
            self.selected_idx = (self.selected_idx + lines).min(max_idx);
        }
    }

    /// 切换隐藏文件显示
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }

    /// 获取当前选中的条目
    pub fn selected_entry(&self) -> Option<&DirEntry> {
        self.entries.get(self.selected_idx)
    }
}

impl Default for DirectoryBrowser {
    fn default() -> Self {
        Self::new()
    }
}
