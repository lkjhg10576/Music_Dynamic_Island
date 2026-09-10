<template>
    <span class="weather-chip" :style="{ color: levelColor }" :title="chipTitle">
        <span class="weather-chip-svg" v-html="iconSvg"></span>
    </span>
</template>

<script setup lang="ts">
// 恶劣天气轻提示的岛上芯片：按"具体情况"取图标（暴雨→雨、大雾→雾、高温→温度计…），
// 按预警等级着色（B 蓝 / Y 黄 / O 橙 / R 红 / W 白）。未展开时只显示此图标，不占岛文字空间。
// 图标表与解析口径见 utils/weather.ts，与系统 toast、轻提示面板共用同一份，避免三处图标不一致。
import { computed } from 'vue';
import { weatherAlertIconKey, weatherIconSvgOf } from '../../utils/weather';

const props = withDefaults(defineProps<{
    /** 预警等级：单字母 B/Y/O/R/W，或中文 蓝色/黄色/橙色/红色/白色 */
    level?: string;
    /** 预警类型（如 暴雨 / 大雾 / 高温），同时决定图标与 hover 提示 */
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

// 类型文本推不出具体情景时回退三角（alert），等级颜色仍然生效
const iconSvg = computed(() => weatherIconSvgOf(weatherAlertIconKey(props.type)));

const chipTitle = computed(() => props.type || '气象预警');
</script>

<style scoped>
.weather-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
}

.weather-chip-svg {
    display: inline-flex;
    align-items: center;
    justify-content: center;
}

.weather-chip-svg :deep(svg) {
    display: block;
    width: 18px;
    height: 18px;
}
</style>
