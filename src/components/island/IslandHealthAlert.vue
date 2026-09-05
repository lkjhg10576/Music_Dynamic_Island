<template>
    <div class="health-alert-box">
        <span class="health-alert-icon">⏰</span>
        <span class="health-alert-text">{{ healthAlertLabel }}</span>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    healthAlertLabel: string;
}>();
</script>

<style scoped>
/* ===== 健康提醒 ===== */
.health-alert-box {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 100%;
}

.health-alert-icon {
    font-size: 18px;
    line-height: 1;
    animation: health-alert-pulse 1s ease-in-out infinite;
}

.health-alert-text {
    font-size: 15px;
    font-weight: 700;
    color: #fbbf24;
    white-space: nowrap;
}

@keyframes health-alert-pulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.15); }
}
</style>
