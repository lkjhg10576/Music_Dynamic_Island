<template>
    <div class="island-close-btn" @click.stop="emit('close')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
    </div>
</template>

<script setup lang="ts">
const emit = defineEmits<{
    (e: 'close'): void;
}>();
</script>

<style scoped>
/* 实时活动全岛关闭按钮 */
.island-close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    color: #888;
    cursor: pointer;
    transition: all 0.2s ease;
    border-radius: 50%;
    z-index: 10;
    transform: translateX(8px) !important;
}

.island-close-btn:hover {
    color: #ff4757;
    background-color: rgba(255, 71, 87, 0.15);
    transform: scale(1.15);
}

.island-close-btn svg {
    width: 20px;
    height: 20px;
}
</style>
