import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import SpeedBall from "./components/SpeedBall.vue";
import "./assets/style.css";

/**
 * 根据窗口 label 挂载不同组件：
 * - main 窗口 → 面板（App.vue）
 * - ball 窗口 → 加速球（SpeedBall.vue）
 */
async function bootstrap() {
  let winLabel = "main";
  try {
    winLabel = getCurrentWindow().label;
  } catch (e) {
    console.warn("[main] 获取窗口 label 失败，默认 main", e);
  }

  const rootComponent = winLabel === "ball" ? SpeedBall : App;
  console.log(`[main] 窗口 label=${winLabel}，挂载组件=${winLabel === "ball" ? "SpeedBall" : "App"}`);
  createApp(rootComponent).mount("#app");
}

bootstrap();