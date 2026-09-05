<template>
    <div class="speed-box">
        <transition name="speed-fade" mode="out-in">
            <div v-if="isShowingUpload" class="speed-item" key="upload">
                <span :class="['label', { 'high-traffic': isHighUpload }]">⬆</span>
                <span class="value">{{ uploadSpeed }}</span>
            </div>
            <div v-else class="speed-item" key="download">
                <span :class="['label', { 'high-traffic': isHighDownload }]">⬇</span>
                <span class="value">{{ downloadSpeed }}</span>
            </div>
        </transition>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    isShowingUpload: boolean;
    uploadSpeed: string;
    downloadSpeed: string;
    isHighUpload: boolean;
    isHighDownload: boolean;
}>();
</script>

<style scoped>
/* 修改网速盒子布局，强制靠左，并加入左侧内边距 */
.speed-box {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    height: 100%;
}

/* 与 .music-ctl-box 共享的绝对定位布局（原父组件级联顺序：此规则在后，justify-content 以此为准） */
.speed-box {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    justify-content: center;
    gap: 4px;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
}

.speed-item {
    display: flex;
    align-items: center;
    gap: 6px;
    /* 稍微拉开箭头和数字的距离 */
    transform: translateY(-1px);
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
}

.label {
    font-size: 10px;
    /* 稍微调大箭头 */
    color: currentColor;
    opacity: 0.5;
    font-weight: 800;
    padding: 2px 5px;
    border-radius: 4px;
    transition: all 0.3s ease;
    background: rgba(150, 150, 150, 0.15);
    /* 默认给一个淡淡的底色，增加质感 */
}

/* 高流量时的 label 样式 */
.label.high-traffic {
    color: currentColor;
    opacity: 1;
    background: rgba(255, 255, 255, 0.25);
}

:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .label.high-traffic {
    background: rgba(0, 0, 0, 0.15);
}

.value {
    font-size: 12px;
    transform: translateY(-0.5px);
    font-weight: 600;
    letter-spacing: 0.2px;
    font-variant-numeric: tabular-nums;
    min-width: 65px;
    text-align: left;
}

.value.high-usage {
    color: #f06861 !important;
}

/* 网速轮换的淡入淡出动画 */
.speed-fade-enter-active,
.speed-fade-leave-active {
    transition: opacity 0.3s ease, transform 0.3s ease;
}

.speed-fade-enter-from {
    opacity: 0;
    transform: translateY(4px);
    /* 微微从下方滑入 */
}

.speed-fade-leave-to {
    opacity: 0;
    transform: translateY(-4px);
    /* 微微向上滑出 */
}
</style>
