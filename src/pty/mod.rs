//! PTY (伪终端) 管理模块
//! 负责创建和管理伪终端会话

#![allow(dead_code)]

mod bridge;
mod handle;
mod manager;
mod process_tree;
mod resource;

pub use bridge::*;
pub use handle::*;
pub use manager::*;
pub use resource::*;

// 内部使用
pub(crate) use process_tree::*;

/// PTY 事件
/// 用于在异步任务和主线程之间传递 PTY 相关事件
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// 收到输出数据
    Output { pty_id: String, data: Vec<u8> },
    /// PTY 进程已退出
    Exited {
        pty_id: String,
        exit_code: Option<i32>,
    },
    /// 错误发生
    Error { pty_id: String, message: String },
}
