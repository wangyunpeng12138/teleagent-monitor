//! 数据库读取模块
//!
//! 以只读模式打开 TeleAgent 的 SQLite 数据库，查询会话和定时任务状态。
//! 使用 `immutable=1` URI 模式，完全不加文件锁，不会干扰 TeleAgent 正常运行。

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use log::{info, warn, error};

use crate::config;

/// 单个 Todo 项
#[derive(Serialize, Deserialize, Debug)]
pub struct TodoInfo {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub position: i64,
}

/// 会话信息（带 todo 列表）
#[derive(Serialize, Deserialize, Debug)]
pub struct SessionInfo {
    pub session_id: String,
    pub title: String,
    pub directory: String,
    pub tag: String,
    pub updated_at: i64,
    pub status: String, // running | needs_human | completed | terminated
    pub todos: Vec<TodoInfo>,
}

/// 定时任务信息
#[derive(Serialize, Deserialize, Debug)]
pub struct SchedulerJob {
    pub id: String,
    pub name: String,
    pub enabled: i64,
    pub cron_expr: Option<String>,
    pub run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_session_id: Option<String>,
}

/// 获取 TeleAgent 用户数据目录（从配置读取）
fn get_teleagent_data_dir() -> Result<PathBuf, String> {
    config::get_teleagent_data_dir()
}

/// 获取 teleagent.db 路径
fn get_db_path() -> Result<PathBuf, String> {
    let dir = get_teleagent_data_dir()?;
    let db_path = dir.join("teleagent.db");
    if !db_path.exists() {
        return Err(format!("数据库文件不存在: {:?}", db_path));
    }
    Ok(db_path)
}

/// 获取 session-status.json 路径
fn get_session_status_path() -> Result<PathBuf, String> {
    let dir = get_teleagent_data_dir()?;
    Ok(dir.join("session-status.json"))
}

/// 获取 scheduler.db 路径
fn get_scheduler_db_path() -> Result<PathBuf, String> {
    let dir = get_teleagent_data_dir()?;
    let db_path = dir.join("scheduler").join("scheduler.db");
    if !db_path.exists() {
        warn!("scheduler.db 不存在: {:?}", db_path);
        return Err(format!("scheduler.db 不存在: {:?}", db_path));
    }
    Ok(db_path)
}

/// 获取 deleted-session-ids.json 路径
fn get_deleted_session_ids_path() -> Result<PathBuf, String> {
    let dir = get_teleagent_data_dir()?;
    Ok(dir.join("deleted-session-ids.json"))
}

/// 以只读 immutable 模式打开 SQLite 连接
fn open_db_readonly(path: &PathBuf) -> Result<Connection, String> {
    let path_str = path.to_str().ok_or("数据库路径包含非法字符")?;
    let uri = format!("file:{}?mode=ro&immutable=1", path_str.replace('\\', "/"));

    match Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_URI,
    ) {
        Ok(conn) => {
            info!("数据库已打开(immutable): {}", path_str);
            Ok(conn)
        }
        Err(e) => {
            warn!("immutable 模式打开失败 ({}), 尝试普通只读模式", e);
            let uri2 = format!("file:{}?mode=ro", path_str.replace('\\', "/"));
            Connection::open_with_flags(
                &uri2,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_URI,
            ).map_err(|e2| format!("打开数据库失败: {} (回退模式也失败: {})", e, e2))
        }
    }
}

/// 读取 session-status.json，返回 {session_id: status} 映射
fn read_session_status() -> Result<std::collections::HashMap<String, String>, String> {
    let path = get_session_status_path()?;

    if !path.exists() {
        info!("session-status.json 不存在，TeleAgent 可能未运行");
        return Ok(std::collections::HashMap::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取 session-status.json 失败: {}", e))?;

    let parsed: std::collections::HashMap<String, String> = if content.trim().is_empty() {
        warn!("session-status.json 为空");
        std::collections::HashMap::new()
    } else {
        serde_json::from_str(&content).map_err(|e| {
            format!("解析 session-status.json 失败: {} (内容可能正在写入)", e)
        })?
    };

    info!("session-status.json 解析成功，{} 个会话", parsed.len());
    Ok(parsed)
}

/// 读取 deleted-session-ids.json，返回已删除会话 ID 集合
fn read_deleted_session_ids() -> std::collections::HashSet<String> {
    let path = match get_deleted_session_ids_path() {
        Ok(p) => p,
        Err(_) => return std::collections::HashSet::new(),
    };

    if !path.exists() {
        return std::collections::HashSet::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("读取 deleted-session-ids.json 失败: {}", e);
            return std::collections::HashSet::new();
        }
    };

    // 格式: {"version":1,"deletedSessionIds":{"ses_xxx": timestamp, ...}}
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            warn!("解析 deleted-session-ids.json 失败: {}", e);
            return std::collections::HashSet::new();
        }
    };

    let mut set = std::collections::HashSet::new();
    if let Some(ids) = parsed.get("deletedSessionIds").and_then(|v| v.as_object()) {
        for key in ids.keys() {
            set.insert(key.clone());
        }
    }
    info!("已删除会话 ID 加载: {} 个", set.len());
    set
}

/// 将 directory 路径映射为短标签名
fn directory_to_tag(directory: &str) -> String {
    if directory.is_empty() {
        return "global".to_string();
    }

    // 取路径最后一级目录名
    let normalized = directory.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        return "global".to_string();
    }

    // 已知的短映射
    let last = parts[parts.len() - 1];
    match last {
        "mes" => "mes".to_string(),
        "test" => "test".to_string(),
        "daai" => "daai".to_string(),
        "algos" => "algos".to_string(),
        "niuma-game" => "niuma".to_string(),
        "game-world" => "game-world".to_string(),
        "Safe-operation-platform" => "safe-platform".to_string(),
        _ => last.to_string(),
    }
}

/// 查询最近的主会话列表
/// 过滤逻辑：
///   - parent_id IS NULL（排除 subagent 子会话）
///   - title NOT LIKE '%_SYS_%'（排除系统内部会话）
///   - time_archived IS NULL OR 0（排除已归档）
///   - 不在 deleted-session-ids.json 中
///   - 最近 15 条
pub fn fetch_running_sessions() -> Result<Vec<SessionInfo>, String> {
    info!("[db] fetch_recent_sessions 被调用");

    // 1. 读取 session-status.json（作为 running 状态的补充标记）
    let status_map = read_session_status()?;

    // 2. 读取已删除会话 ID
    let deleted_ids = read_deleted_session_ids();

    // 3. 打开 teleagent.db
    let db_path = get_db_path()?;
    let conn = open_db_readonly(&db_path)?;

    // 4. 查询最近的主会话（多取一些，后面过滤已删除的）
    let query_limit = (config::get_max_sessions() + 5) as i64; // 多取 5 条用于过滤
    let sql = format!(
        "SELECT id, title, directory, time_updated
         FROM session
         WHERE parent_id IS NULL
           AND title NOT LIKE '%_SYS_%'
           AND (time_archived IS NULL OR time_archived = 0)
         ORDER BY time_updated DESC
         LIMIT {}", query_limit);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("准备 session 查询失败: {}", e))?;

    let session_rows: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| format!("查询 session 失败: {}", e))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                warn!("读取 session 行失败: {}", e);
                None
            }
        })
        .collect();

    info!("[db] 查询到 {} 条候选 session 记录", session_rows.len());

    // 5. 对每个 session 推导状态 + 查询 todo
    let mut result = Vec::new();
    for (session_id, title, directory, updated_at) in &session_rows {
        // 过滤已删除的会话
        if deleted_ids.contains(session_id) {
            info!("[db] 跳过已删除会话: {}", session_id);
            continue;
        }

        // 限制最大会话数（从配置读取）
        let max_sessions = config::get_max_sessions();
        if result.len() >= max_sessions {
            break;
        }

        // 推导状态
        let (status, is_definitive) = derive_session_status(&conn, session_id, &status_map);

        // 查询 todo
        let mut todos = match fetch_todos_for_session(&conn, session_id) {
            Ok(t) => t,
            Err(e) => {
                warn!("查询 session {} 的 todo 失败: {}", session_id, e);
                vec![]
            }
        };

        // 仅当 finish=stop 确认结束时同步子任务状态
        // （超时猜测不破坏子任务原始状态，避免会话恢复后子任务列表损坏）
        // terminated（手动终止）也视为确认结束，同步子任务
        if is_definitive || status == "terminated" {
            for todo in &mut todos {
                if todo.status == "in_progress" {
                    info!("[db] 会话 {} 结束({})，子任务 '{}' in_progress → completed", session_id, status, todo.content);
                    todo.status = "completed".to_string();
                } else if todo.status == "pending" {
                    info!("[db] 会话 {} 结束({})，子任务 '{}' pending → cancelled", session_id, status, todo.content);
                    todo.status = "cancelled".to_string();
                }
            }
        }

        let tag = directory_to_tag(directory);

        result.push(SessionInfo {
            session_id: session_id.clone(),
            title: title.clone(),
            directory: directory.clone(),
            tag,
            updated_at: *updated_at,
            status,
            todos,
        });
    }

    info!("[db] 返回 {} 个会话信息", result.len());
    Ok(result)
}

/// 推导会话状态：running / needs_human / completed / terminated
///
/// 判定优先级：
///   1. 最后一条 message 的 finish=stop → completed（最可靠，确认结束）
///   2. session-status.json 中标记 paused 且在阈值内 → needs_human
///   3. session-status.json 中标记 paused 但超时 → terminated（用户手动终止后未恢复）
///   4. session-status.json 中标记 running 且在阈值内 → running / needs_human（取决于 role）
///   5. session-status.json 中标记 running 但超时 → completed（崩溃兜底）
///   6. 其他情况 → completed
///
/// 返回 (status, is_definitive)：
///   is_definitive=true 表示 finish=stop 确认结束（可安全同步子任务状态）
///   is_definitive=false 表示猜测状态（不应破坏子任务原始状态）
fn derive_session_status(
    conn: &Connection,
    session_id: &str,
    status_map: &std::collections::HashMap<String, String>,
) -> (String, bool) {
    let now = now_ts_millis();
    let stale_threshold_ms = crate::config::get_stale_threshold_ms();
    info!(
        "[db] derive_session_status: session={}, stale_threshold={}ms",
        session_id, stale_threshold_ms
    );

    // 获取最后一条消息的信息（role, finish, timestamp）
    let last_msg = get_last_message_info(conn, session_id);

    // 1. finish=stop → 一定完成（确认结束）
    if let Some((_role, finish, _ts)) = &last_msg {
        if finish == "stop" {
            info!("[db] 会话 {} finish=stop → completed (definitive)", session_id);
            return ("completed".to_string(), true);
        }
    }

    // 2. 检查 session-status.json
    let mapped_status = status_map.get(session_id).cloned();

    if let Some(ref s) = mapped_status {
        if s == "running" || s == "paused" {
            // 检查最后一条消息的时间，判断是否真的还在运行
            let msg_age = match &last_msg {
                Some((_role, _finish, ts)) => now - ts,
                None => i64::MAX, // 查不到消息，视为很久以前
            };

            if msg_age > stale_threshold_ms {
                // 超过阈值没有新消息
                // paused 超时 → terminated（用户手动终止后未恢复）
                // running 超时 → completed（崩溃兜底）
                if s == "paused" {
                    info!(
                        "[db] 会话 {} status=paused 但最后消息距今 {}ms > {}ms → terminated (手动终止)",
                        session_id, msg_age, stale_threshold_ms
                    );
                    return ("terminated".to_string(), false);
                } else {
                    info!(
                        "[db] 会话 {} status=running 但最后消息距今 {}ms > {}ms → completed (崩溃兜底)",
                        session_id, msg_age, stale_threshold_ms
                    );
                    return ("completed".to_string(), false);
                }
            }

            // 在阈值内，信任 session-status.json
            if s == "paused" {
                info!("[db] 会话 {} status=paused 且在阈值内 → needs_human", session_id);
                return ("needs_human".to_string(), false);
            }

            // running
            match &last_msg {
                Some((role, _finish, _ts)) => {
                    if role == "user" {
                        info!("[db] 会话 {} status=running, last_role=user → needs_human", session_id);
                        return ("needs_human".to_string(), false);
                    }
                    info!("[db] 会话 {} status=running, last_role={} → running", session_id, role);
                    return ("running".to_string(), false);
                }
                None => {
                    info!("[db] 会话 {} status=running, 无消息 → running", session_id);
                    return ("running".to_string(), false);
                }
            }
        }
    }

    // 3. 不在 session-status.json 中，或标记为 completed
    info!("[db] 会话 {} 无有效 status_map 标记 → completed (default)", session_id);
    ("completed".to_string(), false)
}

/// 获取会话最后一条 message 的 role、finish 和时间戳
fn get_last_message_info(conn: &Connection, session_id: &str) -> Option<(String, String, i64)> {
    let sql = "SELECT data, time_created FROM message WHERE session_id = ? ORDER BY time_created DESC LIMIT 1";
    let (raw, ts): (String, i64) = conn
        .query_row(sql, [session_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .ok()?;

    // 解析 JSON
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let role = parsed
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let finish = parsed
        .get("finish")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some((role, finish, ts))
}

/// 获取当前时间戳（毫秒）
fn now_ts_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 查询指定会话的 todo 列表
fn fetch_todos_for_session(conn: &Connection, session_id: &str) -> Result<Vec<TodoInfo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT content, status, priority, position
             FROM todo
             WHERE session_id = ?
             ORDER BY position ASC",
        )
        .map_err(|e| format!("准备 todo 查询失败: {}", e))?;

    let todos: Vec<TodoInfo> = stmt
        .query_map([session_id], |row| {
            Ok(TodoInfo {
                content: row.get::<_, String>(0)?,
                status: row.get::<_, String>(1)?,
                priority: row.get::<_, String>(2)?,
                position: row.get::<_, i64>(3)?,
            })
        })
        .map_err(|e| format!("查询 todo 失败: {}", e))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                warn!("读取 todo 行失败: {}", e);
                None
            }
        })
        .collect();

    Ok(todos)
}

/// 查询所有启用的定时任务
pub fn fetch_scheduler_jobs() -> Result<Vec<SchedulerJob>, String> {
    let db_path = match get_scheduler_db_path() {
        Ok(p) => p,
        Err(e) => {
            info!("scheduler.db 不可用: {}", e);
            return Ok(vec![]);
        }
    };

    let conn = open_db_readonly(&db_path)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, enabled, cron_expr, run_at, last_run_at, last_status, last_session_id
             FROM jobs
             WHERE enabled = 1
             ORDER BY last_run_at DESC",
        )
        .map_err(|e| format!("准备 jobs 查询失败: {}", e))?;

    let jobs: Vec<SchedulerJob> = stmt
        .query_map([], |row| {
            Ok(SchedulerJob {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                enabled: row.get::<_, i64>(2)?,
                cron_expr: row.get::<_, Option<String>>(3)?,
                run_at: row.get::<_, Option<String>>(4)?,
                last_run_at: row.get::<_, Option<String>>(5)?,
                last_status: row.get::<_, Option<String>>(6)?,
                last_session_id: row.get::<_, Option<String>>(7)?,
            })
        })
        .map_err(|e| format!("查询 jobs 失败: {}", e))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                warn!("读取 job 行失败: {}", e);
                None
            }
        })
        .collect();

    info!("查询到 {} 个启用的定时任务", jobs.len());

    Ok(jobs)
}

/// 检查 TeleAgent 是否在线（数据目录和核心文件存在）
pub fn check_agent_online() -> bool {
    match get_db_path() {
        Ok(path) => {
            let exists = path.exists();
            if !exists {
                error!("TeleAgent 数据目录存在但 teleagent.db 不存在");
            }
            exists
        }
        Err(_) => false,
    }
}

/// 获取 session-status.json 的完整路径（供 watcher 模块使用）
pub fn get_session_status_file_path() -> Result<PathBuf, String> {
    get_session_status_path()
}
