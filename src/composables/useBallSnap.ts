/**
 * 加速球吸附逻辑 —— 窗口拖动靠近屏幕边缘时自动吸附
 *
 * 单位约定（实测确认）：
 * - win.outerPosition() / win.outerSize() 返回物理像素
 * - currentMonitor().size / .position / .workArea 也返回物理像素
 * - setPosition 必须使用 PhysicalPosition（若用 LogicalPosition 会被 scaleFactor 再次放大）
 * - 吸附以 workArea（排除任务栏）为基准，而非 monitor 全屏，避免窗口被任务栏遮挡
 * 因此全链路统一使用物理像素，不做任何缩放换算。
 */
import { getCurrentWindow, currentMonitor } from "@tauri-apps/api/window";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { LOG } from "./logger";

export function useBallSnap(margin = 12) {
  /** 检测窗口位置，若靠近屏幕边缘则吸附到边缘 */
  async function snapToEdge() {
    try {
      const win = getCurrentWindow();
      const pos = await win.outerPosition(); // 物理像素
      const size = await win.outerSize(); // 物理像素
      const mon = await currentMonitor();
      if (!mon) return;

      // 使用 workArea（已排除任务栏），统一物理像素
      // workArea 形如 { position: PhysicalPosition, size: PhysicalSize }
      const waPos = (mon.workArea?.position) || mon.position;
      const waSize = (mon.workArea?.size) || mon.size;
      const monLeft = waPos.x;
      const monTop = waPos.y;
      const monRight = monLeft + waSize.width;
      const monBottom = monTop + waSize.height;

      let x = pos.x;
      let y = pos.y;

      // 左右吸附
      if (x <= monLeft + margin) {
        x = monLeft;
      } else if (x + size.width >= monRight - margin) {
        x = monRight - size.width;
      }
      // 上下吸附
      if (y <= monTop + margin) {
        y = monTop;
      } else if (y + size.height >= monBottom - margin) {
        y = monBottom - size.height;
      }

      if (x !== pos.x || y !== pos.y) {
        // PhysicalPosition 与 outerPosition 单位一致
        await win.setPosition(new PhysicalPosition(x, y));
        LOG.debug(`[useBallSnap] 吸附至物理坐标 (${x}, ${y})`);
      }
    } catch (e) {
      LOG.warn(`[useBallSnap] 吸附检测失败: ${e}`);
    }
  }

  return { snapToEdge };
}