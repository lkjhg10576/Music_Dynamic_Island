/**
 * 日程同步展示工具（阶段 F）：控制台卡片与岛上展开面板共用的文案格式化。
 * 后端 calendar-tick 推送的是 unix 秒，本地时分 / 剩余时长统一在此换算。
 */
import {
    measureTextWidth,
    FONT_CAL_ROW_META,
    FONT_CAL_ROW_TITLE,
    FONT_PANEL_SUB,
    FONT_PANEL_TITLE,
} from './textMeasure';

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

/** 到点提醒条的倒计时文案（无 end_secs，只判是否已开始） */
export const formatReminderCountdown = (startSecs: number, nowSecs: number): string => {
    if (startSecs <= nowSecs) return '已开始';
    return `还有 ${formatEventRemaining(startSecs - nowSecs)}`;
};

// ──────────────────────────────────────────────
// 岛上日程面板的自适应尺寸
// 宽度按"最长一行"实测像素决定（此前固定用 getExpandTargetWidth()，长标题会被省略号截断），
// 高度按行数增长（仿 expandPrintQueue：只给 42px 岛高时列表可视区仅约一行）。
// 下面的字体串 / 结构常量必须与 IslandCalendarPanel.vue 的 CSS 一一对应。
// ──────────────────────────────────────────────

/** 面板左右内边距（.calendar-detail padding: 5px 30px 5px 8px） */
export const CAL_PANEL_PADDING_X = 38;
/** 行内固定占位：行左右 padding 12 + 标题与 meta 间距 5 + meta 与来源徽标间距 8 + 徽标约 24 */
export const CAL_PANEL_ROW_CHROME = 49;
/** 表头 + 提醒条以外的固定高度预算（表头行 + 与列表的间距） */
export const CAL_PANEL_HEAD_H = 20;
/** 面板上下内边距（.calendar-detail padding 5px 0） */
export const CAL_PANEL_VPAD = 10;
/** 单行高度（10px 标题 + 行内上下 padding + 行间距） */
export const CAL_PANEL_ROW_H = 22;
/** 面板最多展示的行数（更多在列表内滚动） */
export const CAL_PANEL_MAX_ROWS = 3;

/** 面板行取数用的结构化入参（CalendarEventInfo 结构兼容，避免 utils 反向依赖 components） */
export type CalendarPanelRow = {
    title: string;
    start_secs: number;
    end_secs: number;
    all_day: boolean;
    source: string;
};

type CalendarPanelReminder = { title: string; start_secs: number } | null | undefined;

/** 面板目标行数：日程条数（封顶）+ 提醒条（有则额外占一行） */
export const calendarPanelRowCount = (events: CalendarPanelRow[], reminder: CalendarPanelReminder): number =>
    Math.min(events.length, CAL_PANEL_MAX_ROWS) + (reminder ? 1 : 0);

/**
 * 日程面板目标宽度：取「表头 / 提醒条 / 每行（标题 + 时间·倒计时 + 来源徽标）」三者实测宽度的最大值
 * 加上左右内边距，再 clamp 到 [minWidth, maxWidth]。文本变化（新日程、切换提醒）时重新计算即可。
 */
export const measureCalendarPanelWidth = (
    events: CalendarPanelRow[],
    reminder: CalendarPanelReminder,
    nowSecs: number,
    minWidth: number,
    maxWidth: number,
): number => {
    // 表头：「日程」+「未来 24 小时 · N 项」（.calendar-title 与 small，gap 6）
    let widest = measureTextWidth('日程', FONT_PANEL_TITLE)
        + 6
        + measureTextWidth(`未来 24 小时 · ${events.length} 项`, FONT_PANEL_SUB);

    // 提醒条：图标 14 + gap 6 + 事件标题 + gap 8 + 「还有 X 分钟 / 已开始」
    if (reminder) {
        const reminderW = 14 + 6
            + measureTextWidth(reminder.title, FONT_CAL_ROW_TITLE)
            + 8
            + measureTextWidth(formatReminderCountdown(reminder.start_secs, nowSecs), FONT_CAL_ROW_META);
        widest = Math.max(widest, reminderW);
    }

    // 日程行：标题 + 「HH:MM · 还有 X」/「全天」+ 来源徽标
    for (const ev of events) {
        const meta = ev.all_day
            ? formatEventCountdown(ev, nowSecs)
            : `${formatEventHhmm(ev.start_secs)} · ${formatEventCountdown(ev, nowSecs)}`;
        const rowW = measureTextWidth(ev.title, FONT_CAL_ROW_TITLE)
            + CAL_PANEL_ROW_CHROME
            + measureTextWidth(meta, FONT_CAL_ROW_META);
        widest = Math.max(widest, rowW);
    }

    const target = widest + CAL_PANEL_PADDING_X;
    return Math.max(minWidth, Math.min(maxWidth, Math.ceil(target)));
};
