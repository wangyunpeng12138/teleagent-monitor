<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useMonitor } from "./composables/useMonitor";
import { useWindowDrag } from "./composables/useWindowDrag";
import { LOG } from "./composables/logger";
import SessionCard from "./components/SessionCard.vue";
import SchedulerPanel from "./components/SchedulerPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import type { SessionStatus } from "./composables/types";

const { sessions, jobs, lastUpdated, loading, agentOnline, errorMsg, refreshAll, restartPolling } =
  useMonitor();
const { startDrag } = useWindowDrag();

// 设置面板
const showSettings = ref(false);

// 形态切换：'panel' 面板模式 | 'ball' 加速球模式
const viewMode = ref<"panel" | "ball">("ball");

// 标签过滤
const activeTag = ref<string>("全部");

const allTags = computed(() => {
  const set = new Set<string>();
  for (const s of sessions.value) {
    if (s.tag) set.add(s.tag);
  }
  return ["全部", ...Array.from(set).sort()];
});

const filteredSessions = computed(() => {
  if (activeTag.value === "全部") return sessions.value;
  return sessions.value.filter((s) => s.tag === activeTag.value);
});

// 状态统计
const statusCounts = computed(() => {
  let running = 0, needs_human = 0, completed = 0, terminated = 0;
  for (const s of sessions.value) {
    if (s.status === "running") running++;
    else if (s.status === "needs_human") needs_human++;
    else if (s.status === "terminated") terminated++;
    else completed++;
  }
  return { running, needs_human, completed, terminated };
});

function setTag(tag: string) {
  activeTag.value = tag;
  LOG.info(`[App] 切换标签: ${tag}`);
}

function formatClock(ts: number): string {
  if (!ts) return "--:--:--";
  const d = new Date(ts);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}`;
}

async function closeWindow() {
  LOG.info("[App] 用户请求关闭窗口");
  try {
    const win = getCurrentWindow();
    await win.hide();
    LOG.info("[App] 窗口已隐藏");
  } catch (e) {
    LOG.error(`[App] 关闭窗口失败: ${e}`);
  }
}

async function onSettingsSaved() {
  LOG.info("[App] 配置已保存，重启轮询并刷新数据");
  await restartPolling();
  await refreshAll();
  // 配置可能影响内容数量，重新自适应
  await nextTick();
  await fitWindowToContent();
}

// === 形态切换 ===

/** 切换到加速球模式：隐藏面板窗口，显示加速球窗口 */
async function switchToBall() {
  LOG.info("[App] 切换到加速球模式");
  viewMode.value = "ball";
  try {
    const win = getCurrentWindow();
    await win.hide();

    // 显示加速球窗口
    const windows = await getAllWindows();
    const ballWin = windows.find((w) => w.label === "ball");
    if (ballWin) {
      await ballWin.show();
      LOG.info("[App] 加速球窗口已显示");
    }
  } catch (e) {
    LOG.error(`[App] 切换加速球失败: ${e}`);
  }
}

/** 切换到面板模式：显示面板窗口，隐藏加速球窗口 */
async function switchToPanel() {
  LOG.info("[App] 切换到面板模式");
  viewMode.value = "panel";
  try {
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();

    // 窗口尺寸自适应内容
    await nextTick();
    await fitWindowToContent();

    // 隐藏加速球窗口
    const windows = await getAllWindows();
    const ballWin = windows.find((w) => w.label === "ball");
    if (ballWin) {
      await ballWin.hide();
      LOG.info("[App] 加速球窗口已隐藏");
    }
  } catch (e) {
    LOG.error(`[App] 切换面板失败: ${e}`);
  }
}

// 监听托盘"设置"菜单事件 + 加速球"切到面板"事件
let unlistenSettings: UnlistenFn | undefined;
let unlistenBallClick: UnlistenFn | undefined;

onMounted(async () => {
  try {
    unlistenSettings = await listen("open-settings", () => {
      showSettings.value = true;
      LOG.info("[App] 收到托盘设置事件，打开设置面板");
    });
  } catch (e) {
    LOG.warn(`[App] 监听 open-settings 失败: ${e}`);
  }

  try {
    unlistenBallClick = await listen("ball-click-panel", async () => {
      LOG.info("[App] 收到加速球点击事件，切换到面板模式");
      await switchToPanel();
    });
  } catch (e) {
    LOG.warn(`[App] 监听 ball-click-panel 失败: ${e}`);
  }

  // 窗口尺寸自适应内容：消除透明区域遮挡其他软件
  // 窗口 transparent + 固定 height 时，内容不足会留出透明区域遮挡桌面
  await nextTick();
  fitWindowToContent();
});

/** 根据内容实际高度调整窗口大小，消除底部透明遮挡 */
async function fitWindowToContent() {
  try {
    const win = getCurrentWindow();
    const container = document.querySelector(".container") as HTMLElement;
    if (!container) return;
    const w = container.offsetWidth;
    const h = container.offsetHeight;
    await win.setSize(new LogicalSize(w, h));
    LOG.info(`[App] 面板窗口尺寸已自适应为 ${w}x${h}`);
  } catch (e) {
    LOG.warn(`[App] 窗口尺寸自适应失败: ${e}`);
  }
}

// 数据变化后重新自适应窗口高度（初始加载、刷新、标签切换等）
watch([sessions, jobs, activeTag], async () => {
  await nextTick();
  await fitWindowToContent();
});
</script>

<template>
  <div class="container">
    <!-- 标题栏（拖拽区域） -->
    <div class="title-bar" @mousedown="startDrag($event)">
      <div class="title-left">
        <span class="title-icon">{{ agentOnline ? "🤖" : "💤" }}</span>
        <span class="title-text">TeleAgent Monitor</span>
        <!-- 状态摘要 -->
        <span v-if="agentOnline && sessions.length > 0" class="status-summary">
          <span v-if="statusCounts.running > 0" class="summary-running">{{ statusCounts.running }}运行</span>
          <span v-if="statusCounts.needs_human > 0" class="summary-needs-human">{{ statusCounts.needs_human }}需介入</span>
          <span v-if="statusCounts.terminated > 0" class="summary-terminated">{{ statusCounts.terminated }}已终止</span>
        </span>
      </div>
      <div class="title-right">
        <span v-if="errorMsg" class="error-dot" :title="errorMsg">⚠️</span>
        <span class="clock">{{ formatClock(lastUpdated) }}</span>
        <button class="btn-icon" @click.stop="showSettings = true" title="设置">⚙️</button>
        <button class="btn-icon" @click.stop="refreshAll" title="刷新">🔄</button>
        <button class="btn-icon" @click.stop="switchToBall" title="最小化为加速球">🔵</button>
        <button class="btn-icon btn-close" @click.stop="closeWindow" title="隐藏到托盘">✕</button>
      </div>
    </div>

    <!-- 内容区 -->
    <div class="content-area">
      <!-- 错误提示 -->
      <div v-if="errorMsg" class="error-banner">
        ⚠️ {{ errorMsg }}
      </div>

      <!-- 离线提示 -->
      <div v-if="!agentOnline && !errorMsg" class="offline-banner">
        💤 TeleAgent 未运行或数据目录不可用
      </div>

      <template v-if="agentOnline">
        <!-- 标签切换栏 -->
        <div v-if="allTags.length > 1" class="tag-bar">
          <button
            v-for="tag in allTags"
            :key="tag"
            class="tag-btn"
            :class="{ active: activeTag === tag }"
            @click="setTag(tag)"
          >
            {{ tag }}
          </button>
        </div>

        <!-- 交互会话区块 -->
        <div v-if="filteredSessions.length > 0" class="section">
          <div class="section-header">📋 会话 ({{ filteredSessions.length }})</div>
          <div class="section-body session-scroll">
            <SessionCard
              v-for="sess in filteredSessions"
              :key="sess.session_id"
              :session="sess"
            />
          </div>
        </div>

        <!-- 定时任务区块 -->
        <div v-if="jobs.length > 0" class="section scheduler-section">
          <div class="section-header">⏰ 定时任务 ({{ jobs.length }})</div>
          <div class="section-body">
            <SchedulerPanel
              v-for="job in jobs"
              :key="job.id"
              :job="job"
            />
          </div>
        </div>

        <!-- 空状态 -->
        <div
          v-if="filteredSessions.length === 0 && jobs.length === 0 && !errorMsg"
          class="empty-state"
        >
          当前无会话
        </div>
      </template>
    </div>

    <!-- 设置面板 -->
    <SettingsPanel
      v-if="showSettings"
      @close="showSettings = false"
      @saved="onSettingsSaved"
    />
  </div>
</template>

<style scoped>
.container {
  width: 380px;
  max-height: 100vh;
  background: rgba(30, 30, 35, 0.92);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

/* === 标题栏 === */
.title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: rgba(255, 255, 255, 0.03);
  cursor: grab;
  user-select: none;
  flex-shrink: 0;
}

.title-bar:active {
  cursor: grabbing;
}

.title-left {
  display: flex;
  align-items: center;
  gap: 5px;
}

.title-icon {
  font-size: 14px;
}

.title-text {
  color: #e0e0e0;
  font-size: 13px;
  font-weight: 700;
}

/* === 状态摘要 === */
.status-summary {
  display: flex;
  gap: 3px;
  margin-left: 4px;
}

.summary-running {
  background: rgba(64, 158, 255, 0.2);
  color: #409eff;
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  font-weight: 600;
}

.summary-needs-human {
  background: rgba(240, 192, 64, 0.2);
  color: #f0c040;
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  font-weight: 600;
}

.summary-terminated {
  background: rgba(245, 108, 108, 0.2);
  color: #f56c6c;
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  font-weight: 600;
}

.title-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.clock {
  color: #777;
  font-size: 11px;
  margin-right: 4px;
  font-variant-numeric: tabular-nums;
}

.error-dot {
  font-size: 12px;
  margin-right: 2px;
}

.btn-icon {
  background: rgba(255, 255, 255, 0.06);
  color: #aaa;
  border: none;
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 10px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-icon:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.btn-close:hover {
  background: rgba(245, 108, 108, 0.3);
  color: #f56c6c;
}

/* === 标签栏 === */
.tag-bar {
  display: flex;
  gap: 3px;
  padding: 4px 8px;
  overflow-x: auto;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.tag-bar::-webkit-scrollbar {
  height: 0;
}

.tag-btn {
  background: rgba(255, 255, 255, 0.04);
  color: #888;
  border: none;
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 10px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s;
}

.tag-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ccc;
}

.tag-btn.active {
  background: rgba(64, 158, 255, 0.2);
  color: #409eff;
  font-weight: 600;
}

/* === 内容区 === */
.content-area {
  flex: 1;
  overflow-y: auto;
  padding: 6px 8px 8px;
}

.error-banner {
  background: rgba(245, 108, 108, 0.1);
  border: 1px solid rgba(245, 108, 108, 0.2);
  border-radius: 4px;
  padding: 4px 8px;
  color: #f56c6c;
  font-size: 11px;
  margin-bottom: 6px;
}

.offline-banner {
  text-align: center;
  color: #888;
  font-size: 12px;
  padding: 20px 0;
}

.empty-state {
  text-align: center;
  color: #666;
  font-size: 12px;
  padding: 30px 0;
  font-style: italic;
}

/* === 区块 === */
.section {
  margin-bottom: 8px;
}

.section-header {
  color: #909399;
  font-size: 11px;
  font-weight: 600;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  padding-bottom: 3px;
  margin-bottom: 5px;
}

.section-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* === 会话滚动区 ===
   会话列表设固定最大高度，内部滚动，
   确保定时任务区块始终在下方可见 */
.session-scroll {
  max-height: 280px;
  overflow-y: auto;
  padding-right: 2px;
}

.session-scroll::-webkit-scrollbar {
  width: 4px;
}

.session-scroll::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}

.session-scroll::-webkit-scrollbar-track {
  background: transparent;
}

/* 定时任务区块固定在会话区下方 */
.scheduler-section {
  margin-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 4px;
}
</style>
