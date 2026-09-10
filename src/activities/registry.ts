/**
 * 活动注册表（阶段 G）：灵动岛实时活动 + 控制台活动卡片的单一数据源。
 *
 * 设计目标：新增一个活动只需
 *   1. 在 RT_ACTIVITY_DEFS 追加一个定义（元数据 + 行为谓词 + 组件视图）；
 *   2. 提供对应的岛上展开面板/芯片组件与控制台卡片 body；
 * 不再需要改动 WidgetIsland.vue 的候选集（RT_IDS）、图标表（RT_META）与
 * clickRtChip 展开分支、右侧 v-else-if 面板链、collapseAllExpandedActivities，
 * 也不需要改动 useRealtimeActivity 的展示守卫。
 *
 * 契约说明（岛上消费侧的统一事件投影）：
 *   - RT_IDS：参与岛上多活动并行轮换的活动 id，由 realtime=true 的条目按声明顺序派生，
 *     同时作为 priority 平局时的稳定排序键与控制台卡片顺序；
 *   - panelRank：右侧展开面板的命中顺序，复刻旧模板 v-else-if 链
 *     （countdown → pomodoro → health → hardware → printer），与 RT_IDS 顺序无关；
 *   - isActive / textSources / chip / panel 均为只读取 ctx 中 ref 的纯函数，
 *     由主组件的计算属性在渲染期调用；
 *   - expand / collapse 通过 ctx.actions 间接驱动主组件的展开态与尺寸动画；
 *     currentRtIndex 推进与 expandedRtId 标记仍由主组件 clickRtChip 统一处理；
 *   - 各活动后端 tick 事件（pomodoro-tick / countdown-tick / print-queue-tick …）
 *     的监听与状态落 ref 仍留在主组件 onMounted：后端载荷面向控制台富卡片设计
 *     （total_cycles / can_skip / jobs 等字段无法无损映射进单一形状），
 *     注册表消费的是其投影（active / 文本态 / 面板数据），见下方 ctx 定义。
 */
import type { Component, ComputedRef, Ref } from 'vue';
import IslandCalendarPanel from '../components/island/IslandCalendarPanel.vue';
import IslandCdControls from '../components/island/IslandCdControls.vue';
import IslandCloseButton from '../components/island/IslandCloseButton.vue';
import IslandHwDetail from '../components/island/IslandHwDetail.vue';
import IslandHwChipRing from '../components/island/IslandHwChipRing.vue';
import IslandProgressRing from '../components/island/IslandProgressRing.vue';
import IslandPrintQueue from '../components/island/IslandPrintQueue.vue';
import IslandTaskbarProgress from '../components/island/IslandTaskbarProgress.vue';
import IslandWeatherLightAlert from '../components/island/IslandWeatherLightAlert.vue';
import IslandWeatherChip from '../components/island/IslandWeatherChip.vue';
import type { CalendarEventInfo, CalendarReminder, PrintJob } from '../components/island/types';
import { hwMetricPctOf, hwModeSlots, type HwMetric } from '../utils/hwMetrics';

/** 参与岛上多活动并行轮换的实时活动 id */
export type RtId = 'pomodoro' | 'countdown' | 'hardware' | 'health' | 'printer' | 'calendar' | 'taskbar-progress' | 'weather';

/** 控制台活动卡片 id：实时活动 + 仅控制台的 sysmsg（无岛上形态） */
export type ActivityId = RtId | 'sysmsg';

/**
 * 守卫上下文：useRealtimeActivity 持有的状态，守卫谓词的取数源。
 * 注册表只读消费；展开态的写入经由 IslandActivityCtx.actions。
 */
export interface ActivityGuardCtx {
    isPomodoroVisible: Ref<boolean>;
    isPomodoroExpanded: Ref<boolean>;
    isCountdownVisible: Ref<boolean>;
    isCountdownExpanded: Ref<boolean>;
    hwEnabled: Ref<boolean>;
    isHardwareExpanded: Ref<boolean>;
    isHealthAlerting: Ref<boolean>;
    isTaskbarProgressActive: Ref<boolean>;
    isTaskbarProgressExpanded: Ref<boolean>;
    taskbarProgressAppName: Ref<string>;
    taskbarProgressPercent: Ref<number>;
    isWeatherLightAlerting: Ref<boolean>;
    weatherLightAlert: Ref<{
        alertId: string;
        level: string;
        levelText: string;
        type: string;
        title: string;
    } | null>;
}

/** 岛上动作集合：主组件以闭包晚绑定注入（调用点均在交互期，晚于 setup 声明顺序） */
export interface IslandActivityActions {
    /** 展开态统一尺寸动画：宽度恢复/不低于最小展开宽度，仅按需调整高度（番茄钟/倒计时展开共用） */
    animateExpandSize: () => void;
    setPomodoroExpanded: (expanded: boolean) => void;
    setCountdownExpanded: (expanded: boolean) => void;
    expandHardware: () => void;
    collapseHardware: () => void;
    expandPrintQueue: () => void;
    /** restore 缺省 true：面板关闭时还原岛尺寸；候选切换等内部路径传 false 跳过还原 */
    collapsePrintQueue: (restore?: boolean) => void;
    expandCalendar: () => void;
    collapseCalendar: () => void;
    expandTaskbarProgress: () => void;
    collapseTaskbarProgress: () => void;
    toggleCountdownPauseResume: () => void;
    closeCountdownPanel: () => void;
    closePomodoroPanel: () => void;
    dismissHealthAlert: () => void;
    dismissWeatherLightAlert: () => void;
    expandWeatherLightAlert: () => void;
    collapseWeatherLightAlert: () => void;
}

/** 岛上下文：守卫上下文 + 芯片/面板渲染所需的状态与动作 */
export interface IslandActivityCtx extends ActivityGuardCtx {
    cdPaused: Ref<boolean>;
    /** 番茄钟进度圆环（剩余进度 + 按专注/休息切换的主题色），供文本态图标与芯片共用 */
    pomodoroRingPct: ComputedRef<number>;
    pomodoroRingColor: ComputedRef<string>;
    /** 倒计时进度圆环（剩余进度 + 活动主题色） */
    countdownRingPct: ComputedRef<number>;
    countdownRingColor: ComputedRef<string>;
    hwMode: Ref<string>;
    hwDefaultMetric: Ref<HwMetric>;
    hwCpuPct: Ref<number>;
    hwMemPct: Ref<number>;
    hwRingOuter: Ref<HwMetric>;
    hwRingInner: Ref<HwMetric>;
    hwBatteryPct: Ref<number>;
    hwDiskPct: Ref<number>;
    hwRingPct: ComputedRef<number>;
    hwRingColor: ComputedRef<string>;
    printJobs: Ref<PrintJob[]>;
    defaultPrinter: Ref<string>;
    isPrintQueueExpanded: Ref<boolean>;
    /** F 日程同步：未来 24h 内的日程列表（calendar-tick 驱动，系统日历 + 手动提醒合并） */
    calUpcoming: Ref<CalendarEventInfo[]>;
    /** F 日程同步：待处理的到点提醒（calendar-reminder 驱动，事件开始后清除）；有值时日历活动保持活跃 */
    calReminder: Ref<CalendarReminder | null>;
    isCalendarExpanded: Ref<boolean>;
    /** 任务栏/下载进度圆环（已完成进度 0→100% + 绿色主题），供芯片复用 IslandProgressRing */
    taskbarProgressRingPct: ComputedRef<number>;
    taskbarProgressRingColor: ComputedRef<string>;
    /** 天气轻提示展开态：仅展开后才渲染文字面板（未展开时只显示预警小图标） */
    isWeatherLightAlertExpanded: Ref<boolean>;
    actions: IslandActivityActions;
}

/** 左胶囊文本态取数源：visible 为活跃/启用 ref，expanded 为该活动展开 ref */
export interface TextSources {
    visible: Ref<boolean>;
    expanded: Ref<boolean>;
}

/** 右侧展开面板视图：key 保持与旧模板一致，复用 out-in 过渡 */
export interface PanelView {
    key: string;
    component: Component;
    props: Record<string, unknown>;
    events: Record<string, (...args: any[]) => void>;
}

/** 芯片预览内容：缺省为静态 SVG 图标（stroke=currentColor 随 accent 着色），可替换为动态组件 */
export type ChipContent =
    | { kind: 'icon'; icon: string }
    | { kind: 'component'; component: Component; props: Record<string, unknown> };

export interface RtActivityDef {
    /** 唯一 id：持久化优先级 map 与芯片轮换键 */
    id: ActivityId;
    /** 控制台卡片标题 */
    title: string;
    /** 控制台卡片描述 */
    desc: string;
    /** SVG 字符串（stroke=currentColor，随 accent 着色），供控制台卡片与芯片缺省形态共用 */
    icon: string;
    accent: string;
    /** 默认优先级（数字越小越先展示；用户持久化值优先） */
    defaultPriority: number;
    /** 是否参与岛上多活动并行轮换（false = 仅控制台卡片，如 sysmsg） */
    realtime: boolean;
    /** 芯片候选谓词：岛上当前是否处于"活跃"态 */
    isActive: (ctx: IslandActivityCtx) => boolean;
    /** 左胶囊文本态取数源；无文本态的活动（health 独占态 / printer 详情面板）缺省 */
    textSources?: (ctx: ActivityGuardCtx) => TextSources | null;
    /** 芯片自定义内容；缺省渲染 def.icon */
    chip?: (ctx: IslandActivityCtx) => ChipContent;
    /** 右侧展开面板解析；返回 null 表示当前不渲染 */
    panel?: (ctx: IslandActivityCtx) => PanelView | null;
    /** 右侧面板命中顺序（复刻旧 v-else-if 链，与 RT_IDS 顺序无关） */
    panelRank: number;
    /** 芯片点击展开动作（在主组件标记 expandedRtId 之后调用） */
    expand?: (ctx: IslandActivityCtx) => void;
    /** 候选切换前统一折叠动作；health 由事件驱动不参与 */
    collapse?: (ctx: IslandActivityCtx) => void;
    /** 强优先级: 活动时强占轮换第 1 位, 非活动时不出现在轮换队列(默认 false) */
    forceWhenActive?: boolean;
}

/**
 * 活动注册表。声明顺序同时决定：
 *   - 控制台活动卡片的排布顺序；
 *   - RT_IDS 派生顺序（过滤 realtime=false 的 sysmsg 后与旧 RT_IDS 完全一致）。
 */
export const RT_ACTIVITY_DEFS: RtActivityDef[] = [
    {
        id: 'pomodoro',
        title: '专注番茄钟',
        desc: '沉浸工作时间管理',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>',
        accent: '#ff4757',
        defaultPriority: 1,
        realtime: true,
        isActive: ctx => ctx.isPomodoroVisible.value,
        textSources: ctx => ({ visible: ctx.isPomodoroVisible, expanded: ctx.isPomodoroExpanded }),
        // 芯片形态：动态进度圆环（剩余进度随时间递减，颜色随专注/休息阶段切换）
        chip: ctx => ({
            kind: 'component',
            component: IslandProgressRing,
            props: {
                pct: ctx.pomodoroRingPct.value,
                color: ctx.pomodoroRingColor.value,
            },
        }),
        panel: ctx => {
            if (!ctx.isPomodoroExpanded.value) return null;
            return {
                key: 'close-btn',
                component: IslandCloseButton,
                props: {},
                events: { close: () => ctx.actions.closePomodoroPanel() },
            };
        },
        panelRank: 2,
        expand: ctx => {
            ctx.actions.setPomodoroExpanded(true);
            ctx.actions.animateExpandSize();
        },
        collapse: ctx => ctx.actions.setPomodoroExpanded(false),
    },
    {
        id: 'countdown',
        title: '快捷倒计时',
        desc: '自定义时长倒计时',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>',
        accent: '#ff9800',
        defaultPriority: 2,
        realtime: true,
        isActive: ctx => ctx.isCountdownVisible.value,
        textSources: ctx => ({ visible: ctx.isCountdownVisible, expanded: ctx.isCountdownExpanded }),
        // 芯片形态：动态进度圆环（剩余进度随时间递减，固定倒计时主题色）
        chip: ctx => ({
            kind: 'component',
            component: IslandProgressRing,
            props: {
                pct: ctx.countdownRingPct.value,
                color: ctx.countdownRingColor.value,
            },
        }),
        panel: ctx => {
            if (!ctx.isCountdownExpanded.value) return null;
            return {
                key: 'cd-controls',
                component: IslandCdControls,
                props: { cdPaused: ctx.cdPaused.value },
                events: {
                    toggle: () => ctx.actions.toggleCountdownPauseResume(),
                    close: () => ctx.actions.closeCountdownPanel(),
                },
            };
        },
        panelRank: 1,
        expand: ctx => {
            ctx.actions.setCountdownExpanded(true);
            ctx.actions.animateExpandSize();
        },
        collapse: ctx => ctx.actions.setCountdownExpanded(false),
    },
    {
        id: 'hardware',
        title: '硬件监控',
        desc: '实时监测处理器与内存',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2" /><rect x="9" y="9" width="6" height="6" /><line x1="9" y1="1" x2="9" y2="4" /><line x1="15" y1="1" x2="15" y2="4" /><line x1="9" y1="20" x2="9" y2="23" /><line x1="15" y1="20" x2="15" y2="23" /><line x1="20" y1="9" x2="23" y2="9" /><line x1="20" y1="14" x2="23" y2="14" /><line x1="1" y1="9" x2="4" y2="9" /><line x1="1" y1="14" x2="4" y2="14" /></svg>',
        accent: '#3b82f6',
        defaultPriority: 3,
        realtime: true,
        // 硬件"活跃" = 监控开启（环在主岛显示时芯片并存，点击环或芯片均可展开详情）
        isActive: ctx => ctx.hwEnabled.value,
        textSources: ctx => ({ visible: ctx.hwEnabled, expanded: ctx.isHardwareExpanded }),
        chip: ctx => ({
            kind: 'component',
            component: IslandHwChipRing,
            props: {
                hwMode: ctx.hwMode.value,
                hwCpuPct: ctx.hwCpuPct.value,
                hwMemPct: ctx.hwMemPct.value,
                hwRingPct: ctx.hwRingPct.value,
                hwRingColor: ctx.hwRingColor.value,
                hwRingOuter: ctx.hwRingOuter.value,
                hwRingInner: ctx.hwRingInner.value,
                hwBatteryPct: ctx.hwBatteryPct.value,
                hwDiskPct: ctx.hwDiskPct.value,
            },
        }),
        panel: ctx => {
            if (!ctx.isHardwareExpanded.value || !ctx.hwEnabled.value) return null;
            // E：详情行跟随当前模式的圆环槽位（dual = 外/内环；rotation = CPU/内存；single = 选中指标）
            const vals = {
                cpu: ctx.hwCpuPct.value,
                mem: ctx.hwMemPct.value,
                battery: ctx.hwBatteryPct.value,
                disk: ctx.hwDiskPct.value,
            };
            const [outer, inner] = hwModeSlots(
                ctx.hwMode.value,
                ctx.hwRingOuter.value,
                ctx.hwRingInner.value,
                ctx.hwDefaultMetric.value,
            );
            const slots = [outer, inner]
                .filter((metric): metric is HwMetric => metric !== null)
                .map(metric => ({ metric, pct: hwMetricPctOf(metric, vals) }));
            return {
                key: 'hw-detail',
                component: IslandHwDetail,
                props: { slots },
                events: { close: () => ctx.actions.collapseHardware() },
            };
        },
        panelRank: 4,
        expand: ctx => ctx.actions.expandHardware(),
        collapse: ctx => ctx.actions.collapseHardware(),
    },
    {
        id: 'health',
        title: '健康提醒',
        desc: '久坐与喝水提醒',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"></path></svg>',
        accent: '#10b981',
        defaultPriority: 4,
        realtime: true,
        // 健康提醒"活跃" = alerting（独占岛态由 health-reminder-tick 事件驱动，无手动展开）
        isActive: ctx => ctx.isHealthAlerting.value,
        panel: ctx => {
            if (!ctx.isHealthAlerting.value) return null;
            return {
                key: 'health-close',
                component: IslandCloseButton,
                props: {},
                events: { close: () => ctx.actions.dismissHealthAlert() },
            };
        },
        panelRank: 3,
    },
    {
        id: 'sysmsg',
        title: '系统动态感知',
        desc: '实时捕捉软硬件生态变化',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>',
        accent: '#ff4757',
        defaultPriority: 99,
        // 仅控制台卡片：总开关 = 任一分类开启，enabled 由 LiveActive 注入，不参与岛上轮换
        realtime: false,
        isActive: () => false,
        panelRank: 99,
    },
    {
        id: 'printer',
        title: '打印机队列',
        desc: '批量打印进度状态',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>',
        accent: '#8b5cf6',
        defaultPriority: 5,
        realtime: true,
        isActive: ctx => ctx.printJobs.value.length > 0,
        panel: ctx => {
            if (!ctx.isPrintQueueExpanded.value) return null;
            return {
                key: 'print-queue',
                component: IslandPrintQueue,
                props: { jobs: ctx.printJobs.value, defaultPrinter: ctx.defaultPrinter.value },
                events: { close: () => ctx.actions.collapsePrintQueue() },
            };
        },
        panelRank: 5,
        expand: ctx => ctx.actions.expandPrintQueue(),
        collapse: ctx => ctx.actions.collapsePrintQueue(false),
    },
    {
        id: 'calendar',
        title: '日程同步',
        desc: '系统日历与手动提醒',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect><line x1="16" y1="2" x2="16" y2="6"></line><line x1="8" y1="2" x2="8" y2="6"></line><line x1="3" y1="10" x2="21" y2="10"></line></svg>',
        accent: '#06b6d4',
        defaultPriority: 6,
        realtime: true,
        // F：日程"活跃" = 未来 24h 内存在日程（系统日历 + 手动提醒，calendar-tick 驱动），
        // 或存在待处理的到点提醒（提醒 toast 一闪而过后，小图标上的日历入口要留住，
        // 否则事件刚开始、24h 列表刚剔除它的瞬间就没了入口）
        isActive: ctx => ctx.calUpcoming.value.length > 0 || !!ctx.calReminder.value,
        panel: ctx => {
            if (!ctx.isCalendarExpanded.value || (ctx.calUpcoming.value.length === 0 && !ctx.calReminder.value)) return null;
            return {
                key: 'calendar-panel',
                component: IslandCalendarPanel,
                props: { events: ctx.calUpcoming.value, reminder: ctx.calReminder.value },
                events: { close: () => ctx.actions.collapseCalendar() },
            };
        },
        panelRank: 6,
        expand: ctx => ctx.actions.expandCalendar(),
        collapse: ctx => ctx.actions.collapseCalendar(),
    },
    {
        id: 'taskbar-progress',
        title: '任务栏进度',
        desc: '聚合浏览器下载、文件复制、压缩/解压等任务栏进度',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="2" x2="12" y2="6"></line><line x1="12" y1="18" x2="12" y2="22"></line><line x1="4.93" y1="4.93" x2="7.76" y2="7.76"></line><line x1="16.24" y1="16.24" x2="19.07" y2="19.07"></line><line x1="2" y1="12" x2="6" y2="12"></line><line x1="18" y1="12" x2="22" y2="12"></line><line x1="4.93" y1="19.07" x2="7.76" y2="16.24"></line><line x1="16.24" y1="7.76" x2="19.07" y2="4.93"></line></svg>',
        accent: '#22c55e',
        defaultPriority: 99,
        realtime: true,
        /** 特殊优先级: 活动时强占 chip 第 1 位, 非活动时不出现在轮换队列 */
        forceWhenActive: true,
        isActive: ctx => ctx.isTaskbarProgressActive.value,
        // 芯片形态：动态进度圆环（已完成进度 0→100% 递增），与倒计时/番茄钟同一套视觉
        chip: ctx => ({
            kind: 'component',
            component: IslandProgressRing,
            props: {
                pct: ctx.taskbarProgressRingPct.value,
                color: ctx.taskbarProgressRingColor.value,
            },
        }),
        panel: ctx => {
            if (!ctx.isTaskbarProgressExpanded.value) return null;
            return {
                key: 'taskbar-progress-detail',
                component: IslandTaskbarProgress,
                props: {
                    appName: ctx.taskbarProgressAppName.value,
                    percent: ctx.taskbarProgressPercent.value,
                },
                events: { close: () => ctx.actions.collapseTaskbarProgress() },
            };
        },
        panelRank: 7,
        expand: ctx => ctx.actions.expandTaskbarProgress(),
        collapse: ctx => ctx.actions.collapseTaskbarProgress(),
    },
    {
        id: 'weather',
        title: '恶劣天气提醒',
        desc: '小米天气·小时级·早午晚报',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>',
        accent: '#0ea5e9',
        defaultPriority: 7,
        // 轻提示态临时进入轮换，强占第 1 位 5s。
        // 必须 realtime=true 才能进 RT_IDS 获得芯片；此前为 false → 永远没有小图标，
        // 而 panel 又在 alerting 时立刻渲染，导致"直接占岛显示文字"的坏观感。
        realtime: true,
        forceWhenActive: true,
        isActive: ctx => ctx.isWeatherLightAlerting.value,
        // 芯片形态：按预警等级着色的三角小图标（未展开时只显示图标，不占文字空间）
        chip: ctx => ({
            kind: 'component',
            component: IslandWeatherChip,
            props: {
                level: ctx.weatherLightAlert.value?.level ?? 'B',
                type: ctx.weatherLightAlert.value?.type ?? '',
            },
        }),
        // 仅点击展开后才渲染文字面板
        panel: ctx => {
            if (!ctx.isWeatherLightAlerting.value || !ctx.isWeatherLightAlertExpanded.value) return null;
            return {
                key: 'weather-light-alert',
                component: IslandWeatherLightAlert,
                props: { alert: ctx.weatherLightAlert.value },
                events: { close: () => ctx.actions.dismissWeatherLightAlert() },
            };
        },
        panelRank: 8,
        expand: ctx => ctx.actions.expandWeatherLightAlert(),
        collapse: ctx => ctx.actions.collapseWeatherLightAlert(),
    },
];

/** 参与岛上轮换的活动 id（顺序固定，作为 priority 平局时的稳定排序键）。
 * realtime=true ⟺ id 为 RtId（sysmsg 是唯一的 realtime=false 条目），由断言固化该不变式 */
export const RT_IDS: readonly RtId[] = RT_ACTIVITY_DEFS
    .filter((def): def is RtActivityDef & { id: RtId } => def.realtime)
    .map(def => def.id);

/** 右侧展开面板按命中顺序排列（panelRank 升序），供主组件泛型渲染 */
export const PANEL_DEFS_BY_RANK = RT_ACTIVITY_DEFS
    .filter((def): def is RtActivityDef & { panel: NonNullable<RtActivityDef['panel']> } =>
        typeof def.panel === 'function')
    .sort((a, b) => a.panelRank - b.panelRank);

/** 按 id 取活动定义；芯片候选与面板均源自注册表，未知 id 属于调用方错误 */
export function getRtDef(id: string): RtActivityDef {
    const def = RT_ACTIVITY_DEFS.find(d => d.id === id);
    if (!def) throw new Error(`[activities/registry] 未知活动 id: ${id}`);
    return def;
}
