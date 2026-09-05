<template>
    <div class="hardware-ring-box" :class="{ 'is-hw-expanded': isHardwareExpanded }"
        @click.stop="emit('expand')" style="cursor: pointer;">
        <svg viewBox="0 0 36 36" class="hw-ring-svg">
            <!-- 背景圆环 -->
            <circle cx="18" cy="18" r="14" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="3" />
            <!-- 双圆环模式：外/内环指标可自定义（阶段 E），不可用指标（无电池）渲染置灰空环 -->
            <template v-if="hwMode === 'dual'">
                <circle cx="18" cy="18" r="14" fill="none"
                    :stroke="slotColor(hwRingOuter)" stroke-width="3"
                    :stroke-dasharray="slotDash(hwRingOuter, 87.96)"
                    stroke-linecap="round" transform="rotate(-90 18 18)"
                    style="transition: stroke-dasharray 0.5s ease;" />
                <circle cx="18" cy="18" r="8" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="2.5" />
                <circle cx="18" cy="18" r="8" fill="none"
                    :stroke="slotColor(hwRingInner)" stroke-width="2.5"
                    :stroke-dasharray="slotDash(hwRingInner, 50.27)"
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
        <!-- 折叠态才显示标签文字，展开态只保留圆环（避免与右侧指标详情重叠） -->
        <template v-if="!isHardwareExpanded">
            <span class="hw-ring-label" v-if="hwMode !== 'dual'">
                <span class="hw-metric-name">{{ HW_METRIC_ISLAND_LABEL[hwActiveMetric] }}</span>
                <span class="hw-metric-val" :class="{ 'high': slotHigh(hwActiveMetric) }">{{
                    slotText(hwActiveMetric) }}</span>
            </span>
            <span class="hw-ring-label hw-dual-label" v-else>
                <span class="hw-dual-item" :class="{ 'high': slotHigh(hwRingOuter) }">{{ slotShort(hwRingOuter)
                    }}</span>
                <span class="hw-dual-item" :class="{ 'high': slotHigh(hwRingInner) }">{{ slotShort(hwRingInner)
                    }}</span>
            </span>
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import {
    HW_METRIC_ISLAND_LABEL,
    hwMetricSlotColor, hwMetricSlotDash, hwMetricSlotHigh, hwMetricSlotShort, hwMetricSlotText,
    type HwMetric, type HwMetricVals,
} from '../../utils/hwMetrics';

const props = defineProps<{
    hwMode: string;
    hwCpuPct: number;
    hwMemPct: number;
    hwRingPct: number;
    hwRingColor: string;
    hwActiveMetric: HwMetric;
    hwRingOuter: HwMetric;
    hwRingInner: HwMetric;
    hwBatteryPct: number;
    hwDiskPct: number;
    isHardwareExpanded: boolean;
}>();

const emit = defineEmits<{
    (e: 'expand'): void;
}>();

// 槽位渲染工具的实时值集合（阶段 E：cpu/mem 之外新增 battery/disk）
const hwVals = computed<HwMetricVals>(() => ({
    cpu: props.hwCpuPct,
    mem: props.hwMemPct,
    battery: props.hwBatteryPct,
    disk: props.hwDiskPct,
}));

const slotColor = (metric: HwMetric) => hwMetricSlotColor(metric, hwVals.value);
const slotDash = (metric: HwMetric, circumference: number) => hwMetricSlotDash(metric, hwVals.value, circumference);
const slotText = (metric: HwMetric) => hwMetricSlotText(metric, hwVals.value);
const slotShort = (metric: HwMetric) => hwMetricSlotShort(metric, hwVals.value);
const slotHigh = (metric: HwMetric) => hwMetricSlotHigh(metric, hwVals.value);
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
