/**
 * 实时活动 composable：番茄钟 / 倒计时 / 健康提醒 / 硬件监控附属图标 / 主岛内容轮换。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - 番茄钟/倒计时状态（由后端 pomodoro-tick / countdown-tick 事件驱动，事件监听仍在主组件 onMounted）
 *     + 启动时恢复运行状态的两段后端查询（restorePomodoroState / restoreCountdownState，由主组件 onMounted 调用）
 *   - 健康提醒（久坐/喝水）alerting 状态（由后端 health-reminder-tick 事件驱动）
 *   - 硬件监控附属图标：开关/模式/指标数据 + 轮换定时器（startHwRotation/stopHwRotation，含兜底 watch）
 *   - 主岛轮换模式：网速/音乐 5s 轮换（startRotation/stopRotation）
 *   - 展示谓词：showPomodoroText / showCountdownText / showHardwareRing / isSplitMode，
 *     供主组件 displaySpeed / displayMusic 计算属性守卫使用（接入点必须位于其之前）。
 *     守卫不再逐活动硬编码：各活动的文本态取数源由活动注册表（activities/registry.ts）
 *     的 textSources 声明，这里统一遍历生成，新增文本态活动只改注册表。
 * 后端事件监听（pomodoro-tick / countdown-tick / health-reminder-tick / monitor-stats 等）与
 * 展开交互编排（clickRtChip / expandHardware / collapseAllExpandedActivities 等）留在主组件；
 * 岛尺寸恢复动画（animateIslandSize 等）通过主组件中转，不在本域重复实现。
 */
import { ref, computed, watch, type Ref, type ComputedRef } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSettingRaw, setSettingRaw } from '../utils/settings';
import { RT_ACTIVITY_DEFS, type RtId, type ActivityGuardCtx, type TextSources } from '../activities/registry';
import {
    NSD_POMODORO_VISIBLE,
    NSD_COUNTDOWN_VISIBLE,
    NSD_HW_ENABLED,
    NSD_HW_MODE,
    NSD_HW_DEFAULT_METRIC,
    NSD_HW_RING_OUTER,
    NSD_HW_RING_INNER,
    NSD_ROTATION_MODE,
} from '../constants/storageKeys';
import { HW_BATTERY_UNAVAILABLE, HW_METRICS, hwMetricClamp, hwMetricColor, hwMetricPctOf, type HwMetric } from '../utils/hwMetrics';

export function useRealtimeActivity(deps: {
    // 跨域守卫：消息通知 / 系统 toast / 音乐展开态优先级高于实时活动展示
    isMsgActive: Ref<boolean>;
    displaySysToast: Ref<boolean>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    isMusicCtlEnabled: Ref<boolean>;
}) {
    const {
        isMsgActive, displaySysToast, isMusicExpanded, isMusicExpanding,
        isMusicCtlEnabled,
    } = deps;

    // 番茄钟相关变量（由后端 pomodoro-tick 事件驱动）
    const isPomodoroVisible = ref(false);
    const pomodoroRemainingSecs = ref(0);
    const pomodoroPhase = ref<'focus' | 'break'>('focus');
    const pomodoroRemainingCycles = ref(0);
    const isPomodoroExpanded = ref(false);

    // 倒计时相关变量（由后端 countdown-tick 事件驱动）
    const isCountdownVisible = ref(false);
    const countdownRemainingSecs = ref(0);
    const isCountdownExpanded = ref(false);
    const isCountdownFinished = ref(false);
    const cdPaused = ref(false);

    // 健康提醒相关变量（由后端 health-reminder-tick 事件驱动）
    const isHealthAlerting = ref(false);
    const healthAlertLabel = ref('');
    const healthAlertType = ref<'sitting' | 'water'>('sitting');

    // 硬件监控（灵动岛附属图标，由 LiveActive 的开关驱动；数据来自后端 monitor-stats 推送）
    const hwEnabled = ref(getSettingRaw(NSD_HW_ENABLED) === 'true');
    const hwMode = ref(getSettingRaw(NSD_HW_MODE) || 'single');
    // E1：指标统一为 cpu/mem/battery/disk 四选；localStorage 存量值校验，非法值回落默认
    const loadHwMetric = (key: string, fallback: HwMetric): HwMetric => {
        const raw = getSettingRaw(key);
        return raw && (HW_METRICS as readonly string[]).includes(raw) ? (raw as HwMetric) : fallback;
    };
    const hwDefaultMetric = ref<HwMetric>(loadHwMetric(NSD_HW_DEFAULT_METRIC, 'cpu'));
    // 双圆环外/内环独立指标（默认 cpu+mem 保持既有观感）
    const hwRingOuter = ref<HwMetric>(loadHwMetric(NSD_HW_RING_OUTER, 'cpu'));
    const hwRingInner = ref<HwMetric>(loadHwMetric(NSD_HW_RING_INNER, 'mem'));
    const hwCpuPct = ref(0);
    const hwMemPct = ref(0);
    // monitor-stats 扩展数据源：电池推送前按"无电池"哨兵兜底，磁盘默认 0
    const hwBatteryPct = ref<number>(HW_BATTERY_UNAVAILABLE);
    const hwDiskPct = ref(0);
    const isHardwareExpanded = ref(false);

    // ===== 展示守卫（注册表统一遍历） =====
    // 守卫谓词的取数源：全部为只读消费，展开态的写入经由主组件注入的 actions
    const guardCtx: ActivityGuardCtx = {
        isPomodoroVisible, isPomodoroExpanded,
        isCountdownVisible, isCountdownExpanded,
        hwEnabled, isHardwareExpanded, isHealthAlerting,
    };

    // 单个文本态的展示守卫：消息/toast/音乐展开让位；可见 且（无音乐控制 或 已展开）。
    // （旧实现里的 isExpandedSize 分支不可达：isExpandedSize = isMusicExpanded || isMsgActive，
    // 两者已被首位守卫拦截，随守卫统一化一并移除）
    const makeTextGuard = (visible: Ref<boolean>, expanded: Ref<boolean>): ComputedRef<boolean> =>
        computed(() => {
            if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
            return visible.value && (!isMusicCtlEnabled.value || expanded.value);
        });

    // 文本态取数源由注册表声明（textSources），统一遍历生成展示守卫
    const textGuards: Partial<Record<RtId, ComputedRef<boolean>>> = {};
    for (const def of RT_ACTIVITY_DEFS) {
        const src: TextSources | null | undefined = def.textSources?.(guardCtx);
        // 声明了 textSources 的活动必为实时活动（sysmsg 无岛上形态），断言固化该不变式
        if (src) textGuards[def.id as RtId] = makeTextGuard(src.visible, src.expanded);
    }

    // 文本态活动缺守卫 = 注册表配置缺失，不兜底、直接暴露
    const showPomodoroText = textGuards['pomodoro']!;
    const showCountdownText = textGuards['countdown']!;
    const showHardwareRing = textGuards['hardware']!;

    // 分屏布局：任一文本态活动"可见且未展开"即进入（硬件展开态除外，与旧实现一致）
    const isSplitMode = computed(() => {
        if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
        if (isHardwareExpanded.value) return false;
        if (!isMusicCtlEnabled.value) return false;
        for (const def of RT_ACTIVITY_DEFS) {
            const src = def.textSources?.(guardCtx);
            if (src && src.visible.value && !src.expanded.value) return true;
        }
        return false;
    });

    // 番茄钟计算属性
    const formattedIslandPomoTime = computed(() => {
        const m = Math.floor(pomodoroRemainingSecs.value / 60).toString().padStart(2, '0');
        const s = (pomodoroRemainingSecs.value % 60).toString().padStart(2, '0');
        return `${m}:${s}`;
    });

    const pomodoroPhaseClass = computed(() => {
        return pomodoroPhase.value === 'focus' ? 'phase-focus' : 'phase-break';
    });

    // 倒计时计算属性
    const formattedIslandCdTime = computed(() => {
        const m = Math.floor(countdownRemainingSecs.value / 60).toString().padStart(2, '0');
        const s = (countdownRemainingSecs.value % 60).toString().padStart(2, '0');
        return `${m}:${s}`;
    });

    // 轮换模式：当前显示的指标（CPU / 内存）
    const hwRotateMetric = ref<'cpu' | 'mem'>('cpu');
    let hwRotationTimer: number | null = null;

    // 当前激活的指标（四指标统一口径）
    const hwActiveMetric = computed<HwMetric>(() => {
        if (hwMode.value === 'rotation') return hwRotateMetric.value;
        return hwDefaultMetric.value;
    });

    // 实时值集合：统一传给指标工具函数
    const hwVals = computed(() => ({
        cpu: hwCpuPct.value,
        mem: hwMemPct.value,
        battery: hwBatteryPct.value,
        disk: hwDiskPct.value,
    }));

    // 单圆环/轮换模式的进度百分比（电池不可用时按 0 渲染空环）
    const hwRingPct = computed(() => {
        return hwMetricClamp(hwMetricPctOf(hwActiveMetric.value, hwVals.value));
    });

    // 圆环颜色（不可用指标置灰）
    const hwRingColor = computed(() => {
        const unavailable = hwMetricPctOf(hwActiveMetric.value, hwVals.value) === null;
        return hwMetricColor(hwActiveMetric.value, hwRingPct.value, unavailable);
    });

    // 硬件监控轮换定时器：每 5 秒切换 CPU / 内存
    const startHwRotation = () => {
        stopHwRotation();
        hwRotationTimer = window.setInterval(() => {
            hwRotateMetric.value = hwRotateMetric.value === 'cpu' ? 'mem' : 'cpu';
        }, 5000);
    };

    const stopHwRotation = () => {
        if (hwRotationTimer) {
            clearInterval(hwRotationTimer);
            hwRotationTimer = null;
        }
    };

    // 兜底：无论 hwMode / hwEnabled 以何种方式变化（跨窗口事件、启动读取等），
    // 都确保轮换定时器与当前模式严格同步，避免轮换模式不工作
    watch([hwMode, hwEnabled], () => {
        if (hwEnabled.value && hwMode.value === 'rotation') {
            startHwRotation();
        } else {
            stopHwRotation();
        }
    });

    // 轮换功能核心逻辑（主岛内容：0=网速 1=音乐）
    const isRotationEnabled = ref(getSettingRaw(NSD_ROTATION_MODE) === 'true');
    const currentRotIndex = ref(0); // 0=网速 1=音乐
    let rotationTimer: number | null = null;

    const startRotation = () => {
        if (rotationTimer) clearInterval(rotationTimer);
        rotationTimer = window.setInterval(() => {
            currentRotIndex.value = (currentRotIndex.value + 1) % 2;
        }, 5000); // 5秒轮换一次
    };

    const stopRotation = () => {
        if (rotationTimer) {
            clearInterval(rotationTimer);
            rotationTimer = null;
        }
    };

    // 恢复番茄钟运行状态：查询后端当前状态（主组件 onMounted 调用）
    const restorePomodoroState = async () => {
        try {
            const state: any = await invoke('get_pomodoro_state');
            if (state.active) {
                pomodoroRemainingSecs.value = state.remaining_secs;
                pomodoroPhase.value = state.phase;
                pomodoroRemainingCycles.value = state.remaining_cycles;
                isPomodoroVisible.value = true;
                setSettingRaw(NSD_POMODORO_VISIBLE, 'true');
            }
        } catch (_e) {}
    };

    // 恢复倒计时运行状态：查询后端当前状态（主组件 onMounted 调用）
    const restoreCountdownState = async () => {
        try {
            const state: any = await invoke('get_countdown_state');
            if (state.active) {
                countdownRemainingSecs.value = state.remaining_secs;
                cdPaused.value = state.paused || false;
                isCountdownFinished.value = state.phase === 'finished';
                isCountdownVisible.value = true;
                setSettingRaw(NSD_COUNTDOWN_VISIBLE, 'true');
            }
        } catch (_e) {}
    };

    return {
        // 番茄钟
        isPomodoroVisible,
        pomodoroRemainingSecs,
        pomodoroPhase,
        pomodoroRemainingCycles,
        isPomodoroExpanded,
        formattedIslandPomoTime,
        pomodoroPhaseClass,
        showPomodoroText,
        // 倒计时
        isCountdownVisible,
        countdownRemainingSecs,
        isCountdownExpanded,
        isCountdownFinished,
        cdPaused,
        formattedIslandCdTime,
        showCountdownText,
        isSplitMode,
        // 健康提醒
        isHealthAlerting,
        healthAlertLabel,
        healthAlertType,
        // 硬件监控
        hwEnabled,
        hwMode,
        hwDefaultMetric,
        hwRingOuter,
        hwRingInner,
        hwCpuPct,
        hwMemPct,
        hwBatteryPct,
        hwDiskPct,
        isHardwareExpanded,
        hwActiveMetric,
        hwRingPct,
        hwRingColor,
        showHardwareRing,
        startHwRotation,
        stopHwRotation,
        // 主岛轮换
        isRotationEnabled,
        currentRotIndex,
        startRotation,
        stopRotation,
        // 启动恢复
        restorePomodoroState,
        restoreCountdownState,
    };
}

export type RealtimeActivity = ReturnType<typeof useRealtimeActivity>;
