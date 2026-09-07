//! 文件监听模块
//!
//! 监听 session-status.json 的变化，当 TeleAgent 更新该文件时，
//! 通过 Tauri 事件通知前端立即刷新。

use log::{info, warn, error};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use notify::{Watcher, RecursiveMode, EventKind};

use crate::db;

/// 启动文件监听
/// 当 session-status.json 发生变更时，向前端发送 `session-status-changed` 事件
pub fn start_watcher(app: AppHandle) {
    let watch_path = match db::get_session_status_file_path() {
        Ok(p) => p,
        Err(e) => {
            error!("[watcher] 无法获取 session-status.json 路径: {}", e);
            return;
        }
    };

    // 获取父目录（监听目录变更更可靠，Windows 上监听单个文件可能不稳定）
    let watch_dir = watch_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
        error!("[watcher] 无法获取 session-status.json 的父目录");
        PathBuf::from(".")
    });

    let target_file_name = watch_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session-status.json")
        .to_string();

    info!(
        "[watcher] 开始监听目录: {:?}，目标文件: {}",
        watch_dir, target_file_name
    );

    // 将整个监听逻辑放到独立线程中，watcher 在线程内创建和持有
    let app_handle = app.clone();
    std::thread::spawn(move || {
        // 在线程内创建 channel 和 watcher
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();

        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!("[watcher] 创建文件监听器失败: {}", e);
                return;
            }
        };

        // 开始监听
        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            error!("[watcher] 启动监听失败: {}", e);
            return;
        }

        info!("[watcher] 文件监听已启动");

        let mut last_emit = std::time::Instant::now();
        let debounce = Duration::from_millis(500); // 防抖：500ms 内不重复触发
        let file_name = target_file_name.clone();

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    // 只关心目标文件的变更
                    let is_target = event.paths.iter().any(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n == file_name.as_str())
                            .unwrap_or(false)
                    });

                    if !is_target {
                        continue;
                    }

                    // 过滤事件类型：只关心修改和创建
                    let relevant = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );

                    if !relevant {
                        continue;
                    }

                    // 防抖
                    let now = std::time::Instant::now();
                    if now.duration_since(last_emit) < debounce {
                        continue;
                    }
                    last_emit = now;

                    info!("[watcher] session-status.json 已变更，发送事件到前端");

                    // 发送事件到前端
                    if let Err(e) = app_handle.emit(
                        "session-status-changed",
                        &format!("file changed: {}", file_name),
                    ) {
                        warn!("[watcher] 发送事件失败: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    warn!("[watcher] 文件监听事件错误: {}", e);
                }
                Err(e) => {
                    error!("[watcher] channel 接收错误，监听线程退出: {}", e);
                    break;
                }
            }
        }

        info!("[watcher] 监听线程结束");
        // watcher 随线程结束自动 drop
    });
}
