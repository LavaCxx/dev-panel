//! 配置持久化模块
//! 负责配置文件的读写

use super::AppConfig;
use std::path::Path;

/// 默认配置文件名
pub const CONFIG_FILE_NAME: &str = "devpanel.json";

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
pub fn save_config(config: &AppConfig, path: &Path) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// 获取当前工作目录下的配置文件路径
pub fn get_config_path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(CONFIG_FILE_NAME)
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
