//! 状态消息模块

use std::time::Instant;

/// 状态消息（带时间戳，用于自动淡出）
pub struct StatusMessage {
    pub text: String,
    pub created_at: Instant,
}

impl StatusMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            created_at: Instant::now(),
        }
    }

    /// 获取消息年龄（秒）
    pub fn age_secs(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64()
    }

    /// 消息是否应该淡出（超过 3 秒）
    pub fn should_fade(&self) -> bool {
        self.age_secs() > 3.0
    }

    /// 消息是否过期（超过 5 秒）
    pub fn is_expired(&self) -> bool {
        self.age_secs() > 5.0
    }

    /// 获取淡出透明度 (1.0 = 完全可见, 0.0 = 完全透明)
    pub fn opacity(&self) -> f64 {
        let age = self.age_secs();
        if age < 3.0 {
            1.0
        } else if age < 5.0 {
            1.0 - (age - 3.0) / 2.0
        } else {
            0.0
        }
    }
}
