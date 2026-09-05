<template>
    <svg viewBox="0 0 36 36" class="rt-chip-hw-ring">
        <circle cx="18" cy="18" r="14" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="3" />
        <!-- 双圆环模式：外/内环指标可自定义（阶段 E），不可用指标（无电池）渲染置灰空环 -->
        <template v-if="hwMode === 'dual'">
            <circle cx="18" cy="18" r="14" fill="none"
                :stroke="slotColor(hwRingOuter)" stroke-width="3"
                :stroke-dasharray="slotDash(hwRingOuter, 87.96)"
                stroke-linecap="round" transform="rotate(-90 18 18)"
                style="transition: stroke-dasharray 0.5s ease;" />
            <circle cx="18" cy="18" r="8" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="2.5" />
            <circle cx="18" cy="18" r="8" fill="none"
                :stroke="slotColor(hwRingInner)" stroke-width="2.5"
                :stroke-dasharray="slotDash(hwRingInner, 50.27)"
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
</template>

<script setup lang="ts">
// 硬件监控芯片的动态小圆环：从 IslandRtChip 的 hardware 特例分支抽出，
// 经活动注册表（activities/registry.ts）的 chip 契约按需接入，IslandRtChip 本体不再感知具体活动
import { computed } from 'vue';
import {
    hwMetricSlotColor, hwMetricSlotDash,
    type HwMetric, type HwMetricVals,
} from '../../utils/hwMetrics';

const props = defineProps<{
    hwMode: string;
    hwCpuPct: number;
    hwMemPct: number;
    hwRingPct: number;
    hwRingColor: string;
    hwRingOuter: HwMetric;
    hwRingInner: HwMetric;
    hwBatteryPct: number;
    hwDiskPct: number;
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
</script>

<style scoped>
.rt-chip-hw-ring {
    width: 24px;
    height: 24px;
    display: block;
}
</style>
