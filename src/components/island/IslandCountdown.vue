<template>
    <div class="countdown-text-box">
        <!-- 动态进度圆环：剩余进度随时间递减，主题色与倒计时文字一致 -->
        <IslandProgressRing :pct="ringPct" :color="ringColor" />
        <div class="countdown-info">
            <span v-if="isCountdownFinished" class="countdown-finished-text">倒计时结束</span>
            <span v-else class="countdown-time">{{ formattedIslandCdTime }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
import IslandProgressRing from './IslandProgressRing.vue';

defineProps<{
    formattedIslandCdTime: string;
    isCountdownFinished: boolean;
    /** 剩余进度百分比（0~100），驱动动态圆环 */
    ringPct: number;
    /** 圆环主题色（倒计时主题橙） */
    ringColor: string;
}>();
</script>

<style scoped>
/* ===== 倒计时样式 ===== */
.countdown-text-box {
    display: flex;
    align-items: center;
    gap: 8px;
}

.countdown-info {
    display: flex;
    align-items: center;
    gap: 4px;
}

.countdown-time {
    font-size: 18px;
    font-weight: 800;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: #ff9800;
    letter-spacing: 1px;
}

.countdown-finished-text {
    font-size: 13px;
    font-weight: 700;
    color: #ff9800;
    animation: cd-blink 1s ease-in-out infinite;
}

@keyframes cd-blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
}
</style>
