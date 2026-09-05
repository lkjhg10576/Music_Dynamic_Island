<template>
    <div class="cd-expanded-controls">
        <button class="cd-pause-btn" @click.stop="emit('toggle')" :title="cdPaused ? '继续' : '暂停'">
            <svg v-if="cdPaused" viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                <path d="M8 5v14l11-7z" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
            </svg>
        </button>
        <IslandCloseButton class="cd-close-btn" @close="emit('close')" />
    </div>
</template>

<script setup lang="ts">
import IslandCloseButton from './IslandCloseButton.vue';

defineProps<{
    cdPaused: boolean;
}>();

const emit = defineEmits<{
    (e: 'toggle'): void;
    (e: 'close'): void;
}>();
</script>

<style scoped>
/* 倒计时展开态控制按钮组 */
.cd-expanded-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    transform: translateX(8px) !important;
}

.cd-pause-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: rgba(255, 152, 0, 0.15);
    color: #ff9800;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid rgba(255, 152, 0, 0.3);
}

.cd-pause-btn:hover {
    background: rgba(255, 152, 0, 0.25);
}

.cd-close-btn {
    color: #ff9800 !important;
}

.cd-close-btn:hover {
    background-color: rgba(255, 152, 0, 0.15) !important;
    color: #ff9800 !important;
}
</style>
