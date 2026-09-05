/**
 * 灵动岛子组件共享类型（从 WidgetIsland.vue 拆分时引入，供父组件与子组件共用）
 */

/** 打印队列单条作业（后端 print-queue-tick 事件载荷字段） */
export type PrintJob = {
    jobId: number;
    document: string;
    printer: string;
    pagesPrinted: number;
    totalPages: number;
    position: number;
    status: string;
    submitted: number;
};

/** 打印队列整体状态（print-queue-tick / get_printer_state 返回结构） */
export type PrintQueueState = {
    hasJobs: boolean;
    defaultPrinter: string;
    jobs: PrintJob[];
};

/** 实时活动轮换候选条目（RT_IDS + RT_META 组合结果，供 IslandRtChip 展示） */
export type RtActivity = {
    id: string;
    priority: number;
    icon: string;
    accent: string;
};

/** 日程同步单条日程（calendar-tick / calendar_get_state 载荷字段，阶段 F） */
export type CalendarEventInfo = {
    title: string;
    start_secs: number;
    end_secs: number;
    all_day: boolean;
    source: 'system' | 'manual';
};

/** 手动提醒条目（calendar-tick / calendar_get_state 的 manual 字段，供控制台卡片增删） */
export type ManualCalendarEvent = {
    id: number;
    title: string;
    start_secs: number;
    duration_mins: number;
    repeat_daily: boolean;
};
