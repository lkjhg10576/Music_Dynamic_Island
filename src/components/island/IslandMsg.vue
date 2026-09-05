<template>
    <div class="msg-box" @click="emit('select')" style="cursor: pointer;">
        <div class="msg-avatar">
            <img :src="currentMsgIcon" alt="消息图标" class="msg-avatar-img">
        </div>
        <div class="msg-text-wrapper">
            <div class="msg-title">
                <span class="sender-name">{{ msgTitle }}</span>
                <span class="app-name">{{ msgAppName }}</span>
            </div>
            <div class="msg-body">{{ msgBody }}</div>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    msgTitle: string;
    msgAppName: string;
    msgBody: string;
    currentMsgIcon: string;
}>();

const emit = defineEmits<{
    (e: 'select'): void;
}>();
</script>

<style scoped>
/* 灵动岛消息通知样式 */
.msg-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    padding: 0 45px 0 0px;
    box-sizing: border-box;
    z-index: 10;
    gap: 12px;
    -webkit-app-region: no-drag;
    transition: opacity 0.15s ease;
}

/* F11 通知可点击：hover 时给一点视觉反馈 */
.msg-box:hover {
    opacity: 0.8;
}

.msg-box:active {
    opacity: 0.65;
}

/* 预制消息图标/头像样式 */
.msg-avatar {
    width: 35px;
    height: 35px;
    border-radius: 50%;
    background: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.msg-avatar-img {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    object-fit: cover;
}

/* 文本靠左对齐包裹层 */
.msg-text-wrapper {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    overflow: hidden;
    flex-grow: 1;
}

/* 消息弹窗容器 */
.msg-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 700;
    line-height: 1.4;
    width: 100%;
    overflow: hidden;
}

/* 发送者昵称（允许超长省略号） */
.sender-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* 尾部的程序名 */
.app-name {
    font-size: 10.5px;
    font-weight: 600;
    flex-shrink: 0;
    padding: 2px 6px;
    border-radius: 6px;
    background-color: rgba(150, 150, 150, 0.25);
    color: inherit;
    opacity: 0.9;
    letter-spacing: 0.2px;
    transform: translateY(-0.5px);
}

/* 调大后的内容样式 */
.msg-body {
    font-size: 12.5px;
    line-height: 1.4;
    opacity: 0.75;
    text-align: left;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
</style>
