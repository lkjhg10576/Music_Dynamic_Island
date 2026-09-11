<template>
    <div class="light-alert-panel">
        <!-- 图标按"具体情况"取（暴雨→雨、大雾→雾、高温→温度计…），颜色按预警等级 -->
        <div class="light-alert-icon" :style="{ color: levelColor, background: levelBg }">
            <span class="light-alert-icon-svg" v-html="iconSvg"></span>
        </div>

        <div class="light-alert-text-col">
            <div class="light-alert-head">
                <span class="light-alert-level" :style="{ color: levelColor }">{{ alert?.levelText || '预警' }}</span>
                <span v-if="alert?.type" class="light-alert-type">{{ alert.type }}</span>
            </div>
            <div class="light-alert-title" :title="alert?.title || ''">{{ alert?.title || '' }}</div>
        </div>

        <button class="light-alert-close" type="button" title="关闭" @click.stop="emit('close')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
        </button>
    </div>
</template>

<script setup lang="ts">
// 恶劣天气轻提示的岛上展开面板。
// 原版按控制台卡片体例写（padding 16px / min-width 200px / 底部"5s 后自动隐藏 + 关闭"），
// 放进 42px 高的灵动岛会被直接裁掉；此处改为岛上体例：单行两段文字 + 右侧 X，
// 与 IslandTaskbarProgress / IslandCalendarPanel 等面板保持同一套内边距与关闭按钮观感。
import { computed } from 'vue';
import { weatherAlertIconKey, weatherIconSvgOf } from '../../utils/weather';

const props = defineProps<{
    alert: {
        alertId: string;
        level: string;
        levelText: string;
        type: string;
        title: string;
    } | null;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

// 等级配色（与 IslandWeatherChip 同一套口径：B 蓝 / Y 黄 / O 橙 / R 红 / W 白）
const levelColor = computed(() => {
    switch ((props.alert?.level || '').toUpperCase()) {
        case 'Y': case '黄色': return '#eab308';
        case 'O': case '橙色': return '#f97316';
        case 'R': case '红色': return '#ef4444';
        case 'W': case '白色': return '#e5e7eb';
        default: return '#3b82f6';
    }
});

/** 图标底衬：等级色 12% 透明底，让等级一眼可辨又不刺眼 */
const levelBg = computed(() => {
    switch ((props.alert?.level || '').toUpperCase()) {
        case 'Y': case '黄色': return 'rgba(234, 179, 8, 0.14)';
        case 'O': case '橙色': return 'rgba(249, 115, 22, 0.14)';
        case 'R': case '红色': return 'rgba(239, 68, 68, 0.14)';
        case 'W': case '白色': return 'rgba(229, 231, 235, 0.14)';
        default: return 'rgba(59, 130, 246, 0.14)';
    }
});

// 预警类型文本 → 具体情景图标（暴雨/大雾/霾/高温/大风…），推不出时回退三角
const iconSvg = computed(() => weatherIconSvgOf(weatherAlertIconKey(props.alert?.type || '')));
</script>

<style scoped>
/* 岛上展开面板：对齐"系统通知展开"样式（IslandMsg 同一套体例，
   配合主组件把岛展开到 65px 高，不再挤在 42px 单行里字小量少） */
.light-alert-panel {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    width: 100%;
    height: 100%;
    align-items: center;
    padding: 0 34px 0 6px;
    gap: 12px;
    box-sizing: border-box;
    overflow: hidden;
    z-index: 5;
}

/* 左侧图标：30px 圆形衬底（对齐消息头像 35px 观感，天气场景用等级色衬底） */
.light-alert-icon {
    display: flex;
    width: 32px;
    height: 32px;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    flex-shrink: 0;
    transition: color 0.2s ease, background 0.2s ease;
}

.light-alert-icon-svg {
    display: inline-flex;
    align-items: center;
    justify-content: center;
}

.light-alert-icon-svg :deep(svg) {
    display: block;
    width: 20px;
    height: 20px;
}

/* 两行：上行等级 + 类型徽标，下行事件标题（超长省略，title 属性兜底全文） */
.light-alert-text-col {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    gap: 2px;
    overflow: hidden;
}

.light-alert-head {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: 6px;
    max-width: 100%;
}

/* 等级行 14px 粗体：对齐消息通知标题字号 */
.light-alert-level {
    font-size: 14px;
    font-weight: 700;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

/* 类型徽标：对齐消息通知的 app-name 胶囊 */
.light-alert-type {
    flex-shrink: 0;
    padding: 1px 6px;
    border-radius: 6px;
    background: rgba(150, 150, 150, 0.25);
    color: rgba(255, 255, 255, 0.9);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.2px;
    white-space: nowrap;
}

/* 正文 12.5px：对齐消息通知正文字号 */
.light-alert-title {
    overflow: hidden;
    width: 100%;
    font-size: 12.5px;
    font-weight: 400;
    line-height: 1.4;
    opacity: 0.75;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* 右侧 X 关闭（14px 避开 8px 边缘热区） */
.light-alert-close {
    position: absolute;
    top: 50%;
    right: 14px;
    display: flex;
    width: 26px;
    height: 26px;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: #888;
    cursor: pointer;
    transform: translateY(-50%);
    transition: all 0.2s ease;
    z-index: 6;
}

.light-alert-close:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
}

.light-alert-close svg {
    width: 15px;
    height: 15px;
}
</style>
