<template>
    <div class="hw-expanded-detail">
        <div class="hw-detail-row">
            <span class="hw-detail-label">CPU</span>
            <span class="hw-detail-val" :class="{ 'high': hwCpuPct >= 80 }">{{ Math.round(hwCpuPct) }}%</span>
        </div>
        <div class="hw-detail-row">
            <span class="hw-detail-label">RAM</span>
            <span class="hw-detail-val" :class="{ 'high': hwMemPct >= 80 }">{{ Math.round(hwMemPct) }}%</span>
        </div>
        <div class="hw-close-btn-x" @click.stop="emit('close')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    hwCpuPct: number;
    hwMemPct: number;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();
</script>

<style scoped>
/* 硬件监控展开态：CPU / 内存 详情 */
.hw-expanded-detail {
    display: flex;
    align-items: center;
    gap: 16px;
    position: relative;
    width: 100%;
    height: 100%;
    padding-left: 50px;
    padding-right: 34px;
}

.hw-detail-row {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1.15;
}

.hw-detail-label {
    font-size: 9px;
    font-weight: 700;
    opacity: 0.6;
    letter-spacing: 0.5px;
}

.hw-detail-val {
    font-size: 15px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.5px;
}

.hw-detail-val.high {
    color: #ff4757;
}

.hw-close-btn-x {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: #888;
    cursor: pointer;
    transition: all 0.2s ease;
}

.hw-close-btn-x:hover {
    color: #ff4757;
    background-color: rgba(255, 71, 87, 0.15);
}

.hw-close-btn-x svg {
    width: 14px;
    height: 14px;
}
</style>
