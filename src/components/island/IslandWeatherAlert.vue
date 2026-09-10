<template>
    <div class="system-toast-box" :class="{ 'is-weather': isWeatherType }" @click="emit('select')">

        <div v-if="sysToastType === 'app'" class="toast-icon app-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" opacity="0.3" />
                <path d="M8 12.5l3 3 5-6" stroke-width="2.5" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'lock'" class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <path d="M8 12V9a4 4 0 0 1 8 0v3" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'unlock'" class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <path d="M8 12V9a4 4 0 0 1 8 0" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'battery-charge'" class="toast-icon battery-charge-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
                <polygon points="11 7 8 12 12 12 11 17 14 12 10 12 11 7" stroke-width="1.5"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'battery-low'" class="toast-icon battery-low-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
                <line x1="6" y1="12" x2="9" y2="12" stroke-width="4" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <!-- 日程提醒 -->
        <div v-else-if="sysToastType === 'calendar'" class="toast-icon calendar-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                stroke-linejoin="round">
                <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="16" y1="2" x2="16" y2="6"></line>
                <line x1="8" y1="2" x2="8" y2="6"></line>
                <line x1="3" y1="10" x2="21" y2="10"></line>
            </svg>
        </div>

        <!-- 天气类：按后端 icon 字段渲染真实天气/预警图标（不再写死太阳） -->
        <div v-else-if="isWeatherType" class="toast-icon weather-icon" :style="{ color: weatherColor }">
            <span class="weather-svg" v-html="weatherIconSvg"></span>
        </div>

        <div v-else class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" opacity="0.3" />
                <g transform="translate(6, 5.5) scale(0.5)">
                    <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" stroke-width="4"
                        stroke-linecap="round" stroke-linejoin="round" />
                    <path d="M13.73 21a2 2 0 0 1-3.46 0" stroke-width="4" stroke-linecap="round"
                        stroke-linejoin="round" />
                </g>
            </svg>
        </div>

        <!-- 天气类为两行（标题 + 正文小字，仿系统通知）；其余类型保持单行 -->
        <div v-if="hasBody" class="toast-text-col">
            <div class="toast-title">{{ sysToastTitle || sysToastText }}</div>
            <div class="toast-body">{{ sysToastBody }}</div>
        </div>
        <div v-else class="toast-text">{{ sysToastText }}</div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { SysToastType } from '../../composables/useNotifications';

const props = defineProps<{
    sysToastType: SysToastType;
    sysToastText: string;
    /** 标题行（速报为「问好 · 成语」，恶劣天气为事件标题） */
    sysToastTitle?: string;
    /** 正文行：非空即启用两行排版 */
    sysToastBody?: string;
    /** 图标键：sun/cloud/rain/snow/sleet/fog/haze/alert/temp/moon */
    sysToastIcon?: string;
    /** 严重程度：info/warn/danger（恶劣天气预警配色） */
    sysToastSeverity?: string;
}>();

const emit = defineEmits<{
    (e: 'select'): void;
}>();

const isWeatherType = computed(() =>
    props.sysToastType === 'weather'
    || props.sysToastType === 'weather-morning'
    || props.sysToastType === 'weather-noon'
    || props.sysToastType === 'weather-evening');

const hasBody = computed(() => isWeatherType.value && !!props.sysToastBody);

// 恶劣天气（severe）按 severity 着色；早/午/晚报按档位保留各自的时段色
const weatherColor = computed(() => {
    if (props.sysToastType === 'weather') {
        switch (props.sysToastSeverity) {
            case 'danger': return '#ef4444';
            case 'warn': return '#f59e0b';
            case 'info': return '#0ea5e9';
            default: return '#0ea5e9';
        }
    }
    if (props.sysToastType === 'weather-morning') return '#f59e0b';
    if (props.sysToastType === 'weather-noon') return '#3b82f6';
    if (props.sysToastType === 'weather-evening') return '#8b5cf6';
    return '#0ea5e9';
});

// 图标键 → SVG（stroke 用 currentColor 继承 weatherColor）。缺省时回退：恶劣天气三角、速报太阳。
const ICONS: Record<string, string> = {
    sun: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>',
    moon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>',
    cloud: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path></svg>',
    rain: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="21" x2="7" y2="23"></line><line x1="12" y1="21" x2="11" y2="23"></line><line x1="16" y1="21" x2="15" y2="23"></line></svg>',
    snow: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="20" x2="8" y2="22"></line><line x1="12" y1="20" x2="12" y2="22"></line><line x1="16" y1="20" x2="16" y2="22"></line></svg>',
    sleet: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="21" x2="7" y2="23"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="16" y1="21" x2="15" y2="23"></line></svg>',
    fog: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="9" x2="20" y2="9"></line><line x1="4" y1="13" x2="20" y2="13"></line><line x1="4" y1="17" x2="20" y2="17"></line></svg>',
    haze: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="8" x2="20" y2="8"></line><line x1="6" y1="12" x2="18" y2="12"></line><line x1="4" y1="16" x2="20" y2="16"></line></svg>',
    temp: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z"></path></svg>',
    alert: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>',
};

const weatherIconSvg = computed(() => {
    const key = props.sysToastIcon
        || (props.sysToastType === 'weather' ? 'alert' : 'sun');
    return ICONS[key] || ICONS.alert;
});
</script>

<style scoped>
/* 系统操作通知样式 */
.system-toast-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    /* 右侧留白：避免文字伸入频谱/状态点区域（频谱在 left-capsule 右侧 flex 位） */
    padding-left: 0;
    padding-right: 4px;
    gap: 2px;
    z-index: 10;
    -webkit-app-region: no-drag;
    box-sizing: border-box;
    overflow: hidden;
}

/* 天气类左内边距 + 图标回正：原 translateX(-8px) 会让 30px 图标越过左侧圆角被裁切 */
.system-toast-box.is-weather {
    padding-left: 6px;
}

.system-toast-box.is-weather .toast-icon {
    transform: translateX(2px);
}

.toast-icon {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transform: translateX(-8px);
}

/* 灵动岛通知 */
.toast-icon.app-icon {
    color: currentColor;
}

/* 系统通知使用跟随字体的原生对比色 (黑白) */
.toast-icon.sys-icon {
    color: currentColor;
    opacity: 0.85;
}

.toast-icon svg {
    width: 22px;
    height: 22px;
    display: block;
}

.toast-icon.battery-charge-icon {
    color: #34C759;
}

.toast-icon.battery-low-icon {
    color: #FF3B30;
}

.toast-icon.calendar-icon {
    color: #06b6d4;
}

/* 天气图标：颜色由内联 weatherColor 决定，尺寸与普通图标一致 */
.toast-icon.weather-icon {
    color: #0ea5e9;
}

.weather-svg {
    display: inline-flex;
    align-items: center;
    justify-content: center;
}

.weather-svg :deep(svg) {
    width: 22px;
    height: 22px;
    display: block;
}

.toast-text {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0.95;
    transform: translateX(-2px) translateY(-1px);
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    /* 兜底：异常超长文本省略；正常电源/电池文案靠动态岛宽完整显示 */
    max-width: 100%;
    box-sizing: border-box;
}

/* 天气类两行：标题（问好+成语）在上，正文小字在下，仿系统通知 */
.toast-text-col {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    transform: translateX(-2px);
    box-sizing: border-box;
}

.toast-title {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 12.5px;
    font-weight: 700;
    line-height: 1.15;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.toast-body {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 10px;
    font-weight: 500;
    line-height: 1.15;
    opacity: 0.68;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>
