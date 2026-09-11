/**
 * 文本像素宽度测量：灵动岛自适应尺寸（系统 toast 岛宽、展开面板宽/高）共用。
 *
 * 此前测量逻辑私有在 useNotifications 内（只服务 toast 岛宽），日程面板等
 * 需要「按内容自适应宽度」的场景无法复用；此处抽出为单一实现，调用方通过
 * FONT_* 常量传字体串，保证测量口径与组件 CSS 一致（不一致会导致岛宽偏大/偏小）。
 *
 * 实现：canvas measureText 优先（与渲染引擎同一套字形度量）；无 canvas
 * （极端环境 / 测量异常）时逐字回退，按字号折算中文全宽、其余字符 0.6 宽。
 */

let measureCanvas: HTMLCanvasElement | null = null;
let measureCtx: CanvasRenderingContext2D | null = null;

/** 取共享的测量 canvas 上下文（惰性创建；创建失败则返回 null，走逐字回退） */
const getMeasureCtx = (): CanvasRenderingContext2D | null => {
    if (measureCtx) return measureCtx;
    try {
        measureCanvas = document.createElement('canvas');
        measureCtx = measureCanvas.getContext('2d');
    } catch (_e) {
        measureCtx = null;
    }
    return measureCtx;
};

/** 从字体串里取像素字号（用于逐字回退折算；解析失败按 12.5 兜底） */
const fontSizeOf = (font: string): number => {
    const m = /(\d+(?:\.\d+)?)px/.exec(font);
    const size = m ? Number(m[1]) : NaN;
    return Number.isFinite(size) && size > 0 ? size : 12.5;
};

/**
 * 测量文本像素宽度（向上取整）。
 * @param text 待测文本
 * @param font CSS font 简写（如 '700 11px -apple-system, ...'），必须与渲染端一致
 */
export function measureTextWidth(text: string, font: string): number {
    if (!text) return 0;
    const ctx = getMeasureCtx();
    if (ctx) {
        try {
            ctx.font = font;
            const width = ctx.measureText(text).width;
            if (Number.isFinite(width) && width > 0) return Math.ceil(width);
        } catch (_e) { /* 落到逐字回退 */ }
    }
    // 回退：中文（含常用全角标点）按字号整宽，其余按 0.6 字号
    const size = fontSizeOf(font);
    let w = 0;
    for (const ch of text) {
        w += /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/.test(ch) ? size : size * 0.6;
    }
    return Math.ceil(w);
}

// ──────────────────────────────────────────────
// 岛上文案的字体串（与各组件 CSS 的 font-size/font-weight 一一对应）
// ──────────────────────────────────────────────

/** 系统 toast 单行正文（IslandWeatherAlert .toast-text / .toast-title） */
export const FONT_TOAST_TITLE = '600 12.5px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 系统 toast 第二行小字（IslandWeatherAlert .toast-body） */
export const FONT_TOAST_BODY = '500 10px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 展开态通知（岛高 65：消息通知 / 天气速报·预警详情）的标题行（IslandMsg .msg-title） */
export const FONT_MSG_TITLE = '700 14px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 展开态通知的正文行（IslandMsg .msg-body） */
export const FONT_MSG_BODY = '400 12.5px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 任务栏进度面板标题行（IslandTaskbarProgress .taskbar-progress-title） */
export const FONT_PANEL_TITLE = '700 11px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 任务栏进度面板百分比小字（.taskbar-progress-title small） */
export const FONT_PANEL_SUB = '500 9px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 日程面板行标题（IslandCalendarPanel .calendar-event-title） */
export const FONT_CAL_ROW_TITLE = '600 10px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';

/** 日程面板行内时间/倒计时（.calendar-event-meta） */
export const FONT_CAL_ROW_META = '400 9px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
