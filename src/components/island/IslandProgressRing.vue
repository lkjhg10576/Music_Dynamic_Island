<template>
    <svg viewBox="0 0 36 36" class="progress-ring-svg" :style="{ width: `${size}px`, height: `${size}px` }">
        <!-- 轨道底色 -->
        <circle cx="18" cy="18" :r="RING_RADIUS" fill="none" :stroke="RING_TRACK_COLOR" :stroke-width="RING_STROKE_WIDTH" />
        <!-- 剩余进度环：随时间递减（起始满环 → 结束空环），颜色随活动主题色/番茄钟阶段平滑过渡 -->
        <circle cx="18" cy="18" :r="RING_RADIUS" fill="none"
            :stroke="color" :stroke-width="RING_STROKE_WIDTH"
            :stroke-dasharray="dash"
            stroke-linecap="round" transform="rotate(-90 18 18)"
            class="progress-ring-fill" />
    </svg>
</template>

<script setup lang="ts">
// 通用时间进度圆环：与硬件监控单圆环（IslandHardwareRing）同一套视觉规格，
// 供番茄钟 / 倒计时的左侧胶囊图标与右侧实时活动小圆环（注册表 chip 契约）共用
import { computed } from 'vue';
import {
    RING_CIRCUMFERENCE, RING_ICON_SIZE, RING_RADIUS, RING_STROKE_WIDTH, RING_TRACK_COLOR,
    progressRingDash,
} from '../../utils/progressRing';

const props = withDefaults(defineProps<{
    /** 剩余进度百分比（0~100），起始满环、结束空环 */
    pct: number;
    /** 圆环主题色（番茄钟按专注/休息切换，倒计时固定主题色） */
    color: string;
    /** 渲染尺寸（px），默认与岛上既有图标一致 24 */
    size?: number;
}>(), {
    size: RING_ICON_SIZE,
});

const dash = computed(() => progressRingDash(props.pct, RING_CIRCUMFERENCE));
</script>

<style scoped>
.progress-ring-svg {
    flex-shrink: 0;
    display: block;
}

/* 进度随时间递减：dasharray 平滑推进；主题色随番茄钟阶段/活动切换渐变 */
.progress-ring-fill {
    transition: stroke-dasharray 0.5s ease, stroke 0.3s ease;
}
</style>
