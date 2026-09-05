<template>
    <div :class="['status-dot', networkStatus]"></div>
</template>

<script setup lang="ts">
defineProps<{
    networkStatus: 'good' | 'warning' | 'error';
}>();
</script>

<style scoped>
/* 与 .inner-wrapper 共享的层叠定位（原父组件共享规则拆分） */
.status-dot {
    position: relative;
    z-index: 2;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    transition: background-color 0.4s ease;
}

/* 去掉发光阴影，改为纯粹的扁平化圆点，干净利落 */
.good {
    background-color: #34C759;
}

.warning {
    background-color: #FFCC00;
}

.error {
    background-color: #FF3B30;
}
</style>
