<template>
    <div class="print-queue-detail">
        <div class="print-queue-head">
            <div class="print-queue-title">
                <span>打印队列</span>
                <small>{{ defaultPrinter || '默认打印机' }} · {{ jobs.length }} 项</small>
            </div>
            <button class="print-queue-close" type="button" title="关闭打印队列" @click.stop="emit('close')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>
        <div class="print-job-list">
            <div v-for="job in jobs" :key="job.jobId" class="print-job-row">
                <div class="print-job-main">
                    <span class="print-job-document" :title="job.document">{{ job.document || '未命名文档' }}</span>
                    <span class="print-job-status">{{ job.status || '排队中' }}</span>
                </div>
                <div class="print-job-meta">
                    <span>{{ job.pagesPrinted }} / {{ job.totalPages || '?' }} 页</span>
                    <span v-if="job.printer && job.printer !== defaultPrinter">{{ job.printer }}</span>
                </div>
                <div class="print-job-progress" :class="{ 'unknown': job.totalPages <= 0 }">
                    <span :style="{ width: `${printJobProgress(job)}%` }"></span>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import type { PrintJob } from './types';

defineProps<{
    jobs: PrintJob[];
    defaultPrinter: string;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

const printJobProgress = (job: PrintJob) => {
    if (job.totalPages <= 0) return 0;
    return Math.min(100, Math.round((job.pagesPrinted / job.totalPages) * 100));
};
</script>

<style scoped>
/* 打印队列展开态 */
.print-queue-detail {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 5px 30px 5px 8px;
    gap: 4px;
    box-sizing: border-box;
}

.print-queue-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
}

.print-queue-title {
    display: flex;
    align-items: baseline;
    min-width: 0;
    gap: 6px;
    color: #c4b5fd;
    font-size: 11px;
    font-weight: 700;
}

.print-queue-title small {
    min-width: 0;
    overflow: hidden;
    color: rgba(255, 255, 255, 0.55);
    font-size: 9px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.print-queue-close {
    position: absolute;
    top: 50%;
    right: 6px;
    display: flex;
    width: 22px;
    height: 22px;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: #888;
    cursor: pointer;
    transform: translateY(-50%);
}

.print-queue-close:hover {
    background: rgba(139, 92, 246, 0.2);
    color: #c4b5fd;
}

.print-queue-close svg {
    width: 14px;
    height: 14px;
}

.print-job-list {
    display: flex;
    max-height: 76px;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    padding-right: 2px;
}

.print-job-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1px 8px;
    min-width: 0;
}

.print-job-main,
.print-job-meta {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
}

.print-job-document {
    overflow: hidden;
    font-size: 10px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.print-job-status,
.print-job-meta {
    color: rgba(255, 255, 255, 0.58);
    font-size: 9px;
}

.print-job-status {
    color: #c4b5fd;
    white-space: nowrap;
}

.print-job-meta {
    justify-content: flex-end;
    white-space: nowrap;
}

.print-job-progress {
    grid-column: 1 / -1;
    height: 3px;
    overflow: hidden;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.12);
}

.print-job-progress span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: #8b5cf6;
    transition: width 0.25s ease;
}

.print-job-progress.unknown span {
    width: 35% !important;
    background: rgba(139, 92, 246, 0.6);
}
</style>
