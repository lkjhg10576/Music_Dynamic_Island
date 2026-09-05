<template>
    <div class="right-circle rt-chip" @click.stop="emit('activate')"
        :style="{ ...coreContentStyle, cursor: 'pointer' }"
        :title="rtActivities[currentRtIndex] ? ('点击展开：' + rtActivities[currentRtIndex].id) : ''">
        <!-- 预览活动变化时（新活动加入/轮换/优先级重排）做缩小+淡入过渡，避免图标瞬间替换 -->
        <transition name="rt-swap" mode="out-in">
            <span v-if="rtActivities[currentRtIndex]" class="rt-chip-inner"
                :key="rtActivities[currentRtIndex].id"
                :style="{ color: rtActivities[currentRtIndex].accent || '#ffffff' }">
                <!-- 硬件监控：显示动态小圆环 -->
                <svg v-if="rtActivities[currentRtIndex].id === 'hardware'" viewBox="0 0 36 36" class="rt-chip-hw-ring">
                <circle cx="18" cy="18" r="14" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="3" />
                <template v-if="hwMode === 'dual'">
                    <circle cx="18" cy="18" r="14" fill="none"
                        :stroke="hwCpuPct >= 80 ? '#a855f7' : '#ffffff'" stroke-width="3"
                        :stroke-dasharray="`${(hwCpuPct / 100) * 87.96} 87.96`"
                        stroke-linecap="round" transform="rotate(-90 18 18)"
                        style="transition: stroke-dasharray 0.5s ease;" />
                    <circle cx="18" cy="18" r="8" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="2.5" />
                    <circle cx="18" cy="18" r="8" fill="none"
                        :stroke="hwMemPct >= 80 ? '#ff4757' : '#3b82f6'" stroke-width="2.5"
                        :stroke-dasharray="`${(hwMemPct / 100) * 50.27} 50.27`"
                        stroke-linecap="round" transform="rotate(-90 18 18)"
                        style="transition: stroke-dasharray 0.5s ease;" />
                </template>
                <template v-else>
                    <circle cx="18" cy="18" r="14" fill="none"
                        :stroke="hwRingColor" stroke-width="3"
                        :stroke-dasharray="`${(hwRingPct / 100) * 87.96} 87.96`"
                        stroke-linecap="round" transform="rotate(-90 18 18)"
                        style="transition: stroke-dasharray 0.5s ease;" />
                </template>
                </svg>
                <!-- 其他实时活动：保留原有静态图标 -->
                <span v-else class="rt-chip-icon" v-html="rtActivities[currentRtIndex]?.icon || ''"></span>
            </span>
        </transition>
    </div>
</template>

<script setup lang="ts">
import type { CSSProperties } from 'vue';
import type { RtActivity } from './types';

defineProps<{
    rtActivities: RtActivity[];
    currentRtIndex: number;
    hwMode: string;
    hwCpuPct: number;
    hwMemPct: number;
    hwRingPct: number;
    hwRingColor: string;
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

.rt-chip-hw-ring {
    width: 24px;
    height: 24px;
    display: block;
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
