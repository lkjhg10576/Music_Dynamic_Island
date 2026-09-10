<template>
    <div class="system-toast-box" :class="{ 'is-two-line': hasBody }" @click="emit('select')">

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

        <!-- 速报持久停留：右侧 X 手动关闭（其余通知仍是自动消失，不需要按钮） -->
        <div v-if="showDismiss" class="toast-dismiss" title="关闭" @click.stop="emit('close')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { BRIEF_TOAST_TYPES, TWO_LINE_TOAST_TYPES, WEATHER_TOAST_TYPES, type SysToastType } from '../../composables/useNotifications';
import { resolveWeatherIconKey, weatherIconSvgOf } from '../../utils/weather';

const props = defineProps<{
    sysToastType: SysToastType;
    sysToastText: string;
    /** 标题行（速报为「问好 · 成语」，恶劣天气为事件标题，日程提醒为「日程提醒」） */
    sysToastTitle?: string;
    /** 正文行：非空即启用两行排版（标题在上、正文在下、图标在左，仿系统通知） */
    sysToastBody?: string;
    /** 图标键：sun/moon/cloud/rain/snow/sleet/fog/haze/alert/temp/wind */
    sysToastIcon?: string;
    /** 严重程度：info/warn/danger（恶劣天气预警配色） */
    sysToastSeverity?: string;
}>();

const emit = defineEmits<{
    (e: 'select'): void;
    /** 右侧 X：关闭持久停留的速报（由主组件 dismissSysToast 接管收尾） */
    (e: 'close'): void;
}>();

const isWeatherType = computed(() => WEATHER_TOAST_TYPES.has(props.sysToastType));

/** 持久停留的早/午/晚报才显示右侧 X（恶劣天气/天气服务不可用/日程提醒仍自动消失，无需按钮） */
const showDismiss = computed(() => BRIEF_TOAST_TYPES.has(props.sysToastType));

/** 两行排版：类型在名单内且正文非空（正文为空时回退单行，避免出现空行） */
const hasBody = computed(() => TWO_LINE_TOAST_TYPES.has(props.sysToastType) && !!props.sysToastBody);

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

/**
 * 天气图标：后端 icon 优先，通用 alert 时按事件标题/预警文本细化为具体情景图标
 * （图标表与解析口径见 utils/weather.ts，小图标与轻提示面板共用同一份）。
 */
const weatherIconSvg = computed(() => {
    const key = resolveWeatherIconKey(props.sysToastIcon, props.sysToastTitle);
    // 后端 icon 只按天气代码判定（0 → sun），不含时段语义；
    // 晚报档位若为晴天，换成月亮，避免夜间顶着太阳图标
    const finalKey = key === 'sun' && props.sysToastType === 'weather-evening' ? 'moon' : key;
    return weatherIconSvgOf(finalKey, props.sysToastType === 'weather' ? 'alert' : 'sun');
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

/* 两行排版（天气类 / 日程提醒）：左侧补内边距 + 图标回正（原 translateX(-8px) 会让 30px
   图标越过左侧圆角被裁切）+ 图标与文本拉开间距，观感对齐系统通知卡
   （IslandMsg：图标在左、标题在上、正文在下；字号按 42px 岛高收窄） */
.system-toast-box.is-two-line {
    padding-left: 6px;
    gap: 10px;
}

.system-toast-box.is-two-line .toast-icon {
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

/* 速报持久停留：带 X 时右侧留出按钮位，避免文字压到按钮下方 */
.system-toast-box:has(.toast-dismiss) {
    padding-right: 26px;
}

/* 右侧 X 关闭按钮（速报专用；仿实时活动面板关闭按钮的观感） */
.toast-dismiss {
    position: absolute;
    top: 50%;
    right: 4px;
    display: flex;
    width: 22px;
    height: 22px;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    color: #888;
    cursor: pointer;
    transform: translateY(-50%);
    transition: all 0.2s ease;
    z-index: 11;
}

.toast-dismiss:hover {
    color: #ff4757;
    background-color: rgba(255, 71, 87, 0.15);
}

.toast-dismiss svg {
    width: 14px;
    height: 14px;
}
</style>
