<template>
    <div class="system-toast-box" @click="emit('select')">

        <div v-if="sysToastType === 'app'" class="toast-icon app-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" opacity="0.3" />
                <path d="M8 12.5l3 3 5-6" stroke-width="2.5" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'lock'" class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <path d="M8 12V9a4 4 0 0 1 8 0v3" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'unlock'" class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <path d="M8 12V9a4 4 0 0 1 8 0" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'battery-charge'" class="toast-icon battery-charge-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
                <polygon points="11 7 8 12 12 12 11 17 14 12 10 12 11 7" stroke-width="1.5"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else-if="sysToastType === 'battery-low'" class="toast-icon battery-low-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" />
                <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" />
                <line x1="6" y1="12" x2="9" y2="12" stroke-width="4" stroke-linecap="round"
                    stroke-linejoin="round" />
            </svg>
        </div>

        <div v-else class="toast-icon sys-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" opacity="0.3" />
                <g transform="translate(6, 5.5) scale(0.5)">
                    <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" stroke-width="4"
                        stroke-linecap="round" stroke-linejoin="round" />
                    <path d="M13.73 21a2 2 0 0 1-3.46 0" stroke-width="4" stroke-linecap="round"
                        stroke-linejoin="round" />
                </g>
            </svg>
        </div>
        <div class="toast-text">{{ sysToastText }}</div>
    </div>
</template>

<script setup lang="ts">
import type { SysToastType } from '../../composables/useNotifications';

defineProps<{
    sysToastType: SysToastType;
    sysToastText: string;
}>();

const emit = defineEmits<{
    (e: 'select'): void;
}>();
</script>

<style scoped>
/* 系统操作通知样式 */
.system-toast-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    /* 右侧留白：避免文字伸入频谱/状态点区域（频谱在 left-capsule 右侧 flex 位） */
    padding-left: 0;
    padding-right: 4px;
    gap: 2px;
    z-index: 10;
    -webkit-app-region: no-drag;
    box-sizing: border-box;
    overflow: hidden;
}

.toast-icon {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transform: translateX(-8px);
}

/* 灵动岛通知 */
.toast-icon.app-icon {
    color: currentColor;
}

/* 系统通知使用跟随字体的原生对比色 (黑白) */
.toast-icon.sys-icon {
    color: currentColor;
    opacity: 0.85;
}

.toast-icon svg {
    width: 22px;
    height: 22px;
    display: block;
}

.toast-icon.battery-charge-icon {
    color: #34C759;
}

.toast-icon.battery-low-icon {
    color: #FF3B30;
}

.toast-text {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0.95;
    transform: translateX(-2px) translateY(-1px);
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    /* 兜底：异常超长文本省略；正常电源/电池文案靠动态岛宽完整显示 */
    max-width: 100%;
    box-sizing: border-box;
}
</style>
