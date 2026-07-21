<template>
    <div class="dynamic-set-dashboard">
        <div class="grid-section">
            <div class="neo-card">
                <div class="card-header">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                        stroke-linejoin="round" class="title-icon">
                        <path
                            d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                    </svg>
                    <span>物理弹簧</span>
                </div>
                <div class="spring-selector">
                    <button class="spring-btn" :class="{ active: springStyle === 'stiff' }"
                        @click="springStyle = 'stiff'">
                        <svg viewBox="0 0 24 24" class="spring-icon">
                            <path d="M4 12 L8 8 L12 16 L16 8 L20 12" stroke="currentColor" stroke-width="2" fill="none"
                                stroke-linejoin="round" />
                        </svg>
                        <span>快速</span>
                    </button>
                    <button class="spring-btn" :class="{ active: springStyle === 'bouncy' }"
                        @click="springStyle = 'bouncy'">
                        <svg viewBox="0 0 24 24" class="spring-icon">
                            <path d="M3 12 C 7 2, 10 22, 14 12 C 16 7, 18 16, 21 12" stroke="currentColor"
                                stroke-width="2" fill="none" stroke-linecap="round" />
                        </svg>
                        <span>果味</span>
                    </button>
                </div>
            </div>

            <div class="neo-card">
                <div class="card-header">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                        stroke-linejoin="round" class="title-icon">
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                        <circle cx="8.5" cy="8.5" r="1.5" />
                        <polyline points="21 15 16 10 5 21" />
                    </svg>
                    <span>边缘形态</span>
                </div>
                <div class="form-group-list">
                    <div class="form-item">
                        <span class="label">边缘圆角</span>
                        <div class="shape-toggle">
                            <button :class="{ active: borderRadius === 100 }" @click="borderRadius = 100"
                                title="经典胶囊"></button>
                            <button :class="{ active: borderRadius === 12 }" @click="borderRadius = 12"
                                title="圆角矩形" style="border-radius: 6px;"></button>
                        </div>
                    </div>
                </div>
            </div>

            <div class="neo-card">
                <div class="card-header">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                        stroke-linejoin="round" class="title-icon">
                        <polygon points="12 2 2 7 12 12 22 7 12 2" />
                        <polyline points="2 17 12 22 22 17" />
                        <polyline points="2 12 17 22 12" />
                    </svg>
                    <span>窗口层级</span>
                </div>
                <div class="form-group-list">
                    <div class="form-item">
                        <span class="label">始终置顶</span>
                        <label class="mock-switch">
                            <input type="checkbox" v-model="isAlwaysOnTop">
                            <span class="slider"></span>
                        </label>
                    </div>
                </div>
            </div>
        </div>

        <div class="list-section">
            <div class="card-header" style="margin-bottom: 12px;">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" class="title-icon">
                    <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
                </svg>
                <span style="font-size: 16px; letter-spacing: 0.5px;">尺寸边界</span>
            </div>

            <div class="slider-list-container">
                <div class="slider-row">
                    <div class="row-info">
                        <span class="row-title">常规宽度</span>
                        <span class="row-desc">速度态灵动岛宽度</span>
                    </div>
                    <div class="row-action">
                        <input type="range" min="140" max="300" v-model.number="baseWidth"
                            class="track-slider highlight-slider" />
                        <div class="value-box">{{ baseWidth }}<span class="unit">PX</span></div>
                        <button class="reset-btn" @click="baseWidth = 150" title="恢复默认">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                <path d="M3 3v5h5" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="slider-row">
                    <div class="row-info">
                        <span class="row-title">媒体常态宽度</span>
                        <span class="row-desc">音乐控制器收起态宽度</span>
                    </div>
                    <div class="row-action">
                        <input type="range" min="200" max="400" v-model.number="musicBaseWidth"
                            class="track-slider highlight-slider" />
                        <div class="value-box">{{ musicBaseWidth }}<span class="unit">PX</span></div>
                        <button class="reset-btn" @click="musicBaseWidth = 260" title="恢复默认">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                <path d="M3 3v5h5" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="slider-row">
                    <div class="row-info">
                        <span class="row-title">媒体展开宽度</span>
                        <span class="row-desc">音乐详情卡片宽度</span>
                    </div>
                    <div class="row-action">
                        <input type="range" min="260" max="480" v-model.number="musicExpandedWidth"
                            class="track-slider highlight-slider" />
                        <div class="value-box">{{ musicExpandedWidth }}<span class="unit">PX</span></div>
                        <button class="reset-btn" @click="musicExpandedWidth = 320" title="恢复默认">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                <path d="M3 3v5h5" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="slider-row">
                    <div class="row-info">
                        <span class="row-title">消息展开宽度</span>
                        <span class="row-desc">消息通知卡片宽度</span>
                    </div>
                    <div class="row-action">
                        <input type="range" min="300" max="600" v-model.number="msgExpandedWidth"
                            class="track-slider highlight-slider" />
                        <div class="value-box">{{ msgExpandedWidth }}<span class="unit">PX</span></div>
                        <button class="reset-btn" @click="msgExpandedWidth = 360" title="恢复默认">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                <path d="M3 3v5h5" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="slider-row">
                    <div class="row-info">
                        <span class="row-title">全局缩放</span>
                        <span class="row-desc">灵动岛整体缩放比例</span>
                    </div>
                    <div class="row-action">
                        <input type="range" min="1" max="1.75" step="0.25" v-model.number="appScale"
                            class="track-slider highlight-slider" />
                        <div class="value-box">{{ Math.round(appScale * 100) }}<span class="unit">%</span></div>
                        <button class="reset-btn" @click="appScale = 1.0" title="恢复默认">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                <path d="M3 3v5h5" />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { emit } from '@tauri-apps/api/event';
import {
    NSD_SPRING_STYLE,
    NSD_BORDER_RADIUS,
    NSD_ALWAYS_ON_TOP,
    NSD_BASE_WIDTH,
    NSD_MUSIC_BASE_WIDTH,
    NSD_MUSIC_EXPANDED_WIDTH,
    NSD_MSG_EXPANDED_WIDTH,
    NSD_APP_SCALE,
} from '../constants/storageKeys';

const baseWidth = ref(Number(localStorage.getItem(NSD_BASE_WIDTH)) || 150);
const musicBaseWidth = ref(Number(localStorage.getItem(NSD_MUSIC_BASE_WIDTH)) || 260);
const musicExpandedWidth = ref(Number(localStorage.getItem(NSD_MUSIC_EXPANDED_WIDTH)) || 320);
const msgExpandedWidth = ref(Number(localStorage.getItem(NSD_MSG_EXPANDED_WIDTH)) || 360);
const appScale = ref(Number(localStorage.getItem(NSD_APP_SCALE)) || 1.0);
const borderRadius = ref(Number(localStorage.getItem(NSD_BORDER_RADIUS)) || 100);
const springStyle = ref<'stiff' | 'bouncy'>(
    (localStorage.getItem(NSD_SPRING_STYLE) as 'stiff' | 'bouncy') || 'bouncy'
);
const isAlwaysOnTop = ref(localStorage.getItem(NSD_ALWAYS_ON_TOP) !== 'false');

watch(
    [baseWidth, musicBaseWidth, musicExpandedWidth, msgExpandedWidth, borderRadius, springStyle, isAlwaysOnTop, appScale],
    async () => {
        localStorage.setItem(NSD_BASE_WIDTH, String(baseWidth.value));
        localStorage.setItem(NSD_MUSIC_BASE_WIDTH, String(musicBaseWidth.value));
        localStorage.setItem(NSD_MUSIC_EXPANDED_WIDTH, String(musicExpandedWidth.value));
        localStorage.setItem(NSD_MSG_EXPANDED_WIDTH, String(msgExpandedWidth.value));
        localStorage.setItem(NSD_BORDER_RADIUS, String(borderRadius.value));
        localStorage.setItem(NSD_SPRING_STYLE, springStyle.value);
        localStorage.setItem(NSD_ALWAYS_ON_TOP, String(isAlwaysOnTop.value));
        localStorage.setItem(NSD_APP_SCALE, String(appScale.value));

        await emit('sync-dynamic-settings', {
            baseWidth: baseWidth.value,
            musicBaseWidth: musicBaseWidth.value,
            musicExpandedWidth: musicExpandedWidth.value,
            msgExpandedWidth: msgExpandedWidth.value,
            borderRadius: borderRadius.value,
            springStyle: springStyle.value,
            isAlwaysOnTop: isAlwaysOnTop.value,
            appScale: appScale.value,
        });
    }
);
</script>

<style scoped>
.dynamic-set-dashboard {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    max-height: calc(100vh - 120px);
    overflow: hidden;
    box-sizing: border-box;
    user-select: none;
}

.grid-section {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    flex-shrink: 0;
}

.neo-card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    transition: transform 0.2s, border-color 0.2s;
}

.card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 600;
    color: var(--item-title-color);
    margin-bottom: 13px;
}

.title-icon {
    width: 16px;
    height: 16px;
    color: var(--item-desc-color);
}

.spring-selector {
    display: flex;
    gap: 10px;
    height: 100%;
}

.spring-btn {
    flex: 1;
    background: transparent;
    border: 1px solid var(--control-border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--btn-sec-color);
    cursor: pointer;
    transition: all 0.2s;
}

.spring-icon {
    width: 24px;
    height: 24px;
}

.spring-btn:hover {
    background: var(--btn-pri-bg);
    color: var(--btn-sec-hover-color);
}

.spring-btn.active {
    background: var(--btn-pri-bg);
    border-color: var(--btn-pri-border);
    color: var(--btn-pri-color);
    box-shadow: 0 2px 8px var(--card-shadow-hover);
}

.form-group-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    justify-content: center;
}

.form-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.label {
    font-size: 13px;
    color: var(--item-title-color);
}

.shape-toggle {
    display: flex;
    gap: 6px;
}

.shape-toggle button {
    width: 32px;
    height: 20px;
    background: var(--btn-pri-bg);
    border: 2px solid transparent;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s;
}

.shape-toggle button.active {
    border-color: var(--item-title-active-color);
    background: var(--btn-pri-bg);
}

.mock-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
}

.mock-switch input {
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
    background-color: var(--slider-bg);
    transition: .3s;
    border-radius: 24px;
    border: 1px solid var(--control-border);
}

.slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 2px;
    bottom: 2px;
    background-color: #ffffff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: .3s;
    border-radius: 50%;
}

input:checked+.slider {
    background-color: var(--slider-checked-bg);
    border-color: var(--slider-checked-bg);
}

input:checked+.slider:before {
    transform: translateX(20px);
}

.list-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--control-bg, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--card-border, rgba(255, 255, 255, 0.08));
    border-radius: 16px;
    padding: 16px;
    overflow-y: auto;
}

.list-section::-webkit-scrollbar {
    width: 4px;
}

.list-section::-webkit-scrollbar-track {
    background: transparent;
}

.list-section::-webkit-scrollbar-thumb {
    background-color: var(--slider-bg);
    border-radius: 4px;
}

.list-section::-webkit-scrollbar-thumb:hover {
    background-color: var(--slider-checked-bg);
}

.slider-list-container {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
    align-content: flex-start;
}

.slider-row {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 8px 12px;
    background: var(--bg-body, rgba(255, 255, 255, 0.02));
    border-radius: 12px;
}

.row-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.row-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--item-title-color);
}

.row-desc {
    font-size: 11px;
    color: #888;
}

.row-action {
    display: flex;
    align-items: center;
    width: 100%;
    gap: 10px;
}

.track-slider {
    -webkit-appearance: none;
    appearance: none;
    flex: 1;
    width: auto;
    height: 6px;
    background: var(--slider-bg, rgba(255, 255, 255, 0.1));
    border-radius: 3px;
    outline: none;
}

.track-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.3);
    cursor: pointer;
    transition: transform 0.1s;
}

.track-slider::-webkit-slider-thumb:hover {
    transform: scale(1.15);
}

.highlight-slider::-webkit-slider-thumb {
    border: 3px solid #666;
}

.value-box {
    width: 54px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-body, rgba(0, 0, 0, 0.3));
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--item-title-color);
    font-family: ui-monospace, monospace;
}

.value-box .unit {
    font-size: 10px;
    padding: 1px 3px;
    border-radius: 4px;
    background: var(--select-bg, rgba(255, 255, 255, 0.03));
    color: #666;
    margin-left: 2px;
    transform: translateX(3px);
}

.reset-btn {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-body, rgba(0, 0, 0, 0.3));
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: var(--item-title-color);
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
}

.reset-btn:hover {
    background: var(--btn-pri-bg);
    color: var(--btn-pri-color);
}

.reset-btn svg {
    width: 12px;
    height: 12px;
}
</style>
