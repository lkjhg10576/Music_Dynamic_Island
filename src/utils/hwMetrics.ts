/**
 * 硬件监控指标元数据（阶段 E）：可选指标、显示名、圆环配色与"显示模式 → 圆环槽位"映射。
 * 供 useRealtimeActivity / LiveActive 实时预览 / 岛上圆环组件共用，
 * 避免配色与口径在多处各写一份后漂移。
 */

/** 圆环可选指标：电池/磁盘为 E 阶段新增数据源（monitor-stats 扩展字段） */
export type HwMetric = 'cpu' | 'mem' | 'battery' | 'disk';

export const HW_METRICS: readonly HwMetric[] = ['cpu', 'mem', 'battery', 'disk'];

/** 控制台下拉/单选用的显示名 */
export const HW_METRIC_LABEL: Record<HwMetric, string> = {
    cpu: 'CPU',
    mem: '内存',
    battery: '电池',
    disk: '磁盘',
};

/** 岛上折叠态小标签（宽度受限，用短名） */
export const HW_METRIC_ISLAND_LABEL: Record<HwMetric, string> = {
    cpu: 'CPU',
    mem: 'RAM',
    battery: 'BAT',
    disk: 'DSK',
};

/** 双圆环折叠态的单字母前缀（如 C95 / B--） */
export const HW_METRIC_INITIAL: Record<HwMetric, string> = {
    cpu: 'C',
    mem: 'M',
    battery: 'B',
    disk: 'D',
};

/** 电池百分比的"无电池"哨兵值（GetSystemPowerStatus：BatteryLifePercent = 255） */
export const HW_BATTERY_UNAVAILABLE = 255;

/** 各指标实时百分比；电池为哨兵值（无电池）时返回 null，由调用方渲染置灰空环 */
export function hwMetricPctOf(
    metric: HwMetric,
    vals: { cpu: number; mem: number; battery: number; disk: number },
): number | null {
    if (metric === 'battery' && vals.battery >= HW_BATTERY_UNAVAILABLE) return null;
    return vals[metric];
}

/** 钳制到 0~100 的圆环进度（不可用时为 0，即空环） */
export function hwMetricClamp(pct: number | null): number {
    return pct === null ? 0 : Math.max(0, Math.min(100, pct));
}

/** 圆环描边色：沿用 CPU/内存既有色板（≥80% 转告警色），电池按低电量告警，不可用置灰 */
export function hwMetricColor(metric: HwMetric, pct: number, unavailable = false): string {
    if (unavailable) return 'rgba(255, 255, 255, 0.25)';
    switch (metric) {
        case 'cpu': return pct >= 80 ? '#a855f7' : '#ffffff';
        case 'mem': return pct >= 80 ? '#ff4757' : '#3b82f6';
        case 'battery': return pct <= 20 ? '#ff4757' : '#22c55e';
        case 'disk': return pct >= 80 ? '#ff4757' : '#f59e0b';
    }
}

/** 显示模式 → 圆环槽位：dual = [外环, 内环]；rotation = CPU/内存；single = [选中指标, null] */
export function hwModeSlots(
    mode: string,
    outer: HwMetric,
    inner: HwMetric,
    single: HwMetric,
): [HwMetric, HwMetric | null] {
    if (mode === 'dual') return [outer, inner];
    if (mode === 'rotation') return ['cpu', 'mem'];
    return [single, null];
}

// ──────────────────────────────────────────────
// 槽位渲染工具：以实时值集合为参数的纯函数族，
// 供岛上圆环组件（IslandHardwareRing / IslandHwChipRing）与控制台预览复用
// ──────────────────────────────────────────────

export type HwMetricVals = { cpu: number; mem: number; battery: number; disk: number };

/** 槽位描边色（不可用指标自动置灰） */
export function hwMetricSlotColor(metric: HwMetric, vals: HwMetricVals): string {
    const raw = hwMetricPctOf(metric, vals);
    return hwMetricColor(metric, hwMetricClamp(raw), raw === null);
}

/** 槽位 dasharray（circumference 为该圆的周长） */
export function hwMetricSlotDash(metric: HwMetric, vals: HwMetricVals, circumference: number): string {
    const filled = (hwMetricClamp(hwMetricPctOf(metric, vals)) / 100) * circumference;
    return `${filled} ${circumference}`;
}

/** 槽位百分比文本：正常 "42%"，不可用 "--" */
export function hwMetricSlotText(metric: HwMetric, vals: HwMetricVals): string {
    const raw = hwMetricPctOf(metric, vals);
    return raw === null ? '--' : `${Math.round(hwMetricClamp(raw))}%`;
}

/** 双圆环折叠态短文本：C95 / B-- */
export function hwMetricSlotShort(metric: HwMetric, vals: HwMetricVals): string {
    const raw = hwMetricPctOf(metric, vals);
    return `${HW_METRIC_INITIAL[metric]}${raw === null ? '--' : Math.round(hwMetricClamp(raw))}`;
}

/** 是否达到 80% 告警阈值（不可用为 false） */
export function hwMetricSlotHigh(metric: HwMetric, vals: HwMetricVals): boolean {
    const raw = hwMetricPctOf(metric, vals);
    return raw !== null && hwMetricClamp(raw) >= 80;
}
