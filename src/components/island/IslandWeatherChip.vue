<template>
    <span class="weather-chip" :style="{ color: levelColor }" :title="type || '气象预警'">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
            <line x1="12" y1="9" x2="12" y2="13"></line>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
        </svg>
    </span>
</template>

<script setup lang="ts">
// 恶劣天气轻提示的岛上芯片：18px 三角预警图标，按预警等级着色
// （B 蓝 / Y 黄 / O 橙 / R 红 / W 白），未展开时只显示此图标，不占岛文字空间。
import { computed } from 'vue';

const props = withDefaults(defineProps<{
    /** 预警等级：单字母 B/Y/O/R/W，或中文 蓝色/黄色/橙色/红色/白色 */
    level?: string;
    /** 预警类型（如 暴雨 / 高温），用于 title 提示 */
    type?: string;
}>(), {
    level: 'B',
    type: '',
});

const levelColor = computed(() => {
    switch ((props.level || '').toUpperCase()) {
        case 'Y': case '黄色': return '#eab308';
        case 'O': case '橙色': return '#f97316';
        case 'R': case '红色': return '#ef4444';
        case 'W': case '白色': return '#e5e7eb';
        default: return '#3b82f6'; // B / 蓝色 / 未知
    }
});
</script>

<style scoped>
.weather-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
}

.weather-chip svg {
    display: block;
    width: 18px;
    height: 18px;
}
</style>
