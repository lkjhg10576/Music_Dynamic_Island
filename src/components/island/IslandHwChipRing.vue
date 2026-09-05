<template>
    <svg viewBox="0 0 36 36" class="rt-chip-hw-ring">
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
</template>

<script setup lang="ts">
// 硬件监控芯片的动态小圆环：从 IslandRtChip 的 hardware 特例分支抽出，
// 经活动注册表（activities/registry.ts）的 chip 契约按需接入，IslandRtChip 本体不再感知具体活动
defineProps<{
    hwMode: string;
    hwCpuPct: number;
    hwMemPct: number;
    hwRingPct: number;
    hwRingColor: string;
}>();
</script>

<style scoped>
.rt-chip-hw-ring {
    width: 24px;
    height: 24px;
    display: block;
}
</style>
