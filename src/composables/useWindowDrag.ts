/**
 * 窗口拖拽 Hook —— 利用 Tauri 的窗口拖拽 API
 */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LOG } from "./logger";

export function useWindowDrag() {
  let dragging = false;

  async function startDrag() {
    if (dragging) return;
    dragging = true;
    try {
      const win = getCurrentWindow();
      await win.startDragging();
      LOG.debug("[useWindowDrag] startDragging 已调用");
    } catch (e) {
      LOG.error(`[useWindowDrag] startDragging 失败: ${e}`);
    } finally {
      // startDragging 是异步阻塞的，松开鼠标后返回
      dragging = false;
    }
  }

  return { startDrag };
}
