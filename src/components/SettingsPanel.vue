<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { LOG } from "../composables/logger";
import type { AppConfig } from "../composables/useMonitor";
import { DEFAULT_CONFIG } from "../composables/useMonitor";

const emit = defineEmits<{
  (e: "close"): void;
  (e: "saved"): void;
}>();

const config = ref<AppConfig>({ ...DEFAULT_CONFIG });
const saving = ref(false);
const saveMsg = ref("");
const saveError = ref(false);
const autostartEnabled = ref(false);
const autostartToggling = ref(false);

onMounted(async () => {
  try {
    const cfg = await invoke<AppConfig>("get_config");
    config.value = { ...cfg };
    LOG.info(`[Settings] 配置加载: ${JSON.stringify(cfg)}`);
  } catch (e) {
    LOG.error(`[Settings] 加载配置失败: ${e}`);
  }

  // 加载开机自启动状态
  try {
    autostartEnabled.value = await invoke<boolean>("is_autostart_enabled");
    LOG.info(`[Settings] 开机自启动状态: ${autostartEnabled.value}`);
  } catch (e) {
    LOG.error(`[Settings] 查询开机自启动状态失败: ${e}`);
  }
});

async function toggleAutostart() {
  autostartToggling.value = true;
  try {
    if (autostartEnabled.value) {
      await invoke("enable_autostart");
      LOG.info("[Settings] 开机自启动已开启");
    } else {
      await invoke("disable_autostart");
      LOG.info("[Settings] 开机自启动已关闭");
    }
  } catch (e) {
    LOG.error(`[Settings] 切换开机自启动失败: ${e}`);
    autostartEnabled.value = !autostartEnabled.value; // 回滚
  } finally {
    autostartToggling.value = false;
  }
}

async function handleSave() {
  saving.value = true;
  saveMsg.value = "";
  saveError.value = false;

  try {
    await invoke("save_config", {
      teleagentDataDir: config.value.teleagent_data_dir,
      sessionPollInterval: config.value.session_poll_interval,
      jobPollInterval: config.value.job_poll_interval,
      maxSessions: config.value.max_sessions,
      staleThresholdMinutes: config.value.stale_threshold_minutes,
    });
    saveMsg.value = "保存成功！配置已生效。";
    saveError.value = false;
    LOG.info("[Settings] 配置保存成功");
    setTimeout(() => {
      emit("saved");
      emit("close");
    }, 800);
  } catch (e) {
    saveMsg.value = `保存失败: ${e}`;
    saveError.value = true;
    LOG.error(`[Settings] 配置保存失败: ${e}`);
  } finally {
    saving.value = false;
  }
}

function handleReset() {
  config.value = { ...DEFAULT_CONFIG };
  LOG.info("[Settings] 已重置为默认值");
}

function handleCancel() {
  emit("close");
}
</script>

<template>
  <div class="settings-overlay" @click.self="handleCancel">
    <div class="settings-panel">
      <div class="settings-header">
        <span>设置</span>
        <button class="btn-close" @click="handleCancel">✕</button>
      </div>

      <div class="settings-body">
        <!-- 开机自启动 -->
        <div class="form-group">
          <label class="form-label">开机自启动</label>
          <div class="toggle-row">
            <label class="toggle-switch">
              <input
                type="checkbox"
                v-model="autostartEnabled"
                :disabled="autostartToggling"
                @change="toggleAutostart"
              />
              <span class="toggle-slider"></span>
            </label>
            <span class="toggle-text">{{ autostartEnabled ? "已启用" : "未启用" }}</span>
          </div>
          <span class="form-hint">开机时自动启动 TeleAgent Monitor</span>
        </div>

        <!-- TeleAgent 数据目录 -->
        <div class="form-group">
          <label class="form-label">TeleAgent 数据目录</label>
          <input
            v-model="config.teleagent_data_dir"
            class="form-input"
            type="text"
            placeholder="留空使用默认路径 (~/.local/share/TeleAgent)"
          />
          <span class="form-hint">TeleAgent 运行数据所在目录，包含 teleagent.db 等文件</span>
        </div>

        <!-- 会话轮询间隔 -->
        <div class="form-group">
          <label class="form-label">会话刷新间隔</label>
          <div class="input-with-unit">
            <input
              v-model.number="config.session_poll_interval"
              class="form-input"
              type="number"
              min="1000"
              step="500"
            />
            <span class="unit">毫秒</span>
          </div>
          <span class="form-hint">每隔多久刷新一次会话列表（建议 2000-5000）</span>
        </div>

        <!-- 定时任务轮询间隔 -->
        <div class="form-group">
          <label class="form-label">定时任务刷新间隔</label>
          <div class="input-with-unit">
            <input
              v-model.number="config.job_poll_interval"
              class="form-input"
              type="number"
              min="5000"
              step="1000"
            />
            <span class="unit">毫秒</span>
          </div>
          <span class="form-hint">每隔多久刷新一次定时任务列表（建议 10000-30000）</span>
        </div>

        <!-- 最大会话数 -->
        <div class="form-group">
          <label class="form-label">最大会话显示数</label>
          <div class="input-with-unit">
            <input
              v-model.number="config.max_sessions"
              class="form-input"
              type="number"
              min="5"
              max="50"
            />
            <span class="unit">条</span>
          </div>
          <span class="form-hint">面板中最多显示的会话数量</span>
        </div>

        <!-- 超时阈值 -->
        <div class="form-group">
          <label class="form-label">会话超时阈值</label>
          <div class="input-with-unit">
            <input
              v-model.number="config.stale_threshold_minutes"
              class="form-input"
              type="number"
              min="1"
              step="1"
            />
            <span class="unit">分钟</span>
          </div>
          <span class="form-hint">超过此时间无新消息的运行中会话将被判定为结束</span>
        </div>

        <!-- 保存反馈 -->
        <div v-if="saveMsg" class="save-feedback" :class="{ error: saveError }">
          {{ saveMsg }}
        </div>
      </div>

      <div class="settings-footer">
        <button class="btn btn-secondary" @click="handleReset">恢复默认</button>
        <button class="btn btn-secondary" @click="handleCancel">取消</button>
        <button class="btn btn-primary" :disabled="saving" @click="handleSave">
          {{ saving ? "保存中..." : "保存" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.settings-panel {
  width: 340px;
  background: rgba(35, 35, 40, 0.98);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  flex-direction: column;
  max-height: 90%;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 700;
  color: #e0e0e0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.btn-close {
  background: none;
  border: none;
  color: #888;
  cursor: pointer;
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 3px;
}

.btn-close:hover {
  background: rgba(245, 108, 108, 0.2);
  color: #f56c6c;
}

.settings-body {
  padding: 10px 12px;
  overflow-y: auto;
}

.form-group {
  margin-bottom: 10px;
}

.form-label {
  display: block;
  color: #b0b0b0;
  font-size: 11px;
  font-weight: 600;
  margin-bottom: 3px;
}

.form-input {
  width: 100%;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  padding: 4px 8px;
  color: #e0e0e0;
  font-size: 12px;
  outline: none;
  box-sizing: border-box;
}

.form-input:focus {
  border-color: rgba(64, 158, 255, 0.5);
  background: rgba(64, 158, 255, 0.05);
}

.input-with-unit {
  display: flex;
  align-items: center;
  gap: 4px;
}

.input-with-unit .form-input {
  flex: 1;
}

.unit {
  color: #888;
  font-size: 11px;
  flex-shrink: 0;
}

.form-hint {
  display: block;
  color: #666;
  font-size: 9px;
  margin-top: 2px;
}

.save-feedback {
  font-size: 11px;
  padding: 4px 8px;
  border-radius: 4px;
  margin-top: 4px;
}

.save-feedback:not(.error) {
  color: #67c23a;
  background: rgba(103, 194, 58, 0.1);
}

.save-feedback.error {
  color: #f56c6c;
  background: rgba(245, 108, 108, 0.1);
}

.settings-footer {
  display: flex;
  gap: 6px;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  justify-content: flex-end;
}

.btn {
  border: none;
  border-radius: 4px;
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary {
  background: rgba(64, 158, 255, 0.8);
  color: #fff;
}

.btn-primary:hover:not(:disabled) {
  background: rgba(64, 158, 255, 1);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.08);
  color: #aaa;
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

/* === 开关组件 === */
.toggle-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 18px;
  flex-shrink: 0;
}

.toggle-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.12);
  border-radius: 9px;
  transition: 0.3s;
}

.toggle-slider:before {
  content: "";
  position: absolute;
  height: 14px;
  width: 14px;
  left: 2px;
  bottom: 2px;
  background: #ccc;
  border-radius: 50%;
  transition: 0.3s;
}

.toggle-switch input:checked + .toggle-slider {
  background: rgba(64, 158, 255, 0.8);
}

.toggle-switch input:checked + .toggle-slider:before {
  transform: translateX(18px);
  background: #fff;
}

.toggle-switch input:disabled + .toggle-slider {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-text {
  color: #aaa;
  font-size: 11px;
}
</style>
