//! 进程资源使用信息模块

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
