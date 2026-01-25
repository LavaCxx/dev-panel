# DevPanel

[![CI](https://github.com/lavac/devpanel/actions/workflows/ci.yml/badge.svg)](https://github.com/lavac/devpanel/actions/workflows/ci.yml)
[![Release](https://github.com/lavac/devpanel/actions/workflows/release.yml/badge.svg)](https://github.com/lavac/devpanel/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

多项目终端管理 TUI 工具 - 在单窗口内管理多个前端项目，支持切割视图（Dev Server 日志 + 交互式 Shell）。

![DevPanel Screenshot](docs/screenshot.png)

## 特性

- **多项目管理**: 在一个终端窗口中管理多个前端项目
- **分屏视图**: 上方显示 Dev Server 输出，下方显示交互式 Shell
- **命令面板**: 快速执行 npm scripts 和自定义命令
- **自动检测**: 自动解析 `package.json` 获取可用脚本
- **包管理器智能检测**: 自动检测 npm/yarn/pnpm/bun
- **进程冻结**: 暂停/恢复 Dev Server 进程，节省系统资源
- **项目别名**: 为项目设置自定义显示名称
- **配置持久化**: 项目列表和自定义命令自动保存
- **跨平台**: 支持 macOS 和 Windows
- **精美 UI**: Catppuccin Mocha 配色 + 圆角边框
- **完整终端支持**: 支持 ANSI 颜色、Starship prompt 等美化
- **鼠标支持**: 点击切换焦点和选择项目
- **中英文切换**: 支持界面语言切换

## 安装

### 从 Releases 下载（推荐）

从 [Releases](https://github.com/lavac/devpanel/releases) 页面下载对应平台的二进制文件：

| 平台 | 架构 | 文件 |
|------|------|------|
| macOS | Intel | `devpanel-macos-x86_64` |
| macOS | Apple Silicon | `devpanel-macos-aarch64` |
| Linux | x86_64 | `devpanel-linux-x86_64` |
| Linux | ARM64 | `devpanel-linux-aarch64` |
| Windows | x86_64 | `devpanel-windows-x86_64.exe` |

```bash
# macOS / Linux
chmod +x devpanel-*
./devpanel-*

# 或安装到系统
sudo mv devpanel-* /usr/local/bin/devpanel
```

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/lavac/devpanel.git
cd devpanel

# 构建
cargo build --release

# 运行
./target/release/devpanel
```

## 使用方法

### 快捷键

**项目导航（侧边栏）**
| 按键 | 功能 |
|------|------|
| `1-9` | 快速切换到对应项目 |
| `Tab` / `Shift+Tab` | 切换项目 |
| `j` / `k` / `↑` / `↓` | 切换项目 |
| `Enter` | 进入 Interactive Shell |
| `r` | 打开命令面板 |
| `a` | 添加新项目 |
| `e` | 编辑项目别名 |
| `c` | 添加自定义命令 |
| `d` | 删除项目 |
| `,` | 打开设置 |
| `q` / `Ctrl+C` | 退出程序 |
| `?` | 显示帮助 |

**Dev Terminal（只显示输出）**
| 按键 | 功能 |
|------|------|
| `x` | 发送中断信号给进程 |
| `p` | 暂停/恢复进程（冻结） |
| `r` | 运行新命令（覆盖当前） |
| `s` | 停止 Dev Server |
| 鼠标点击 | 聚焦并可滚动查看 log |
| `j/k/↑/↓` | 滚动查看历史 log |
| `Esc` | 返回侧边栏 |

**Interactive Shell（完全交互）**
| 按键 | 功能 |
|------|------|
| 所有按键 | 直接发送给 Shell |
| `Esc` | 返回侧边栏（不关闭 Shell） |

### 鼠标操作

| 操作 | 功能 |
|------|------|
| 左键点击侧边栏 | 选择项目 |
| 左键点击 Dev 区域 | 聚焦到 Dev Terminal（可滚动） |
| 左键点击 Shell 区域 | 进入 Interactive Shell |
| 滚轮滚动 | 根据焦点区域滚动内容 |

### 添加项目

1. 按 `a` 键进入添加项目模式
2. 输入项目的完整路径（需包含 `package.json`）
3. 按 `Enter` 确认

### 运行命令

1. 选择一个项目
2. 按 `r` 打开命令面板
3. 使用 `j`/`k` 选择命令
4. 按 `Enter` 执行

### 添加自定义命令

1. 选择一个项目
2. 按 `c` 进入添加命令模式
3. 输入格式: `命令名称:实际命令`
   - 例如: `docker:docker-compose up -d`
4. 按 `Enter` 确认

### 暂停/恢复进程

按 `p` 键可以暂停（冻结）正在运行的 Dev Server 进程，节省 CPU 和内存资源。
- 暂停状态在侧边栏显示 `⏸` 图标
- 再次按 `p` 恢复运行
- 仅支持 macOS/Linux

## 配置文件

配置文件 `devpanel.json` 保存在当前工作目录：

```json
{
  "projects": [
    {
      "path": "/path/to/your/project",
      "alias": "My App",
      "custom_commands": [
        {
          "id": "uuid",
          "name": "Start Docker",
          "command": "docker-compose up -d",
          "type": "RawShell"
        }
      ]
    }
  ],
  "settings": {
    "theme": "catppuccin-mocha",
    "default_runner": "pnpm",
    "language": "Chinese"
  }
}
```

## 命令类型

- **NpmScript**: 通过包管理器执行的 npm scripts（如 `pnpm dev`）
- **RawShell**: 直接在 Shell 中执行的原始命令（如 `docker-compose up`）

## 架构

```
devpanel/
├── src/
│   ├── main.rs           # 入口点，异步主循环
│   ├── app.rs            # AppState 全局状态管理
│   ├── event.rs          # 事件处理和快捷键
│   ├── i18n.rs           # 国际化支持
│   ├── ui/               # UI 组件
│   │   ├── layout.rs     # 主布局
│   │   ├── sidebar.rs    # 项目列表
│   │   ├── terminal.rs   # 终端面板
│   │   ├── settings_popup.rs
│   │   ├── command_palette.rs
│   │   └── theme.rs      # Catppuccin 主题
│   ├── pty/              # PTY 管理
│   │   ├── manager.rs    # PTY 生命周期
│   │   └── bridge.rs     # PTY-UI 桥接
│   ├── project/          # 项目管理
│   │   ├── package.rs    # package.json 解析
│   │   └── scanner.rs    # 项目扫描
│   ├── config/           # 配置持久化
│   └── platform/         # 跨平台工具
└── Cargo.toml
```

## 技术栈

- **UI**: [Ratatui](https://ratatui.rs/) + [Crossterm](https://github.com/crossterm-rs/crossterm)
- **异步**: [Tokio](https://tokio.rs/)
- **PTY**: [portable-pty](https://github.com/wez/wezterm/tree/main/pty) + [tui-term](https://github.com/a-kenji/tui-term)
- **终端解析**: [vt100](https://github.com/doy/vt100-rust)

## 许可证

MIT
