//! 配置管理模块
//!
//! 从配置文件 config.json 读取运行时配置，支持外部化修改。
//! 配置文件位于 exe 同目录下（便携式），不存在时使用默认值并自动创建。

use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

/// 配置结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// TeleAgent 数据目录路径（为空则使用默认 ~/.local/share/TeleAgent）
    #[serde(default)]
    pub teleagent_data_dir: String,

    /// 会话轮询间隔（毫秒）
    #[serde(default = "default_session_poll_interval")]
    pub session_poll_interval: u64,

    /// 定时任务轮询间隔（毫秒）
    #[serde(default = "default_job_poll_interval")]
    pub job_poll_interval: u64,

    /// 最大会话显示数
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,

    /// 超时阈值（分钟）——超过此时间无新消息的 running/paused 会话视为结束
    #[serde(default = "default_stale_threshold_minutes")]
    pub stale_threshold_minutes: i64,
}

fn default_session_poll_interval() -> u64 {
    3000
}
fn default_job_poll_interval() -> u64 {
    10000
}
fn default_max_sessions() -> usize {
    15
}
fn default_stale_threshold_minutes() -> i64 {
    10
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            teleagent_data_dir: String::new(),
            session_poll_interval: default_session_poll_interval(),
            job_poll_interval: default_job_poll_interval(),
            max_sessions: default_max_sessions(),
            stale_threshold_minutes: default_stale_threshold_minutes(),
        }
    }
}

/// 全局配置实例（运行时加载一次，save 时更新）
static CONFIG: RwLock<Option<AppConfig>> = RwLock::new(None);

/// 获取配置文件路径（exe 同目录下的 config.json）
fn get_config_path() -> PathBuf {
    // 优先使用 exe 所在目录
    if let Some(exe_dir) = get_exe_dir() {
        return exe_dir.join("config.json");
    }
    // 回退到当前工作目录
    PathBuf::from("config.json")
}

/// 获取 exe 所在目录
fn get_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// 加载配置文件（如果不存在则创建默认配置）
pub fn load_config() -> AppConfig {
    // 先尝试从全局实例读取
    {
        let guard = CONFIG.read().unwrap();
        if let Some(ref cfg) = *guard {
            return cfg.clone();
        }
    }

    let config_path = get_config_path();
    info!("[config] 配置文件路径: {:?}", config_path);

    let config = if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match serde_json::from_str::<AppConfig>(&content) {
                    Ok(mut cfg) => {
                        // 如果 teleagent_data_dir 为空，使用默认值
                        if cfg.teleagent_data_dir.is_empty() {
                            cfg.teleagent_data_dir = get_default_data_dir();
                        }
                        // 补齐缺失字段的默认值
                        normalize_config(&mut cfg);
                        info!("[config] 配置加载成功: {:?}", cfg);
                        cfg
                    }
                    Err(e) => {
                        warn!("[config] 配置解析失败 ({}), 使用默认值", e);
                        let mut default = AppConfig::default();
                        default.teleagent_data_dir = get_default_data_dir();
                        default
                    }
                }
            }
            Err(e) => {
                warn!("[config] 读取配置文件失败 ({}), 使用默认值", e);
                let mut default = AppConfig::default();
                default.teleagent_data_dir = get_default_data_dir();
                default
            }
        }
    } else {
        info!("[config] 配置文件不存在, 创建默认配置");
        let mut default = AppConfig::default();
        default.teleagent_data_dir = get_default_data_dir();
        // 写入默认配置文件
        let _ = save_config_to_file(&default);
        default
    };

    // 存入全局实例
    {
        let mut guard = CONFIG.write().unwrap();
        *guard = Some(config.clone());
    }

    config
}

/// 补齐缺失字段的默认值
fn normalize_config(cfg: &mut AppConfig) {
    let default = AppConfig::default();
    if cfg.session_poll_interval == 0 {
        cfg.session_poll_interval = default.session_poll_interval;
    }
    if cfg.job_poll_interval == 0 {
        cfg.job_poll_interval = default.job_poll_interval;
    }
    if cfg.max_sessions == 0 {
        cfg.max_sessions = default.max_sessions;
    }
    if cfg.stale_threshold_minutes == 0 {
        cfg.stale_threshold_minutes = default.stale_threshold_minutes;
    }
}

/// 获取默认 TeleAgent 数据目录
fn get_default_data_dir() -> String {
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".local").join("share").join("TeleAgent");
        return path.to_string_lossy().to_string();
    }
    String::new()
}

/// 保存配置到文件并更新全局实例
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let mut config = config.clone();
    normalize_config(&mut config);

    save_config_to_file(&config)?;

    // 更新全局实例
    {
        let mut guard = CONFIG.write().unwrap();
        *guard = Some(config.clone());
    }

    info!("[config] 配置已保存并更新: {:?}", config);
    Ok(())
}

/// 将配置写入文件
fn save_config_to_file(config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path();
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, &content).map_err(|e| format!("写入配置文件失败: {}", e))?;

    info!("[config] 配置已写入: {:?}", config_path);
    Ok(())
}

/// 获取当前配置的只读副本（便捷方法）
pub fn get_config() -> AppConfig {
    load_config()
}

/// 获取 TeleAgent 数据目录路径（从配置读取）
pub fn get_teleagent_data_dir() -> Result<PathBuf, String> {
    let config = get_config();

    let data_dir = if config.teleagent_data_dir.is_empty() {
        // 使用默认值
        let home = dirs::home_dir()
            .ok_or_else(|| "无法获取用户主目录".to_string())?;
        home.join(".local").join("share").join("TeleAgent")
    } else {
        PathBuf::from(&config.teleagent_data_dir)
    };

    if !data_dir.exists() {
        return Err(format!(
            "TeleAgent 数据目录不存在: {:?}\n请确认 TeleAgent 已安装并运行过至少一次\n可在设置面板中修改数据目录路径",
            data_dir
        ));
    }

    Ok(data_dir)
}

/// 获取超时阈值（毫秒）
pub fn get_stale_threshold_ms() -> i64 {
    let config = get_config();
    config.stale_threshold_minutes * 60 * 1000
}

/// 获取最大会话数
pub fn get_max_sessions() -> usize {
    let config = get_config();
    config.max_sessions
}

/// 获取会话轮询间隔（毫秒）
pub fn get_session_poll_interval() -> u64 {
    let config = get_config();
    config.session_poll_interval
}

/// 获取定时任务轮询间隔（毫秒）
pub fn get_job_poll_interval() -> u64 {
    let config = get_config();
    config.job_poll_interval
}
