import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { initSettings } from "./utils/settings";
import { initFrontendDebugLog } from "./utils/debugLog";

// 临时诊断日志（9.9.9-1）：先于应用挂载安装全局错误捕获（main / widget 窗口共用入口）
initFrontendDebugLog();

// 设置走 Rust 侧 config.json 单一数据源：先完成迁移与全量拉取，再挂载应用，
// 保证组件 setup 期间的同步读取拿到的是后端数据而非空缓存
initSettings().finally(() => {
    createApp(App).use(router).mount("#app");
});
