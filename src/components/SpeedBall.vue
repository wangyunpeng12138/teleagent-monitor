<script setup lang="ts">
/**
 * 加速球组件 —— 竖条形态
 * 上格子：🔄 刷新按钮（独占一格）
 * 下格子：红绿灯展示（蓝=运行中 / 黄=需介入 / 绿=空闲）
 * 点击红绿灯 → 通知主窗口切换到面板
 */
import { ref, computed, onMounted, onUnmounted } from "vue";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useMonitor } from "../composables/useMonitor";
import { useBallSnap } from "../composables/useBallSnap";
import { LOG } from "../composables/logger";

const { sessions, agentOnline, refreshAll } = useMonitor();
const { snapToEdge } = useBallSnap();

// 刷新中动画
const refreshing = ref(false);
let dragging = false;

/**
 * 拖拽加速球：仅在非交互格子区域触发
 * 注意：startDragging() 返回的 Promise 立即 resolve（不等待拖动结束），
 * 所以边缘吸附由 onMoved + 防抖实现（见下方监听）
 */
async function startDrag(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (target.closest(".cell")) return; // 交互格子不触发拖拽
  if (dragging) return;
  dragging = true;
  try {
    const win = getCurrentWindow();
    await win.startDragging();
    LOG.debug("[SpeedBall] startDragging 已调用");
  } catch (e) {
    LOG.error(`[SpeedBall] startDragging 失败: ${e}`);
  } finally {
    dragging = false;
  }
}

// 红绿灯状态：优先需介入 → 运行中 → 空闲绿
const lightState = computed<"green" | "blue" | "yellow">(() => {
  const running = sessions.value.filter((s) => s.status === "running").length;
  const needsHuman = sessions.value.filter((s) => s.status === "needs_human").length;
  if (needsHuman > 0) return "yellow";
  if (running > 0) return "blue";
  return "green";
});

const countText = computed(() => {
  const running = sessions.value.filter((s) => s.status === "running").length;
  const needsHuman = sessions.value.filter((s) => s.status === "needs_human").length;
  if (needsHuman > 0) return String(needsHuman);
  if (running > 0) return String(running);
  return "";
});

/** 点击红绿灯 → 通知主窗口切到面板 */
async function openPanel() {
  LOG.info("[SpeedBall] 点击红绿灯，请求打开面板");
  try {
    await emit("ball-click-panel");
  } catch (e) {
    LOG.error(`[SpeedBall] 发送面板事件失败: ${e}`);
  }
}

/** 点击刷新 → 刷新数据 */
async function onRefresh() {
  if (refreshing.value) return;
  refreshing.value = true;
  LOG.info("[SpeedBall] 手动刷新数据");
  try {
    await refreshAll();
  } finally {
    setTimeout(() => (refreshing.value = false), 600);
  }
}

// === 边缘吸附：onMoved + 防抖 ===
// 拖动过程中 onMoved 持续触发，防抖不断重置计时器 → 不会中途吸附
// 拖动停止 300ms 后判定结束 → 执行吸附
const SNAP_DEBOUNCE_MS = 300;
let snapTimer: number | undefined;
let dragUnlisten: (() => void) | undefined;

onMounted(async () => {
  try {
    const win = getCurrentWindow();
    dragUnlisten = await win.onMoved(async () => {
      if (snapTimer) clearTimeout(snapTimer);
      snapTimer = window.setTimeout(async () => {
        LOG.debug("[SpeedBall] 拖动停止，执行边缘吸附");
        await snapToEdge();
      }, SNAP_DEBOUNCE_MS);
    });
    LOG.info("[SpeedBall] 吸附监听已注册（防抖 300ms）");
  } catch (e) {
    LOG.warn(`[SpeedBall] 吸附监听失败: ${e}`);
  }
});

onUnmounted(() => {
  if (snapTimer) clearTimeout(snapTimer);
  if (dragUnlisten) {
    dragUnlisten();
    LOG.info("[SpeedBall] 吸附监听已注销");
  }
});
</script>

<template>
  <div class="ball-root" :class="{ offline: !agentOnline }" @mousedown="startDrag">
    <!-- 上格子：刷新按钮 -->
    <div class="cell refresh-cell" title="刷新数据" @click="onRefresh">
      <span class="refresh-icon" :class="{ spinning: refreshing }">🔄</span>
    </div>

    <!-- 下格子：红绿灯 -->
    <div class="cell light-cell" :title="lightState === 'green' ? '全部任务已完成' : '点击打开监控面板'" @click="openPanel">
      <div class="traffic-light" :class="lightState">
        <span class="ball-dot"></span>
        <span v-if="countText" class="ball-count">{{ countText }}</span>
        <span v-else class="ball-count done">✓</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ball-root {
  width: 60px;
  height: 120px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
  background: rgba(30, 30, 35, 0.75);
  border-radius: 18px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);
  cursor: grab;
  user-select: none;
  transition: background 0.2s;
}

.ball-root:active {
  cursor: grabbing;
}

.cell {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.refresh-cell {
  background: rgba(255, 255, 255, 0.05);
}

.refresh-cell:hover {
  background: rgba(255, 255, 255, 0.12);
}

.refresh-icon {
  font-size: 18px;
  line-height: 1;
}

.refresh-icon.spinning {
  animation: spin 0.8s linear infinite;
}

.light-cell {
  background: rgba(255, 255, 255, 0.05);
}

.light-cell:hover {
  background: rgba(255, 255, 255, 0.12);
}

.traffic-light {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
}

/* 红绿灯圆点：缩小为 9px（原 26px 的 1/3） */
.ball-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  box-shadow: 0 0 4px currentColor;
  transition: background 0.3s;
}

.ball-count {
  color: #ddd;
  font-size: 8px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  line-height: 1;
}

.ball-count.done {
  color: #67c23a;
  font-size: 9px;
}

/* 红绿灯颜色 */
.traffic-light.blue .ball-dot {
  background: #409eff;
  color: #409eff;
}

.traffic-light.yellow .ball-dot {
  background: #f0c040;
  color: #f0c040;
}

.traffic-light.green .ball-dot {
  background: #67c23a;
  color: #67c23a;
}

/* 离线状态整体变灰 */
.ball-root.offline .ball-dot {
  background: #666;
  color: #666;
  box-shadow: none;
}
</style>