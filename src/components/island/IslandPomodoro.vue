<template>
    <div class="pomodoro-text-box">
        <svg viewBox="0 0 24 24" class="pomodoro-svg" :class="pomodoroPhaseClass" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
        </svg>
        <div class="pomodoro-info">
            <span class="pomodoro-time" :class="pomodoroPhaseClass">{{ formattedIslandPomoTime }}</span>
            <span class="pomodoro-cycle-badge" v-if="pomodoroRemainingCycles > 0">{{ pomodoroRemainingCycles }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    formattedIslandPomoTime: string;
    pomodoroPhaseClass: string;
    pomodoroRemainingCycles: number;
}>();
</script>

<style scoped>
/* 番茄钟文本排版 */
.pomodoro-text-box {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    transform: translateX(-5px) !important;
}

.pomodoro-info {
    display: flex;
    flex-direction: row;
    align-items: center;
}

.pomodoro-time {
    font-size: 18px;
    font-weight: bold;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.5px;
    transform: translateY(-0.5px);
    transition: color 0.3s ease;
}

.pomodoro-time.phase-focus {
    color: #ff4757;
}

.pomodoro-time.phase-break {
    color: #2196f3;
}

.pomodoro-svg {
    width: 24px;
    height: 24px;
    transition: color 0.3s ease;
}

.pomodoro-svg.phase-focus {
    color: #ff4757;
}

.pomodoro-svg.phase-break {
    color: #2196f3;
}

/* 番茄钟剩余轮数徽章 */
.pomodoro-cycle-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 18px;
    padding: 0 5px;
    margin-left: 6px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 800;
    background: rgba(128, 128, 128, 0.2);
    color: var(--item-title-color, #888);
    font-variant-numeric: tabular-nums;
    transform: translateY(-0.5px);
}
</style>
