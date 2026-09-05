<template>
    <div class="hardware-ring-box" :class="{ 'is-hw-expanded': isHardwareExpanded }"
        @click.stop="emit('expand')" style="cursor: pointer;">
        <svg viewBox="0 0 36 36" class="hw-ring-svg">
            <!-- 背景圆环 -->
            <circle cx="18" cy="18" r="14" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="3" />
            <!-- 双圆环模式 -->
            <template v-if="hwMode === 'dual'">
                <circle cx="18" cy="18" r="14" fill="none"
                    :stroke="hwCpuPct >= 80 ? '#a855f7' : '#ffffff'" stroke-width="3"
                    :stroke-dasharray="`${(hwCpuPct / 100) * 87.96} 87.96`"
                    stroke-linecap="round" transform="rotate(-90 18 18)"
                    style="transition: stroke-dasharray 0.5s ease;" />
                <circle cx="18" cy="18" r="8" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="2.5" />
                <circle cx="18" cy="18" r="8" fill="none"
                    :stroke="hwMemPct >= 80 ? '#ff4757' : '#3b82f6'" stroke-width="2.5"
                    :stroke-dasharray="`${(hwMemPct / 100) * 50.27} 50.27`"
                    stroke-linecap="round" transform="rotate(-90 18 18)"
                    style="transition: stroke-dasharray 0.5s ease;" />
            </template>
            <!-- 单圆环 / 轮换模式 -->
            <template v-else>
                <circle cx="18" cy="18" r="14" fill="none"
                    :stroke="hwRingColor" stroke-width="3"
                    :stroke-dasharray="`${(hwRingPct / 100) * 87.96} 87.96`"
                    stroke-linecap="round" transform="rotate(-90 18 18)"
                    style="transition: stroke-dasharray 0.5s ease;" />
            </template>
        </svg>
        <!-- 折叠态才显示标签文字，展开态只保留圆环（避免与右侧 CPU/RAM 详情重叠） -->
        <template v-if="!isHardwareExpanded">
            <span class="hw-ring-label" v-if="hwMode !== 'dual'">
                <span class="hw-metric-name">{{ hwActiveMetric === 'cpu' ? 'CPU' : 'RAM' }}</span>
                <span class="hw-metric-val" :class="{ 'high': hwRingPct >= 80 }">{{ Math.round(hwRingPct) }}%</span>
            </span>
            <span class="hw-ring-label hw-dual-label" v-else>
                <span class="hw-dual-item" :class="{ 'high': hwCpuPct >= 80 }">C{{ Math.round(hwCpuPct) }}</span>
                <span class="hw-dual-item" :class="{ 'high': hwMemPct >= 80 }">M{{ Math.round(hwMemPct) }}</span>
            </span>
        </template>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    hwMode: string;
    hwCpuPct: number;
    hwMemPct: number;
    hwRingPct: number;
    hwRingColor: string;
    hwActiveMetric: string;
    isHardwareExpanded: boolean;
}>();

const emit = defineEmits<{
    (e: 'expand'): void;
}>();
</script>

<style scoped>
/* 硬件监控主环形显示 */
.hardware-ring-box {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    height: 100%;
    padding-left: 6px;
    gap: 10px;
}

/* 展开态：圆环左移到最左侧（与音乐专辑封面一致），只保留圆环图标 */
.hardware-ring-box.is-hw-expanded {
    padding-left: 5px;
    width: auto;
}

.hw-ring-svg {
    width: 30px;
    height: 30px;
    flex-shrink: 0;
}

.hw-ring-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
}

.hw-metric-name {
    font-size: 9px;
    font-weight: 700;
    opacity: 0.5;
    letter-spacing: 0.5px;
    text-transform: uppercase;
}

.hw-metric-val {
    font-size: 13px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
}

.hw-metric-val.high {
    color: #ff4757;
}

.hw-dual-label {
    gap: 8px;
}

.hw-dual-item {
    font-size: 12px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
}

.hw-dual-item.high {
    color: #ff4757;
}
</style>
