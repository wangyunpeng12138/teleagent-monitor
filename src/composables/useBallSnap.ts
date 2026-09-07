/**
 * 加速球吸附逻辑 —— 窗口拖动靠近屏幕边缘时自动吸附
 */
import { getCurrentWindow, currentMonitor } from "@tauri-apps/api/window";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { LOG } from "./logger";

export function useBallSnap(margin = 28) {
  /** 检测窗口位置，若靠近屏幕边缘则吸附到边缘 */
  async function snapToEdge() {
    try {
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      const size = await win.outerSize();
      const mon = await currentMonitor();
      if (!mon) return;

      const mp = mon.position;
      const ms = mon.size;
      let x = pos.x;
      let y = pos.y;

      // 左右吸附
      if (x <= mp.x + margin) {
        x = mp.x;
      } else if (x + size.width >= mp.x + ms.width - margin) {
        x = mp.x + ms.width - size.width;
      }
      // 上下吸附
      if (y <= mp.y + margin) {
        y = mp.y;
      } else if (y + size.height >= mp.y + ms.height - margin) {
        y = mp.y + ms.height - size.height;
      }

      if (x !== pos.x || y !== pos.y) {
        await win.setPosition(new PhysicalPosition(x, y));
        LOG.debug(`[useBallSnap] 吸附至 (${x}, ${y})`);
      }
    } catch (e) {
      LOG.warn(`[useBallSnap] 吸附检测失败: ${e}`);
    }
  }

  return { snapToEdge };
}