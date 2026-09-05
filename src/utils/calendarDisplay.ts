/**
 * 日程同步展示工具（阶段 F）：控制台卡片与岛上展开面板共用的文案格式化。
 * 后端 calendar-tick 推送的是 unix 秒，本地时分 / 剩余时长统一在此换算。
 */

/** unix 秒 → 本地 HH:MM */
export const formatEventHhmm = (startSecs: number): string => {
    const d = new Date(startSecs * 1000);
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

/** 秒数 → "X 秒 / X 分钟 / X 小时 X 分"（分钟级精度的剩余时长文案） */
export const formatEventRemaining = (secs: number): string => {
    const s = Math.max(0, Math.round(secs));
    if (s < 60) return `${s} 秒`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m} 分钟`;
    return `${Math.floor(m / 60)} 小时 ${m % 60} 分`;
};

/** 单条日程的"还有多久"文案：未开始→还有 X；进行中→进行中；全天→全天 */
export const formatEventCountdown = (ev: { start_secs: number; end_secs: number; all_day: boolean }, nowSecs: number): string => {
    if (ev.all_day) return '全天';
    if (ev.start_secs <= nowSecs) return '进行中';
    return `还有 ${formatEventRemaining(ev.start_secs - nowSecs)}`;
};
