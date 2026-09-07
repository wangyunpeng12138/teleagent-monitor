/**
 * 前端日志模块
 * 通过 Tauri 后端 log 命令将日志写入文件，同时 console 输出
 */
import { invoke } from "@tauri-apps/api/core";

type LogLevel = "info" | "warn" | "error" | "debug";

function formatMsg(level: LogLevel, msg: string): string {
  const ts = new Date().toISOString();
  return `[${ts}] [${level.toUpperCase()}] ${msg}`;
}

async function sendToBackend(level: LogLevel, msg: string) {
  try {
    await invoke("write_log", { level, message: msg });
  } catch {
    // 后端不可用时静默失败，不影响前端逻辑
  }
}

export const LOG = {
  info(msg: string) {
    console.log(formatMsg("info", msg));
    sendToBackend("info", msg);
  },
  warn(msg: string) {
    console.warn(formatMsg("warn", msg));
    sendToBackend("warn", msg);
  },
  error(msg: string) {
    console.error(formatMsg("error", msg));
    sendToBackend("error", msg);
  },
  debug(msg: string) {
    console.debug(formatMsg("debug", msg));
    sendToBackend("debug", msg);
  },
};
