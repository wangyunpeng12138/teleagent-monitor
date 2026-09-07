/**
 * 窗口拖拽 Hook —— 利用 Tauri 的窗口拖拽 API
 *
 * 仅在鼠标按在非交互元素（按钮等）上时才启动拖拽，避免拦截按钮点击。
 */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LOG } from "./logger";

export function useWindowDrag() {
  let dragging = false;

  async function startDrag(event: MouseEvent) {
    // 如果点击目标是 BUTTON 或其子元素，不启动拖拽（让 click 事件正常触发）
    const target = event.target as HTMLElement;
    if (target && (target.tagName === "BUTTON" || target.closest("button"))) {
      return;
    }

    if (dragging) return;
    dragging = true;
    try {
      const win = getCurrentWindow();
      await win.startDragging();
      LOG.debug("[useWindowDrag] startDragging 已调用");
    } catch (e) {
      LOG.error(`[useWindowDrag] startDragging 失败: ${e}`);
    } finally {
      dragging = false;
    }
  }

  return { startDrag };
}
