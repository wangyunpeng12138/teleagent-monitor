import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { SessionInfo, SchedulerJob, MonitorData } from "./types";
import { LOG } from "./logger";

/** 前端配置接口（镜像后端 AppConfig） */
export interface AppConfig {
  teleagent_data_dir: string;
  session_poll_interval: number;
  job_poll_interval: number;
  max_sessions: number;
  stale_threshold_minutes: number;
}

/** 默认配置值（与后端一致） */
export const DEFAULT_CONFIG: AppConfig = {
  teleagent_data_dir: "",
  session_poll_interval: 3000,
  job_poll_interval: 10000,
  max_sessions: 15,
  stale_threshold_minutes: 10,
};

/**
 * 数据采集 Hook
 * 负责调用 Rust 后端命令获取会话/定时任务数据，并定时刷新
 */
export function useMonitor() {
  const sessions = ref<SessionInfo[]>([]);
  const jobs = ref<SchedulerJob[]>([]);
  const lastUpdated = ref<number>(0);
  const loading = ref(false);
  const agentOnline = ref(false);
  const errorMsg = ref<string | null>(null);

  let sessionTimer: number | undefined;
  let jobTimer: number | undefined;
  let unlistenFs: UnlistenFn | undefined;

  /** 刷新交互会话列表 */
  async function refreshSessions() {
    try {
      loading.value = true;
      const data = await invoke<SessionInfo[]>("fetch_running_sessions");
      sessions.value = data;
      agentOnline.value = true;
      errorMsg.value = null;
      lastUpdated.value = Date.now();
      LOG.info(`[useMonitor] 会话刷新成功，共 ${data.length} 个会话`);
    } catch (e: any) {
      LOG.error(`[useMonitor] 刷新会话失败: ${e}`);
      errorMsg.value = String(e);
      // 如果是数据库/文件读取错误，不代表 agent 离线
      // 只有文件不存在才可能离线
      if (String(e).includes("不存在") || String(e).includes("not found")) {
        agentOnline.value = false;
      }
    } finally {
      loading.value = false;
    }
  }

  /** 刷新定时任务列表 */
  async function refreshJobs() {
    try {
      const data = await invoke<SchedulerJob[]>("fetch_scheduler_jobs");
      jobs.value = data;
      LOG.info(`[useMonitor] 定时任务刷新成功，共 ${data.length} 个启用任务`);
    } catch (e: any) {
      LOG.error(`[useMonitor] 刷新定时任务失败: ${e}`);
    }
  }

  /** 全量刷新 */
  async function refreshAll() {
    await Promise.all([refreshSessions(), refreshJobs()]);
  }

  /** 从后端加载配置并启动轮询 */
  async function startPolling() {
    let sessionInterval = DEFAULT_CONFIG.session_poll_interval;
    let jobInterval = DEFAULT_CONFIG.job_poll_interval;

    try {
      const cfg = await invoke<AppConfig>("get_config");
      sessionInterval = cfg.session_poll_interval || sessionInterval;
      jobInterval = cfg.job_poll_interval || jobInterval;
      LOG.info(
        `[useMonitor] 配置加载: session=${sessionInterval}ms, job=${jobInterval}ms`,
      );
    } catch (e) {
      LOG.warn(`[useMonitor] 加载配置失败，使用默认值: ${e}`);
    }

    sessionTimer = window.setInterval(refreshSessions, sessionInterval);
    jobTimer = window.setInterval(refreshJobs, jobInterval);
  }

  /** 重新启动轮询（配置变更后调用） */
  async function restartPolling() {
    if (sessionTimer) {
      clearInterval(sessionTimer);
      sessionTimer = undefined;
    }
    if (jobTimer) {
      clearInterval(jobTimer);
      jobTimer = undefined;
    }
    await startPolling();
    LOG.info("[useMonitor] 轮询已根据新配置重启");
  }

  onMounted(async () => {
    LOG.info("[useMonitor] 组件挂载，开始初始加载...");
    await refreshAll();

    // 从配置启动定时轮询
    await startPolling();

    // 监听 Rust 端的文件变更事件
    try {
      unlistenFs = await listen("session-status-changed", (event) => {
        LOG.info(`[useMonitor] 收到文件变更事件: ${event.payload}`);
        refreshSessions();
      });
      LOG.info("[useMonitor] 文件监听已注册");
    } catch (e) {
      LOG.warn(`[useMonitor] 文件监听注册失败（非致命）: ${e}`);
    }

    LOG.info("[useMonitor] 初始化完成");
  });

  onUnmounted(() => {
    if (sessionTimer) {
      clearInterval(sessionTimer);
      LOG.info("[useMonitor] 会话定时器已清除");
    }
    if (jobTimer) {
      clearInterval(jobTimer);
      LOG.info("[useMonitor] 任务定时器已清除");
    }
    if (unlistenFs) {
      unlistenFs();
      LOG.info("[useMonitor] 文件监听已注销");
    }
  });

  return {
    sessions,
    jobs,
    lastUpdated,
    loading,
    agentOnline,
    errorMsg,
    refreshAll,
    restartPolling,
  };
}
