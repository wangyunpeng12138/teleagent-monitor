<script setup lang="ts">
import { ref } from "vue";
import type { SessionInfo, SessionStatus } from "../composables/types";
import TodoItem from "./TodoItem.vue";
import { LOG } from "../composables/logger";

const props = defineProps<{
  session: SessionInfo;
}>();

/** 折叠状态：默认折叠 */
const collapsed = ref(true);

/** 切换折叠/展开 */
function toggleCollapse() {
  collapsed.value = !collapsed.value;
  LOG.info(`[SessionCard] 会话 ${props.session.session_id} ${collapsed.value ? "折叠" : "展开"}`);
}

/** 格式化时间戳为 HH:MM:SS */
function formatTime(ts: number): string {
  if (!ts || ts <= 0) return "--";
  const d = new Date(ts);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

/** 统计 todo 完成情况 */
function todoStats(): { total: number; done: number; inProgress: number; cancelled: number } {
  if (!props.session.todos || props.session.todos.length === 0) {
    return { total: 0, done: 0, inProgress: 0, cancelled: 0 };
  }
  const total = props.session.todos.length;
  const done = props.session.todos.filter((t) => t.status === "completed").length;
  const inProgress = props.session.todos.filter((t) => t.status === "in_progress").length;
  const cancelled = props.session.todos.filter((t) => t.status === "cancelled").length;
  return { total, done, inProgress, cancelled };
}

/** 进度百分比：已完成的会话固定 100%，否则按实际完成率计算 */
function progressPercent(): number {
  if (props.session.status === "completed") return 100;
  const { total, done } = todoStats();
  if (total === 0) return 0;
  return Math.round((done / total) * 100);
}

/** 状态元数据 */
const STATUS_META: Record<SessionStatus, { dotClass: string; label: string }> = {
  running: { dotClass: "dot-running", label: "运行中" },
  needs_human: { dotClass: "dot-needs-human", label: "需介入" },
  completed: { dotClass: "dot-completed", label: "已完成" },
  terminated: { dotClass: "dot-terminated", label: "已终止" },
};

function statusMeta(): { dotClass: string; label: string } {
  return STATUS_META[props.session.status] ?? STATUS_META.completed;
}

/** 进度徽章样式 */
function progressClass(): string {
  const { total, done, cancelled } = todoStats();
  if (total === 0) return "";
  // 全部完成或已取消 → 绿色
  if (done + cancelled === total) return "progress-done";
  // terminated（手动终止）→ 红色
  if (props.session.status === "terminated") return "progress-terminated";
  return "progress-active";
}

/** 折叠时的进度百分比文本 */
function progressText(): string {
  const { total, done, inProgress, cancelled } = todoStats();
  if (total === 0) return "";
  // 会话已结束且子任务已同步 → 显示最终状态，不含"进行中"
  if ((props.session.status === "completed" || props.session.status === "terminated") && inProgress === 0) {
    if (done + cancelled === total) {
      return `${done}/${total} (${Math.round((done / total) * 100)}%)`;
    }
    return `${done}/${total} (${Math.round((done / total) * 100)}%)`;
  }
  // 运行中/需介入 → 显示实际进度
  const pct = Math.round((done / total) * 100);
  let text = `${done}/${total} (${pct}%)`;
  if (inProgress > 0) text += ` · ${inProgress}进行中`;
  return text;
}
</script>

<template>
  <div class="session-card fade-in" @click="toggleCollapse" :title="collapsed ? '点击展开任务详情' : '点击折叠'">
    <!-- 标题行 -->
    <div class="session-header">
      <span class="collapse-indicator">{{ collapsed ? '▶' : '▼' }}</span>
      <span class="status-dot" :class="statusMeta().dotClass"></span>
      <span class="session-title" :title="session.title">
        {{ session.title || "(未命名会话)" }}
      </span>
      <!-- 折叠时在标题行右侧显示进度摘要 -->
      <span
        v-if="collapsed && todoStats().total > 0"
        class="progress-summary"
        :class="progressClass()"
      >{{ progressText() }}</span>
    </div>

    <!-- 元信息行 -->
    <div class="session-meta">
      <span class="dir-badge" :title="session.directory">
        📁 {{ session.tag }}
      </span>
      <span class="status-label" :class="statusMeta().dotClass">
        {{ statusMeta().label }}
      </span>
      <span class="time-badge">⏱ {{ formatTime(session.updated_at) }}</span>
      <span v-if="todoStats().total > 0 && !collapsed" class="progress-badge" :class="progressClass()">
        {{ todoStats().done }}/{{ todoStats().total }}
      </span>
    </div>

    <!-- Todo 列表（展开时显示） -->
    <div v-if="!collapsed && session.todos && session.todos.length > 0" class="todo-list">
      <TodoItem
        v-for="todo in session.todos"
        :key="todo.position"
        :todo="todo"
      />
    </div>
    <div v-else-if="!collapsed && session.status !== 'completed'" class="no-todo">
      暂无任务记录
    </div>
  </div>
</template>

<style scoped>
.session-card {
  background: rgba(255, 255, 255, 0.04);
  border-radius: 6px;
  padding: 6px 8px;
  margin-bottom: 6px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.session-card:hover {
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(64, 158, 255, 0.3);
}

.session-header {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-bottom: 3px;
}

/* === 折叠指示器（非交互，仅展示） === */
.collapse-indicator {
  width: 14px;
  flex-shrink: 0;
  text-align: center;
  color: #888;
  font-size: 9px;
  user-select: none;
  line-height: 1;
}

/* === 状态圆点 === */
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-running {
  background: #409eff;
  box-shadow: 0 0 6px rgba(64, 158, 255, 0.6);
  animation: pulse 1.5s ease-in-out infinite;
}

.dot-needs-human {
  background: #f0c040;
  box-shadow: 0 0 6px rgba(240, 192, 64, 0.6);
  animation: pulse 1s ease-in-out infinite;
}

.dot-completed {
  background: #67c23a;
}

.dot-terminated {
  background: #f56c6c;
  box-shadow: 0 0 6px rgba(245, 108, 108, 0.6);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.session-title {
  color: #e0e0e0;
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

/* === 折叠时的进度摘要 === */
.progress-summary {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 3px;
  white-space: nowrap;
}

.progress-summary.progress-active {
  background: rgba(64, 158, 255, 0.15);
  color: #409eff;
}

.progress-summary.progress-done {
  background: rgba(103, 194, 58, 0.1);
  color: #67c23a;
}

.progress-summary.progress-terminated {
  background: rgba(245, 108, 108, 0.15);
  color: #f56c6c;
}

.session-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
  font-size: 10px;
  color: #777;
}

.dir-badge {
  background: rgba(255, 255, 255, 0.06);
  padding: 1px 5px;
  border-radius: 3px;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* === 状态标签 === */
.status-label {
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 9px;
  font-weight: 600;
}

.status-label.dot-running {
  background: rgba(64, 158, 255, 0.15);
  color: #409eff;
}

.status-label.dot-needs-human {
  background: rgba(240, 192, 64, 0.15);
  color: #f0c040;
}

.status-label.dot-completed {
  background: rgba(103, 194, 58, 0.1);
  color: #67c23a;
}

.status-label.dot-terminated {
  background: rgba(245, 108, 108, 0.15);
  color: #f56c6c;
}

.time-badge {
  color: #666;
  margin-left: auto;
}

/* === 进度徽章 === */
.progress-badge {
  padding: 1px 5px;
  border-radius: 3px;
  font-weight: 600;
}

.progress-active {
  background: rgba(64, 158, 255, 0.15);
  color: #409eff;
}

.progress-done {
  background: rgba(103, 194, 58, 0.1);
  color: #67c23a;
}

.todo-list {
  padding-left: 14px;
}

.no-todo {
  padding-left: 14px;
  color: #555;
  font-size: 11px;
  font-style: italic;
}
</style>
