//! TeleAgent Monitor — 主入口
//!
//! Tauri 2 应用，置顶悬浮窗监控 TeleAgent 会话任务状态
//! 支持系统托盘图标：左键切换面板显示、右键菜单（显示/隐藏/退出）

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod config;
mod db;
mod watcher;

use log::{info, warn};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};

/// 切换主窗口的显示/隐藏状态
fn toggle_window_visibility(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        info!("[tray] 窗口已隐藏到托盘");
    } else {
        let _ = window.show();
        let _ = window.set_focus();
        info!("[tray] 窗口已从托盘恢复显示");
    }
}

fn main() {
    // 初始化日志
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .init();

    info!("========================================");
    info!("  TeleAgent Monitor 启动中...");
    info!("========================================");

    // 加载配置文件
    let _app_config = config::load_config();
    info!("[main] 配置已加载");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            info!("[main] Tauri 应用初始化完成");

            // === 创建托盘菜单 ===
            let show_item = MenuItem::with_id(app, "show", "显示面板", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

            // === 创建托盘图标 ===
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("TeleAgent Monitor")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    // 左键点击托盘图标 → 切换窗口显示/隐藏
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window_visibility(&window);
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                info!("[tray] 菜单：显示面板");
                            }
                        }
                        "hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                                info!("[tray] 菜单：隐藏到托盘");
                            }
                        }
                        "quit" => {
                            info!("[tray] 菜单：退出程序");
                            app.exit(0);
                        }
                        _ => {
                            warn!("[tray] 未知菜单事件: {:?}", event.id);
                        }
                    }
                })
                .build(app)?;

            info!("[main] 系统托盘已创建");

            // 启动文件监听
            let app_handle = app.handle().clone();
            watcher::start_watcher(app_handle);
            info!("[main] 文件监听已注册");

            Ok(())
        })
        .on_window_event(|window, event| {
            // 拦截窗口关闭请求，改为隐藏到托盘
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
                info!("[main] 窗口关闭请求已拦截，改为隐藏到托盘");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_running_sessions,
            commands::fetch_scheduler_jobs,
            commands::check_agent_online,
            commands::open_teleagent,
            commands::write_log,
            commands::get_config,
            commands::save_config,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
