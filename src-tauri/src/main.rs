//! TeleAgent Monitor — 主入口
//!
//! Tauri 2 应用，置顶悬浮窗监控 TeleAgent 会话任务状态
//! 支持系统托盘图标：左键切换面板显示、右键菜单（显示/隐藏/退出）
//!
//! 双窗口架构：
//! - main 窗口：面板模式（380x500）
//! - ball 窗口：加速球模式（60x120，默认启动时显示）
//! 两个窗口互斥显示，由前端控制切换

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
    Emitter, Manager, PhysicalPosition,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// 将面板窗口定位到屏幕右侧垂直居中（workArea 右边缘）
fn position_panel_right(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let wa = monitor.work_area();
        let wa_size = wa.size;
        let wa_pos = wa.position;
        let win_size = window.outer_size().unwrap_or_default();
        let x = wa_pos.x + wa_size.width as i32 - win_size.width as i32;
        let y = wa_pos.y + (wa_size.height as i32 - win_size.height as i32) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        info!("[window] 面板定位至右侧 ({}, {})", x, y);
    } else {
        warn!("[window] 无法获取显示器信息，跳过右侧定位");
    }
}

/// 将加速球窗口定位到屏幕右侧垂直居中（workArea 右边缘）
fn position_ball_right(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let wa = monitor.work_area();
        let wa_size = wa.size;
        let wa_pos = wa.position;
        let win_size = window.outer_size().unwrap_or_default();
        let x = wa_pos.x + wa_size.width as i32 - win_size.width as i32;
        let y = wa_pos.y + (wa_size.height as i32 - win_size.height as i32) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        info!("[ball] 加速球定位至右侧 ({}, {})", x, y);
    } else {
        warn!("[ball] 无法获取显示器信息，跳过右侧定位");
    }
}

/// 切换主窗口的显示/隐藏状态
///
/// 若主窗口当前可见 → 隐藏
/// 若主窗口当前隐藏 → 显示，并同步隐藏加速球（互斥）
fn toggle_window_visibility(app: &tauri::AppHandle) {
    let Some(main_win) = app.get_webview_window("main") else {
        warn!("[tray] 找不到 main 窗口");
        return;
    };

    if main_win.is_visible().unwrap_or(false) {
        let _ = main_win.hide();
        info!("[tray] 窗口已隐藏到托盘");
    } else {
        // 显示面板前先隐藏加速球（互斥），再定位到右侧
        hide_ball_window(app);
        let _ = main_win.show();
        let _ = main_win.set_focus();
        position_panel_right(&main_win);
        info!("[tray] 窗口已从托盘恢复显示");
    }
}

/// 隐藏加速球窗口（若存在）
fn hide_ball_window(app: &tauri::AppHandle) {
    if let Some(ball) = app.get_webview_window("ball") {
        let _ = ball.hide();
        info!("[tray] 加速球窗口已隐藏");
    }
}

/// 显示加速球窗口并隐藏面板（互斥）
/// 加速球定位到屏幕右侧中间偏下
fn show_ball_window(app: &tauri::AppHandle) {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }
    if let Some(ball) = app.get_webview_window("ball") {
        // 定位到屏幕右侧垂直居中
        position_ball_right(&ball);
        let _ = ball.show();
        let _ = ball.set_focus();
        info!("[ball] 加速球窗口已显示");
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
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["com.teleagent.monitor"]),
        ))
        .setup(|app| {
            info!("[main] Tauri 应用初始化完成");

            // === 创建托盘菜单 ===
            let show_item = MenuItem::with_id(app, "show", "显示面板", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &settings_item, &hide_item, &quit_item])?;

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
                        toggle_window_visibility(app);
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            // 显示面板，隐藏加速球，定位到右侧
                            hide_ball_window(app);
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                position_panel_right(&window);
                                info!("[tray] 菜单：显示面板");
                            }
                        }
                        "settings" => {
                            // 打开设置：显示面板并隐藏加速球，定位到右侧
                            hide_ball_window(app);
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                position_panel_right(&window);
                                // 通知前端打开设置面板
                                let _ = app.emit("open-settings", ());
                                info!("[tray] 菜单：打开设置");
                            }
                        }
                        "hide" => {
                            // 隐藏所有窗口（面板 + 加速球）到托盘
                            hide_ball_window(app);
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

            // === 双窗口互斥初始化 ===
            // 默认启动显示加速球（ball 窗口），隐藏面板（main 窗口）
            // 主窗口和 ball 窗口在 tauri.conf.json 中都设置 visible:false
            // 这里由 Rust 控制显示哪个窗口
            show_ball_window(app.handle());
            info!("[main] 启动完成：默认显示加速球窗口");

            // === 开机自启动：首次启动默认启用 ===
            // 如果当前未启用则自动启用（用户可在设置中关闭）
            let autostart_manager = app.autolaunch();
            if !autostart_manager.is_enabled().unwrap_or(false) {
                match autostart_manager.enable() {
                    Ok(()) => info!("[main] 开机自启动已默认启用"),
                    Err(e) => warn!("[main] 开机自启动启用失败: {}", e),
                }
            } else {
                info!("[main] 开机自启动已启用（跳过）");
            }

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
            commands::is_autostart_enabled,
            commands::enable_autostart,
            commands::disable_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}