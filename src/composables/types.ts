/** 前端类型定义 */

export type SessionStatus = "running" | "needs_human" | "completed" | "terminated";

export interface TodoInfo {
  content: string;
  status: "pending" | "in_progress" | "completed" | "cancelled";
  priority: "high" | "medium" | "low" | "";
  position: number;
}

export interface SessionInfo {
  session_id: string;
  title: string;
  directory: string;
  tag: string;
  updated_at: number;
  status: SessionStatus;
  todos: TodoInfo[];
}

export interface SchedulerJob {
  id: string;
  name: string;
  enabled: number;
  cron_expr: string | null;
  run_at: string | null;
  last_run_at: string | null;
  last_status: string | null;
  last_session_id: string | null;
}

export interface MonitorData {
  sessions: SessionInfo[];
  jobs: SchedulerJob[];
  last_updated: number;
  agent_online: boolean;
  error: string | null;
}
