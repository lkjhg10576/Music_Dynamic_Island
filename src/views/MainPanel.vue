<template>
    <div class="panel-container">
        <header class="panel-header">
            <div class="brand">
                <div class="logo-icon">⚡</div>
                <div>
                    <h1>NetSpeed Dynamic</h1>
                    <p class="subtitle">NSD 桌面动态组件 v1.0.0</p>
                </div>
            </div>
            <span class="status-badge" :class="{ 'is-active': isWidgetVisible }">
                {{ isWidgetVisible ? '已开启' : '已关闭' }}
            </span>
        </header>

        <hr class="divider" />

        <div class="main-content">
            <div class="card status-card">
                <h3>当前实时状态</h3>
                <div class="speed-monitor">
                    <div class="speed-item">
                        <span class="arrow up">↑</span>
                        <div class="speed-info">
                            <span class="label">上传速度</span>
                            <span class="value">124.5 KB/s</span>
                        </div>
                    </div>
                    <div class="speed-item">
                        <span class="arrow down">↓</span>
                        <div class="speed-info">
                            <span class="label">下载速度</span>
                            <span class="value">2.4 MB/s</span>
                        </div>
                    </div>
                </div>
                <div class="mini-chart">
                    <div class="bar" style="height: 40%"></div>
                    <div class="bar" style="height: 60%"></div>
                    <div class="bar" style="height: 45%"></div>
                    <div class="bar" style="height: 75%"></div>
                    <div class="bar" style="height: 90%"></div>
                    <div class="bar" style="height: 55%"></div>
                </div>
            </div>

            <div class="card settings-card">
                <h3>常规设置</h3>

                <div class="setting-item">
                    <div class="item-meta">
                        <span class="item-title">显示灵动岛悬浮窗</span>
                        <span class="item-desc">在桌面上方显示网速交互岛</span>
                    </div>
                    <label class="switch">
                        <input type="checkbox" :checked="isWidgetVisible" @change="toggleWidget">
                        <span class="slider"></span>
                    </label>
                </div>

                <div class="setting-item">
                    <div class="item-meta">
                        <span class="item-title">开机自动启动</span>
                        <span class="item-desc">跟随系统启动 NetSpeed</span>
                    </div>
                    <label class="switch">
                        <input type="checkbox" v-model="autoStart">
                        <span class="slider"></span>
                    </label>
                </div>

                <div class="setting-item slider-item">
                    <div class="item-meta">
                        <span class="item-title">悬浮窗不透明度</span>
                        <span class="item-desc">调节灵动岛的外观透明度</span>
                    </div>
                    <input type="range" min="20" max="100" v-model="opacity" class="range-input" />
                </div>
            </div>
        </div>

        <footer class="panel-footer">
            <span>&copy; 2026 Ryen. All rights reserved.</span>
            <span class="action-link">检查更新</span>
        </footer>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

const isWidgetVisible = ref(false);
const autoStart = ref(true);
const opacity = ref(90);

onMounted(async () => {
    for (let i = 0; i < 6; i++) {
        try {
            const visible = await invoke<boolean>('is_widget_visible');
            if (visible) {
                isWidgetVisible.value = true;
                return;
            }
        } catch { /* 窗口还没注册好，忽略 */ }
        await new Promise(r => setTimeout(r, 200));
    }
    // 6 次都没拿到 true，说明确实没开
    isWidgetVisible.value = false;
});

const toggleWidget = async () => {
    const widget = await WebviewWindow.getByLabel('widget');
    if (!widget) return;

    if (isWidgetVisible.value) {
        await widget.hide();
        isWidgetVisible.value = false;
    } else {
        await widget.show();
        await widget.setAlwaysOnTop(true);
        isWidgetVisible.value = true;
    }
};
</script>

<style scoped>
/* 全局样式基础重置（纯白极简） */
:global(body) {
    background-color: #ffffff;
    color: #1e293b;
    /* 明确的深色字 */
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    margin: 0;
    padding: 0;
    user-select: none;
}

/* 布局主容器 */
.panel-container {
    padding: 24px;
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    min-height: calc(100vh - 48px);
}

/* 头部样式 */
.panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
}

.brand {
    display: flex;
    align-items: center;
    gap: 12px;
}

.logo-icon {
    font-size: 24px;
    background: linear-gradient(135deg, #007aff, #00f0ff);
    width: 42px;
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.15);
}

.brand h1 {
    font-size: 18px;
    margin: 0;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: #0f172a;
    /* 纯黑标题 */
}

.subtitle {
    font-size: 12px;
    color: #64748b;
    /* 灰黑辅助字，保证清晰度 */
    margin: 2px 0 0 0;
}

.status-badge {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 20px;
    background: #fef2f2;
    color: #ef4444;
    border: 1px solid #fee2e2;
    transition: all 0.3s;
    font-weight: 500;
}

.status-badge.is-active {
    background: #ecfdf5;
    color: #10b981;
    border: 1px solid #d1fae5;
}

.divider {
    border: none;
    border-top: 1px solid #e2e8f0;
    /* 浅灰分割线 */
    margin-bottom: 20px;
}

/* 主主体分栏 */
.main-content {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    gap: 20px;
    flex-grow: 1;
}

/* 白系卡片通用样式 */
.card {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    /* 浅灰色边框替代深色背景 */
    border-radius: 16px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05), 0 2px 4px -1px rgba(0, 0, 0, 0.03);
}

.card h3 {
    font-size: 14px;
    color: #475569;
    margin: 0 0 16px 0;
    font-weight: 600;
}

/* 左侧：状态监控面板 */
.status-card {
    background: #ffffff;
}

.speed-monitor {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-bottom: 20px;
}

.speed-item {
    display: flex;
    align-items: center;
    gap: 12px;
}

.arrow {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 14px;
}

.arrow.up {
    background: #eff6ff;
    color: #3b82f6;
}

.arrow.down {
    background: #ecfdf5;
    color: #10b981;
}

.speed-info {
    display: flex;
    flex-direction: column;
}

.speed-info .label {
    font-size: 11px;
    color: #64748b;
}

.speed-info .value {
    font-size: 16px;
    font-weight: 600;
    font-family: monospace;
    color: #0f172a;
}

/* 波动图表占位 */
.mini-chart {
    display: flex;
    align-items: flex-end;
    gap: 6px;
    height: 45px;
    margin-top: auto;
    padding-top: 10px;
    border-top: 1px dashed #e2e8f0;
}

.mini-chart .bar {
    flex: 1;
    background: linear-gradient(to top, #3b82f6, #60a5fa);
    border-radius: 3px 3px 0 0;
    opacity: 0.8;
}

/* 右侧：设置列表 */
.setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0;
    border-bottom: 1px solid #f1f5f9;
}

.setting-item:last-child {
    border-bottom: none;
}

.slider-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
}

.item-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.item-title {
    font-size: 14px;
    font-weight: 500;
    color: #0f172a;
}

.item-desc {
    font-size: 12px;
    color: #64748b;
}

/* 白系 Switch 切换开关样式 */
.switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
}

.switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #cbd5e1;
    /* 未选中时为淡灰 */
    transition: .3s;
    border-radius: 24px;
}

.slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: .3s;
    border-radius: 50%;
}

input:checked+.slider {
    background-color: #007aff;
}

input:checked+.slider:before {
    transform: translateX(20px);
}

/* 白系滑块 Range 样式 */
.range-input {
    width: 100%;
    -webkit-appearance: none;
    appearance: none;
    background: #e2e8f0;
    height: 6px;
    border-radius: 3px;
    outline: none;
}

.range-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #007aff;
    cursor: pointer;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
    transition: 0.1s;
}

.range-input::-webkit-slider-thumb:hover {
    transform: scale(1.2);
}

/* 页脚 */
.panel-footer {
    margin-top: 24px;
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #94a3b8;
}

.action-link {
    color: #007aff;
    cursor: pointer;
}

.action-link:hover {
    text-decoration: underline;
}
</style>