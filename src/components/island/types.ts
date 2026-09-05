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
