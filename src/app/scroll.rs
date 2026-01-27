//! 平滑滚动模块

/// 平滑滚动状态
/// 通过缓动插值实现流畅的滚动动画效果
#[derive(Debug, Clone)]
pub struct SmoothScroll {
    /// 目标滚动位置
    target: f32,
    /// 当前滚动位置（用于渲染）
    current: f32,
    /// 缓动系数（0.0-1.0，越大越快收敛）
    easing: f32,
}

impl Default for SmoothScroll {
    fn default() -> Self {
        Self {
            target: 0.0,
            current: 0.0,
            easing: 0.75, // 每帧移动 45% 的距离差，轻快响应
        }
    }
}

impl SmoothScroll {
    /// 创建新的平滑滚动状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置目标位置（绝对值）
    pub fn set_target(&mut self, target: f32) {
        self.target = target.max(0.0);
    }

    /// 增量滚动
    pub fn scroll_by(&mut self, delta: f32) {
        self.target = (self.target + delta).max(0.0);
    }

    /// 限制目标位置不超过最大值
    pub fn clamp_target(&mut self, max: f32) {
        if self.target > max {
            self.target = max;
        }
        if self.current > max {
            self.current = max;
        }
    }

    /// 更新当前位置（每帧调用）
    /// 返回是否仍在动画中
    pub fn update(&mut self) -> bool {
        let diff = self.target - self.current;
        if diff.abs() < 0.5 {
            // 足够接近，直接到达目标
            self.current = self.target;
            false
        } else {
            // 缓动插值：每帧移动一定比例的距离差
            self.current += diff * self.easing;
            true
        }
    }

    /// 获取当前渲染位置（四舍五入到整数）
    pub fn position(&self) -> usize {
        self.current.round().max(0.0) as usize
    }

    /// 是否正在动画中
    pub fn is_animating(&self) -> bool {
        (self.target - self.current).abs() > 0.5
    }

    /// 重置滚动状态
    pub fn reset(&mut self) {
        self.target = 0.0;
        self.current = 0.0;
    }

    /// 立即跳转到目标位置（无动画）
    pub fn jump_to(&mut self, position: f32) {
        self.target = position.max(0.0);
        self.current = self.target;
    }
}
