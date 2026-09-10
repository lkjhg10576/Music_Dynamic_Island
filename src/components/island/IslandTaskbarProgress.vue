<template>
    <div class="taskbar-progress-detail">
        <div class="taskbar-progress-head">
            <div class="taskbar-progress-title">
                <span>{{ appName || '任务栏进度' }}</span>
                <small>{{ percent }}%</small>
            </div>
            <button class="taskbar-progress-close" type="button" @click.stop="emit('close')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>
        <div class="taskbar-progress-bar">
            <span :style="{ width: `${percent}%` }" :class="{ indeterminate: percent === 0 }"></span>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    appName: string;
    percent: number;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();
</script>

<style scoped>
/* 任务栏进度展开态（仿照 IslandPrintQueue 配色, accent 用绿色 #22c55e） */
.taskbar-progress-detail {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 5px 30px 5px 8px;
    gap: 4px;
    box-sizing: border-box;
}

.taskbar-progress-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
}

.taskbar-progress-title {
    display: flex;
    align-items: baseline;
    min-width: 0;
    gap: 6px;
    color: #86efac;
    font-size: 11px;
    font-weight: 700;
}

.taskbar-progress-title small {
    min-width: 0;
    overflow: hidden;
    color: rgba(255, 255, 255, 0.55);
    font-size: 9px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.taskbar-progress-close {
    position: absolute;
    top: 50%;
    right: 6px;
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
}

.taskbar-progress-close:hover {
    background: rgba(34, 197, 94, 0.2);
    color: #86efac;
}

.taskbar-progress-close svg {
    width: 14px;
    height: 14px;
}

.taskbar-progress-bar {
    height: 3px;
    overflow: hidden;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.12);
}

.taskbar-progress-bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: #22c55e;
    transition: width 0.25s ease;
}

.taskbar-progress-bar span.indeterminate {
    width: 35% !important;
    background: rgba(34, 197, 94, 0.6);
}
</style>
