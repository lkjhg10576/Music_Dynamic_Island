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
/* 岛上展开面板：占满整岛（整屏面板展开时左侧内容已由主组件守卫卸载） */
.light-alert-panel {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    width: 100%;
    height: 100%;
    align-items: center;
    padding: 5px 30px 5px 8px;
    gap: 8px;
    box-sizing: border-box;
    overflow: hidden;
}

.light-alert-icon {
    display: flex;
    width: 26px;
    height: 26px;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
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
    width: 16px;
    height: 16px;
}

/* 两行：上行等级 + 类型，下行事件标题（超长省略，title 属性兜底全文） */
.light-alert-text-col {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    overflow: hidden;
}

.light-alert-head {
    display: flex;
    align-items: baseline;
    min-width: 0;
    gap: 6px;
}

.light-alert-level {
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
}

.light-alert-type {
    min-width: 0;
    overflow: hidden;
    color: rgba(255, 255, 255, 0.55);
    font-size: 9px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.light-alert-title {
    overflow: hidden;
    font-size: 10px;
    font-weight: 500;
    line-height: 1.15;
    opacity: 0.72;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* 右侧 X 关闭（10px 避开 8px 边缘调宽热区） */
.light-alert-close {
    position: absolute;
    top: 50%;
    right: 10px;
    display: flex;
    width: 22px;
    height: 22px;
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
}

.light-alert-close:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
}

.light-alert-close svg {
    width: 14px;
    height: 14px;
}
</style>
