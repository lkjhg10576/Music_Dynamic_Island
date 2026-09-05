<template>
    <div class="right-circle rt-chip" @click.stop="emit('activate')"
        :style="{ ...coreContentStyle, cursor: 'pointer' }"
        :title="rtActivities[currentRtIndex] ? ('点击展开：' + rtActivities[currentRtIndex].id) : ''">
        <!-- 预览活动变化时（新活动加入/轮换/优先级重排）做缩小+淡入过渡，避免图标瞬间替换 -->
        <transition name="rt-swap" mode="out-in">
            <span v-if="rtActivities[currentRtIndex]" class="rt-chip-inner"
                :key="rtActivities[currentRtIndex].id"
                :style="{ color: rtActivities[currentRtIndex].accent || '#ffffff' }">
                <!-- 注册表 chip 契约的组件形态（如硬件监控的动态小圆环） -->
                <component v-if="chipContent.kind === 'component'" :is="chipContent.component"
                    v-bind="chipContent.props" />
                <!-- 缺省形态：活动注册的静态图标（stroke=currentColor 继承 accent 着色） -->
                <span v-else class="rt-chip-icon" v-html="chipContent.icon"></span>
            </span>
        </transition>
    </div>
</template>

<script setup lang="ts">
import type { CSSProperties } from 'vue';
import type { RtActivity } from './types';
import type { ChipContent } from '../../activities/registry';

defineProps<{
    rtActivities: RtActivity[];
    currentRtIndex: number;
    /** 当前预览活动的芯片内容（父组件按活动注册表 chip 契约解析） */
    chipContent: ChipContent;
    coreContentStyle: CSSProperties;
}>();

const emit = defineEmits<{
    (e: 'activate'): void;
}>();
</script>

<style scoped>
/* 右侧独立实时小球 */
.right-circle {
    position: absolute;
    right: 0;
    width: 38px;
    height: 38px;
    border-radius: 50% !important;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.05);
}

/* 多实时活动并行：单一常驻小图标 */
.rt-chip {
    /* 继承 .right-circle 的定位与尺寸；color 由内联 style 设置为活动 accent */
    background: rgba(0, 0, 0, 0.18);
}

.rt-chip-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    position: relative;
    z-index: 2;
}

.rt-chip-icon :deep(svg) {
    width: 18px;
    height: 18px;
    /* stroke 使用 currentColor，自动继承 .rt-chip 的 color */
}

/* 小图标内容包裹层：占满圆形区域并居中，accent 颜色随预览活动一起过渡 */
.rt-chip-inner {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
}

/* 预览活动切换过渡（新活动加入/轮换/优先级重排）：缩小淡出 → 弹性放大淡入 */
.rt-swap-enter-active {
    transition: opacity 0.24s ease, transform 0.24s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.rt-swap-leave-active {
    transition: opacity 0.16s ease, transform 0.16s ease;
}

.rt-swap-enter-from,
.rt-swap-leave-to {
    opacity: 0;
    transform: scale(0.4);
}
</style>
