<template>
    <div class="calendar-detail">
        <div class="calendar-head">
            <div class="calendar-title">
                <span>日程</span>
                <small>未来 24 小时 · {{ events.length }} 项</small>
            </div>
            <button class="calendar-close" type="button" title="关闭日程" @click.stop="emit('close')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>
        <div class="calendar-list">
            <!-- 到点提醒条：后端 calendar-reminder 触发后保留到事件开始，
                 保证「提醒 toast 一闪而过」之后仍能从实时活动小图标点进来查看 -->
            <div v-if="reminder" class="calendar-reminder">
                <svg class="calendar-reminder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round">
                    <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path>
                    <path d="M13.73 21a2 2 0 0 1-3.46 0"></path>
                </svg>
                <span class="calendar-reminder-title" :title="reminder.title">{{ reminder.title }}</span>
                <span class="calendar-reminder-meta">{{ reminderCountdown }}</span>
            </div>
            <div v-for="row in rows" :key="row.key" class="calendar-row" :class="{ 'is-next': row.isNext }">
                <div class="calendar-main">
                    <span class="calendar-event-title" :title="row.event.title">{{ row.event.title }}</span>
                    <span class="calendar-event-meta">
                        <template v-if="!row.event.all_day">{{ row.hhmm }} · </template>{{ row.countdown }}
                    </span>
                </div>
                <span class="calendar-source" :class="row.event.source">{{ row.event.source === 'manual' ? '手动' : '日历' }}</span>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { CalendarEventInfo, CalendarReminder } from './types';
import { formatEventCountdown, formatEventHhmm, formatReminderCountdown } from '../../utils/calendarDisplay';

const props = defineProps<{
    events: CalendarEventInfo[];
    /** 待处理的到点提醒（calendar-reminder 写入，事件开始后清除）；有值时在列表上方显示高亮提醒条 */
    reminder?: CalendarReminder | null;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

// 倒计时文案随 events（每 30s tick）重算：computed 重估时重新取当前时刻，避免面板久挂时"还有 X 分钟"冻结
const nowSecsOf = () => Math.floor(Date.now() / 1000);

const rows = computed(() => {
    const nowSecs = nowSecsOf();
    return props.events.map((ev, i) => ({
        key: `${ev.start_secs}-${ev.title}-${i}`,
        event: ev,
        isNext: i === 0,
        countdown: formatEventCountdown(ev, nowSecs),
        hhmm: ev.all_day ? '' : formatEventHhmm(ev.start_secs),
    }));
});

// 提醒条倒计时与 rows 同源重算（每 30s tick 推进一次）
const reminderCountdown = computed(() =>
    props.reminder ? formatReminderCountdown(props.reminder.start_secs, nowSecsOf()) : '');
</script>

<style scoped>
/* 日程展开态（阶段 F）：布局对齐打印队列面板，青色 accent */
.calendar-detail {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 5px 30px 5px 8px;
    gap: 4px;
    box-sizing: border-box;
}

.calendar-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
}

.calendar-title {
    display: flex;
    align-items: baseline;
    min-width: 0;
    gap: 6px;
    color: #67e8f9;
    font-size: 11px;
    font-weight: 700;
}

.calendar-title small {
    min-width: 0;
    overflow: hidden;
    color: rgba(255, 255, 255, 0.55);
    font-size: 9px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.calendar-close {
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

.calendar-close:hover {
    background: rgba(6, 182, 212, 0.2);
    color: #67e8f9;
}

.calendar-close svg {
    width: 14px;
    height: 14px;
}

.calendar-list {
    display: flex;
    max-height: 76px;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    padding-right: 2px;
}

.calendar-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-width: 0;
    padding: 2px 6px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
}

.calendar-row.is-next {
    background: rgba(6, 182, 212, 0.14);
}

/* 到点提醒条：琥珀色高亮，与普通日程行区分（它是"现在要发生"的那条） */
.calendar-reminder {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 2px 6px;
    border-radius: 8px;
    background: rgba(251, 191, 36, 0.16);
    color: #fbbf24;
}

.calendar-reminder-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
}

.calendar-reminder-title {
    min-width: 0;
    overflow: hidden;
    color: #fde68a;
    font-size: 10px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.calendar-reminder-meta {
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 600;
    white-space: nowrap;
}

.calendar-main {
    display: flex;
    min-width: 0;
    align-items: baseline;
    gap: 5px;
}

.calendar-event-title {
    overflow: hidden;
    font-size: 10px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.calendar-event-meta {
    color: #67e8f9;
    font-size: 9px;
    white-space: nowrap;
}

.calendar-source {
    flex-shrink: 0;
    font-size: 8px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.5);
    white-space: nowrap;
}

.calendar-source.manual {
    color: #fbbf24;
}
</style>
