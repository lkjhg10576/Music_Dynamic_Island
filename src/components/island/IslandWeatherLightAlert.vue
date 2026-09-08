<template>
    <div class="weather-light-alert-panel">
        <div class="light-alert-header">
            <div class="light-alert-icon" :class="'level-' + alert?.level?.toLowerCase()">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                    <line x1="12" y1="9" x2="12" y2="13"></line>
                    <line x1="12" y1="17" x2="12.01" y2="17"></line>
                </svg>
            </div>
            <div class="light-alert-title">{{ alert?.levelText || '蓝色预警' }}</div>
        </div>
        <div class="light-alert-body">
            <div class="light-alert-type">{{ alert?.type || '' }}</div>
            <div class="light-alert-text">{{ alert?.title || '' }}</div>
        </div>
        <div class="light-alert-footer">
            <span class="light-alert-hint">5s 后自动隐藏</span>
            <button class="light-alert-close" @click="emit('close')">关闭</button>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
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
</script>

<style scoped>
.weather-light-alert-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    min-width: 200px;
    max-width: 280px;
}

.light-alert-header {
    display: flex;
    align-items: center;
    gap: 10px;
}

.light-alert-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.light-alert-icon svg {
    width: 18px;
    height: 18px;
}

.light-alert-icon.level-b,
.light-alert-icon.level-blue {
    color: #3b82f6;
    background: rgba(59, 130, 246, 0.1);
}

.light-alert-icon.level-y,
.light-alert-icon.level-yellow {
    color: #eab308;
    background: rgba(234, 179, 8, 0.1);
}

.light-alert-icon.level-o,
.light-alert-icon.level-orange {
    color: #f97316;
    background: rgba(249, 115, 22, 0.1);
}

.light-alert-icon.level-r,
.light-alert-icon.level-red {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
}

.light-alert-icon.level-w,
.light-alert-icon.level-white {
    color: #e5e7eb;
    background: rgba(229, 231, 235, 0.1);
}

.light-alert-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--item-title-color, #fff);
}

.light-alert-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.light-alert-type {
    font-size: 12px;
    font-weight: 600;
    color: var(--item-desc-color, rgba(255,255,255,0.6));
}

.light-alert-text {
    font-size: 12px;
    color: var(--item-desc-color, rgba(255,255,255,0.6));
    line-height: 1.4;
    word-break: break-word;
}

.light-alert-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--control-border, rgba(255,255,255,0.08));
}

.light-alert-hint {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.light-alert-close {
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    background: transparent;
    color: var(--item-title-color, #fff);
    transition: all 0.2s ease;
    outline: none;
}

.light-alert-close:hover {
    background: var(--control-border, rgba(255,255,255,0.15));
    border-color: var(--accent-color, #10b981);
    color: var(--accent-color, #10b981);
}
</style>
