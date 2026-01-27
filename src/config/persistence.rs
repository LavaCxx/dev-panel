//! 配置持久化模块
//! 负责配置文件的读写

use super::AppConfig;
use std::path::{Path, PathBuf};

/// 配置文件夹名称（位于用户主目录下）
pub const CONFIG_DIR_NAME: &str = ".devpanel";

/// 默认配置文件名
pub const CONFIG_FILE_NAME: &str = "config.json";

/// 从文件加载配置
pub fn load_config(path: &Path) -> anyhow::Result<AppConfig> {
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // 配置文件不存在，返回默认配置
        Ok(AppConfig::default())
    }
}

/// 保存配置到文件
/// 自动创建父目录（如果不存在）
pub fn save_config(config: &AppConfig, path: &Path) -> anyhow::Result<()> {
    // 确保配置目录存在
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// 获取配置文件目录路径
/// 返回 ~/.devpanel/
pub fn get_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR_NAME)
}

/// 获取配置文件路径
/// 返回 ~/.devpanel/config.json
pub fn get_config_path() -> PathBuf {
    get_config_dir().join(CONFIG_FILE_NAME)
}

/// 确保配置目录存在
/// 如果不存在则创建
pub fn ensure_config_dir() -> anyhow::Result<PathBuf> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_round_trip() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test-config.json");

        let mut config = AppConfig::new();
        config.settings.theme = "test-theme".to_string();

        save_config(&config, &config_path).unwrap();
        let loaded = load_config(&config_path).unwrap();

        assert_eq!(loaded.settings.theme, "test-theme");
    }
}
