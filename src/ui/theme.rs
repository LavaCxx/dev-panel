//! 主题模块
//! 实现 Catppuccin Mocha 配色方案

#![allow(dead_code)]

use ratatui::style::Color;

/// Catppuccin Mocha 配色
/// 参考: https://github.com/catppuccin/catppuccin
pub struct CatppuccinMocha;

impl CatppuccinMocha {
    // 基础色
    pub const BASE: Color = Color::Rgb(30, 30, 46);       // #1e1e2e
    pub const MANTLE: Color = Color::Rgb(24, 24, 37);     // #181825
    pub const CRUST: Color = Color::Rgb(17, 17, 27);      // #11111b
    
    // 表面色
    pub const SURFACE0: Color = Color::Rgb(49, 50, 68);   // #313244
    pub const SURFACE1: Color = Color::Rgb(69, 71, 90);   // #45475a
    pub const SURFACE2: Color = Color::Rgb(88, 91, 112);  // #585b70
    
    // 覆盖色
    pub const OVERLAY0: Color = Color::Rgb(108, 112, 134); // #6c7086
    pub const OVERLAY1: Color = Color::Rgb(127, 132, 156); // #7f849c
    pub const OVERLAY2: Color = Color::Rgb(147, 153, 178); // #9399b2
    
    // 文字色
    pub const TEXT: Color = Color::Rgb(205, 214, 244);     // #cdd6f4
    pub const SUBTEXT1: Color = Color::Rgb(186, 194, 222); // #bac2de
    pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200); // #a6adc8
    
    // 强调色
    pub const ROSEWATER: Color = Color::Rgb(245, 224, 220); // #f5e0dc
    pub const FLAMINGO: Color = Color::Rgb(242, 205, 205);  // #f2cdcd
    pub const PINK: Color = Color::Rgb(245, 194, 231);      // #f5c2e7
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);     // #cba6f7
    pub const RED: Color = Color::Rgb(243, 139, 168);       // #f38ba8
    pub const MAROON: Color = Color::Rgb(235, 160, 172);    // #eba0ac
    pub const PEACH: Color = Color::Rgb(250, 179, 135);     // #fab387
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);    // #f9e2af
    pub const GREEN: Color = Color::Rgb(166, 227, 161);     // #a6e3a1
    pub const TEAL: Color = Color::Rgb(148, 226, 213);      // #94e2d5
    pub const SKY: Color = Color::Rgb(137, 220, 235);       // #89dceb
    pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);  // #74c7ec
    pub const BLUE: Color = Color::Rgb(137, 180, 250);      // #89b4fa
    pub const LAVENDER: Color = Color::Rgb(180, 190, 254);  // #b4befe
}

/// 应用主题
/// 定义各 UI 元素使用的颜色
pub struct Theme {
    /// 背景色
    pub bg: Color,
    /// 前景色（文字）
    pub fg: Color,
    /// 边框颜色（未聚焦）
    pub border: Color,
    /// 边框颜色（聚焦）
    pub border_focused: Color,
    /// 选中项高亮
    pub selection: Color,
    /// 选中项文字
    pub selection_fg: Color,
    /// 状态栏背景
    pub status_bg: Color,
    /// 状态栏文字
    pub status_fg: Color,
    /// 标题颜色
    pub title: Color,
    /// 成功色
    pub success: Color,
    /// 警告色
    pub warning: Color,
    /// 错误色
    pub error: Color,
    /// 信息色
    pub info: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl Theme {
    /// 创建 Catppuccin Mocha 主题
    pub fn catppuccin_mocha() -> Self {
        Self {
            bg: CatppuccinMocha::BASE,
            fg: CatppuccinMocha::TEXT,
            border: CatppuccinMocha::SURFACE1,
            border_focused: CatppuccinMocha::LAVENDER,
            selection: CatppuccinMocha::SURFACE0,
            selection_fg: CatppuccinMocha::TEXT,
            status_bg: CatppuccinMocha::MANTLE,
            status_fg: CatppuccinMocha::SUBTEXT1,
            title: CatppuccinMocha::MAUVE,
            success: CatppuccinMocha::GREEN,
            warning: CatppuccinMocha::YELLOW,
            error: CatppuccinMocha::RED,
            info: CatppuccinMocha::BLUE,
        }
    }
}
