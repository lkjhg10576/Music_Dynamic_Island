<template>
    <div class="clipboard-box">
        <div class="clipboard-icon">
            <img :src="clipboardIcon" alt="剪贴板" class="clipboard-icon-img">
        </div>
        <div class="clipboard-text-wrapper">
            <div class="clipboard-title">检测到复制了链接</div>
            <div class="clipboard-link">{{ link }}</div>
        </div>
        <button class="clipboard-open-btn" title="打开链接" @click.stop="emit('open')">
            <img :src="openLinkIcon" alt="打开链接" class="clipboard-open-img">
        </button>
    </div>
</template>

<script setup lang="ts">
import clipboardIcon from '../../assets/Clipboard.png';
import openLinkIcon from '../../assets/open_the_link.png';

defineProps<{
    link: string;
}>();

const emit = defineEmits<{
    (e: 'open'): void;
}>();
</script>

<style scoped>
/* 剪贴板链接卡片（移植自上游 2.4.5，样式对齐系统通知组件的占位方式） */
.clipboard-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    padding: 0 12px;
    box-sizing: border-box;
    z-index: 10;
    gap: 12px;
    -webkit-app-region: no-drag;
}

.clipboard-icon {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    overflow: hidden;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.clipboard-icon-img {
    width: 24px;
    height: 24px;
    object-fit: contain;
}

.clipboard-text-wrapper {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    overflow: hidden;
    flex-grow: 1;
    min-width: 0;
}

.clipboard-title {
    font-size: 13.5px;
    font-weight: 700;
    line-height: 1.4;
    white-space: nowrap;
}

.clipboard-link {
    font-size: 12.5px;
    line-height: 1.4;
    opacity: 0.78;
    text-align: left;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: ltr;
    unicode-bidi: plaintext;
}

/* 右侧的打开链接按钮 */
.clipboard-open-btn {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: none;
    outline: none;
    background-color: rgba(150, 150, 150, 0.25);
    color: inherit;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: background-color 0.2s ease, transform 0.1s ease;
}

.clipboard-open-btn:hover {
    background-color: rgba(255, 255, 255, 0.35);
}

.clipboard-open-btn:active {
    transform: scale(0.92);
}

.clipboard-open-img {
    width: 20px;
    height: 20px;
    object-fit: contain;
}
</style>
