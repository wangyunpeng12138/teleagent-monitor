<script setup lang="ts">
import type { TodoInfo } from "../composables/types";
import { LOG } from "../composables/logger";

const props = defineProps<{
  todo: TodoInfo;
}>();

const STATUS_META: Record<
  string,
  { icon: string; color: string; label: string }
> = {
  completed: { icon: "✅", color: "#67C23A", label: "已完成" },
  in_progress: { icon: "🔄", color: "#409EFF", label: "进行中" },
  pending: { icon: "⬜", color: "#888", label: "待处理" },
  cancelled: { icon: "❌", color: "#E6A23C", label: "已取消" },
};

function getMeta(status: string) {
  const meta = STATUS_META[status];
  if (!meta) {
    LOG.warn(`[TodoItem] 未知 todo 状态: ${status}`);
    return { icon: "❓", color: "#999", label: status };
  }
  return meta;
}
</script>

<template>
  <div class="todo-item" :style="{ color: getMeta(todo.status).color }">
    <span class="todo-icon" :class="{ 'fa-spin': todo.status === 'in_progress' }">
      {{ getMeta(todo.status).icon }}
    </span>
    <span class="todo-text">{{ todo.content || "(空任务)" }}</span>
    <span v-if="todo.priority === 'high'" class="priority-tag high">高</span>
    <span v-else-if="todo.priority === 'medium'" class="priority-tag medium">中</span>
  </div>
</template>

<style scoped>
.todo-item {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  font-size: 12px;
  line-height: 20px;
  padding: 1px 0;
}

.todo-icon {
  flex-shrink: 0;
  width: 16px;
  text-align: center;
}

.todo-text {
  flex: 1;
  word-break: break-all;
}

.priority-tag {
  flex-shrink: 0;
  font-size: 10px;
  padding: 0 4px;
  border-radius: 3px;
  line-height: 16px;
  margin-top: 2px;
}

.priority-tag.high {
  background: rgba(245, 108, 108, 0.2);
  color: #f56c6c;
}

.priority-tag.medium {
  background: rgba(230, 162, 60, 0.2);
  color: #e6a23c;
}
</style>
