<script setup lang="ts">
import type { SchedulerJob } from "../composables/types";

const props = defineProps<{
  job: SchedulerJob;
}>();

const STATUS_META: Record<string, { text: string; color: string }> = {
  dispatch_success: { text: "✅ 成功", color: "#67C23A" },
  schedule_error: { text: "⚠️ 失败", color: "#F56C6C" },
};

function getStatus(): { text: string; color: string } {
  if (!props.job.last_status) return { text: "—", color: "#666" };
  const meta = STATUS_META[props.job.last_status];
  return meta || { text: props.job.last_status, color: "#999" };
}

/** 从 ISO 时间提取 HH:MM */
function formatTime(iso: string | null): string {
  if (!iso) return "";
  try {
    const d = new Date(iso);
    const hh = String(d.getHours()).padStart(2, "0");
    const mm = String(d.getMinutes()).padStart(2, "0");
    return `${hh}:${mm}`;
  } catch {
    return "";
  }
}

/** cron 表达式转中文简述 */
function cronLabel(cron: string | null, runAt: string | null): string {
  if (runAt) return "一次性";
  if (!cron) return "未知";
  // 简单解析常见 cron 模式
  const parts = cron.trim().split(/\s+/);
  if (parts.length < 5) return cron;
  const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;

  if (dayOfWeek === "1-5" && hour && minute) return `工作日 ${pad(hour)}:${pad(minute)}`;
  if (dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
    return `每天 ${pad(hour)}:${pad(minute)}`;
  }
  return cron;
}

function pad(s: string): string {
  const n = parseInt(s, 10);
  if (isNaN(n)) return s;
  return String(n).padStart(2, "0");
}
</script>

<template>
  <div class="job-row">
    <span class="job-name">
      <span class="job-icon">⏰</span>
      {{ job.name || "(未命名任务)" }}
    </span>
    <span class="job-info">
      <span class="cron-label">{{ cronLabel(job.cron_expr, job.run_at) }}</span>
      <span :style="{ color: getStatus().color }" class="status-label">
        {{ formatTime(job.last_run_at) }} {{ getStatus().text }}
      </span>
    </span>
  </div>
</template>

<style scoped>
.job-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 6px;
  font-size: 11px;
  border-radius: 4px;
}

.job-row:hover {
  background: rgba(255, 255, 255, 0.03);
}

.job-name {
  color: #b0b0b0;
  display: flex;
  align-items: center;
  gap: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.job-icon {
  flex-shrink: 0;
}

.job-info {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.cron-label {
  color: #555;
  font-size: 10px;
}

.status-label {
  font-size: 10px;
  white-space: nowrap;
}
</style>
