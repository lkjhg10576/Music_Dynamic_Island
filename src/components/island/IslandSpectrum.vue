<template>
    <div class="audio-spectrum" :class="{ 'is-playing': isPlaying, 'expanded': isMusicExpanded }">
        <span class="bar" v-for="(val, index) in bars" :key="index"
            :style="{ transform: `scaleY(${val})`, backgroundColor: barColor, transition: 'transform 0.08s ease-out, background-color 0.08s linear' }"></span>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    bars: number[];
    isPlaying: boolean;
    isMusicExpanded: boolean;
    barColor: string;
}>();
</script>

<style scoped>
/* 与 .inner-wrapper 共享的层叠定位（原父组件共享规则拆分） */
.audio-spectrum {
    position: relative;
    z-index: 2;
}

/* 音乐律动频谱样式 */
.audio-spectrum {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 12px;
    padding-right: 2px;
}

/* 暂停状态下的竖线（统一高度） */
.audio-spectrum .bar {
    width: 2px;
    height: 18px;
    background-color: #b6e0ee;
    border-radius: 3px;
    transform-origin: center;
    /* 改用极速的 ease-out 过渡，让前端完美衔接后端的帧率 */
    transition: transform 0.08s ease-out, background-color 0.08s linear;
    will-change: transform;
}

.audio-spectrum.expanded {
    position: absolute;
    right: 18px !important;
    top: 27px !important;
    transform: scale(1.3);
    /* 把 all 换成具体的属性，防止抖动 */
    transition: opacity 0.3s ease, transform 0.3s ease !important;
}
</style>
