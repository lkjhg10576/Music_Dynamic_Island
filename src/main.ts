import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";

// 灵动岛 WebView 启动即关系统装饰 + 透明底：
// 否则路由懒加载完成前可能闪出 “MDI Widget” 标题栏空窗。
const bootstrapWidgetFrame = async () => {
  const path = `${location.pathname || ""}${location.hash || ""}`;
  if (!path.includes("widget")) return;

  document.documentElement.style.background = "transparent";
  document.documentElement.style.backgroundColor = "transparent";
  if (document.body) {
    document.body.style.background = "transparent";
    document.body.style.backgroundColor = "transparent";
  }

  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    await Promise.allSettled([
      win.setDecorations(false),
      win.setShadow(false),
      win.setAlwaysOnTop(true),
    ]);
  } catch {
    // 浏览器预览或 capability 未就绪时忽略
  }
};

void bootstrapWidgetFrame();

createApp(App).use(router).mount("#app");
