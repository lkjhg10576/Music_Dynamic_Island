<template>
    <div class="countdown-text-box">
        <svg viewBox="0 0 24 24" class="countdown-svg" fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
        </svg>
        <div class="countdown-info">
            <span v-if="isCountdownFinished" class="countdown-finished-text">倒计时结束</span>
            <span v-else class="countdown-time">{{ formattedIslandCdTime }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    formattedIslandCdTime: string;
    isCountdownFinished: boolean;
}>();
</script>

<style scoped>
/* ===== 倒计时样式 ===== */
.countdown-text-box {
    display: flex;
    align-items: center;
    gap: 8px;
}

.countdown-svg {
    width: 24px;
    height: 24px;
    color: #ff9800;
    transition: color 0.3s ease;
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
