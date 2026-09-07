//! Tauri IPC 命令定义
//!
//! 前端通过 invoke("command_name") 调用这些函数

use log::{info, warn, error};

use crate::config;
use crate::db;

/// 查询所有 running 状态的会话及其 todo
#[tauri::command]
pub fn fetch_running_sessions() -> Result<Vec<db::SessionInfo>, String> {
    info!("[cmd] fetch_running_sessions 被调用");

    match db::fetch_running_sessions() {
        Ok(sessions) => {
            info!("[cmd] 返回 {} 个会话", sessions.len());
            Ok(sessions)
        }
        Err(e) => {
            error!("[cmd] fetch_running_sessions 失败: {}", e);
            Err(e)
        }
    }
}

/// 查询所有启用的定时任务
#[tauri::command]
pub fn fetch_scheduler_jobs() -> Result<Vec<db::SchedulerJob>, String> {
    info!("[cmd] fetch_scheduler_jobs 被调用");

    match db::fetch_scheduler_jobs() {
        Ok(jobs) => {
            info!("[cmd] 返回 {} 个定时任务", jobs.len());
            Ok(jobs)
        }
        Err(e) => {
            error!("[cmd] fetch_scheduler_jobs 失败: {}", e);
            Err(e)
        }
    }
}

/// 检查 TeleAgent 是否在线
#[tauri::command]
pub fn check_agent_online() -> bool {
    let online = db::check_agent_online();
    if !online {
        warn!("[cmd] TeleAgent 离线");
    }
    online
}

/// 唤起并聚焦 TeleAgent 主窗口
///
/// 通过 teleai-cowork:// 自定义协议唤起 TeleAgent（该协议已注册到
/// HKCU\Software\Classes\teleai-cowork，命令为 TeleAgent.exe "%1"）。
/// 注意：TeleAgent 主进程为加密字节码，无法精确跳转到指定会话，
/// 此处仅实现"唤起并聚焦主窗口"。
#[tauri::command]
pub fn open_teleagent() -> Result<(), String> {
    info!("[cmd] open_teleagent 被调用");

    let protocol_url = "teleai-cowork://open";

    // 方式一：直接用 cmd start 唤起协议
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", "start", "", protocol_url])
            .output()
            .map_err(|e| format!("唤起 TeleAgent 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("[cmd] 唤起 TeleAgent 失败: {}", stderr);
            return Err(format!("唤起 TeleAgent 失败: {}", stderr));
        }

        info!("[cmd] teleai-cowork:// 协议已发起");
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = protocol_url;
        Err("当前平台不支持协议唤起".to_string())
    }
}

/// 前端日志写入——前端通过此命令将日志传递到 Rust 端统一输出
#[tauri::command]
pub fn write_log(level: String, message: String) {
    match level.as_str() {
        "info" => info!("[frontend] {}", message),
        "warn" => warn!("[frontend] {}", message),
        "error" => error!("[frontend] {}", message),
        "debug" => log::debug!("[frontend] {}", message),
        _ => info!("[frontend][{}] {}", level, message),
    }
}

/// 获取当前配置
#[tauri::command]
pub fn get_config() -> Result<config::AppConfig, String> {
    info!("[cmd] get_config 被调用");
    Ok(config::get_config())
}

/// 保存配置
#[tauri::command]
pub fn save_config(
    teleagent_data_dir: String,
    session_poll_interval: u64,
    job_poll_interval: u64,
    max_sessions: usize,
    stale_threshold_minutes: i64,
) -> Result<(), String> {
    info!("[cmd] save_config 被调用");

    let cfg = config::AppConfig {
        teleagent_data_dir,
        session_poll_interval,
        job_poll_interval,
        max_sessions,
        stale_threshold_minutes,
    };

    config::save_config(&cfg)?;
    info!("[cmd] 配置已保存");
    Ok(())
}
