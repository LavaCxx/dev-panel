//! Spinner 组件
//! 类似 Bubble Tea 风格的加载动画

#![allow(dead_code)]

use std::time::{Duration, Instant};

/// Spinner 动画帧
pub struct Spinner {
    frames: Vec<&'static str>,
    interval: Duration,
    start_time: Instant,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::dots()
    }
}

impl Spinner {
    /// 点状 spinner (Bubble Tea 默认风格)
    pub fn dots() -> Self {
        Self {
            frames: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            interval: Duration::from_millis(80),
            start_time: Instant::now(),
        }
    }

    /// 线条 spinner
    pub fn line() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            interval: Duration::from_millis(80),
            start_time: Instant::now(),
        }
    }

    /// 脉冲 spinner
    pub fn pulse() -> Self {
        Self {
            frames: vec!["█", "▓", "▒", "░", "▒", "▓"],
            interval: Duration::from_millis(120),
            start_time: Instant::now(),
        }
    }

    /// 弹跳点 spinner
    pub fn bounce() -> Self {
        Self {
            frames: vec!["⠁", "⠂", "⠄", "⠂"],
            interval: Duration::from_millis(120),
            start_time: Instant::now(),
        }
    }

    /// 圆形 spinner
    pub fn circle() -> Self {
        Self {
            frames: vec!["◐", "◓", "◑", "◒"],
            interval: Duration::from_millis(100),
            start_time: Instant::now(),
        }
    }

    /// 获取当前帧
    pub fn frame(&self) -> &'static str {
        let elapsed = self.start_time.elapsed();
        let total_frames = self.frames.len();
        let frame_index = (elapsed.as_millis() / self.interval.as_millis()) as usize % total_frames;
        self.frames[frame_index]
    }

    /// 重置 spinner
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }
}

/// 进度条风格
pub struct ProgressStyle {
    pub filled: char,
    pub empty: char,
    pub head: char,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self {
            filled: '█',
            empty: '░',
            head: '▓',
        }
    }
}

impl ProgressStyle {
    /// 圆角风格
    pub fn rounded() -> Self {
        Self {
            filled: '●',
            empty: '○',
            head: '◐',
        }
    }

    /// 方块风格
    pub fn block() -> Self {
        Self {
            filled: '■',
            empty: '□',
            head: '▣',
        }
    }

    /// 生成进度条字符串
    pub fn render(&self, progress: f64, width: usize) -> String {
        let progress = progress.clamp(0.0, 1.0);
        let filled_width = (progress * width as f64) as usize;
        let empty_width = width.saturating_sub(filled_width);

        let mut result = String::with_capacity(width);

        if filled_width > 0 {
            result.push_str(
                &self
                    .filled
                    .to_string()
                    .repeat(filled_width.saturating_sub(1)),
            );
            if filled_width < width {
                result.push(self.head);
            } else {
                result.push(self.filled);
            }
        }

        result.push_str(&self.empty.to_string().repeat(empty_width));
        result
    }
}
