/**
 * 时间进度圆环元数据：圆环几何、主题色与进度口径。
 * 供 useRealtimeActivity / 岛上图标（IslandProgressRing）共用，
 * 与 utils/hwMetrics.ts 的硬件单圆环保持同一套视觉规格，避免两处各写一份后漂移。
 */

// ──────────────────────────────────────────────
// 几何规格：与硬件监控单圆环（IslandHardwareRing / IslandHwChipRing）逐项一致
// viewBox 0 0 36 36、cx/cy=18、r=14、stroke-width=3
// ──────────────────────────────────────────────

/** 圆环半径 */
export const RING_RADIUS = 14;

/** 圆环描边宽度 */
export const RING_STROKE_WIDTH = 3;

/** 主圆环周长（2πr，r=14）；dasharray 以此为单位切分进度 */
export const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

/** 未完成部分的轨道底色（与硬件监控背景环一致） */
export const RING_TRACK_COLOR = 'rgba(255,255,255,0.1)';

/** 环内/环外尺寸（岛上左侧胶囊与右侧 chip 统一取 24px） */
export const RING_ICON_SIZE = 24;

// ──────────────────────────────────────────────
// 主题色：与活动注册表的 accent 及既有文字配色对齐
// ──────────────────────────────────────────────

/** 番茄钟专注阶段主题色（同 registry pomodoro.accent 与 .phase-focus） */
export const POMODORO_FOCUS_COLOR = '#ff4757';

/** 番茄钟休息阶段主题色（同 .phase-break 文字色） */
export const POMODORO_BREAK_COLOR = '#2196f3';

/** 倒计时主题色（同 registry countdown.accent 与 .countdown-time） */
export const COUNTDOWN_COLOR = '#ff9800';

/** 番茄钟阶段 → 圆环主题色 */
export function pomodoroRingColor(phase: 'focus' | 'break'): string {
    return phase === 'break' ? POMODORO_BREAK_COLOR : POMODORO_FOCUS_COLOR;
}

// ──────────────────────────────────────────────
// 进度口径
// ──────────────────────────────────────────────

/**
 * 剩余进度百分比（0~100）：环随时间递减，起始满环、结束空环。
 * 未知总时长（total <= 0，如首个 tick 到达前）按 0 兜底，避免 NaN dasharray。
 */
export function progressRingPct(remainingSecs: number, totalSecs: number): number {
    if (!Number.isFinite(totalSecs) || totalSecs <= 0) return 0;
    const pct = (remainingSecs / totalSecs) * 100;
    return Math.max(0, Math.min(100, pct));
}

/** 剩余进度对应的 dasharray（filled 空格 circumference） */
export function progressRingDash(pct: number, circumference = RING_CIRCUMFERENCE): string {
    const filled = (Math.max(0, Math.min(100, pct)) / 100) * circumference;
    return `${filled} ${circumference}`;
}
