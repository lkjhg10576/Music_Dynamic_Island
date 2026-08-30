/**
 * 实时活动 composable：番茄钟 / 倒计时 / 健康提醒 / 硬件监控附属图标 / 主岛内容轮换。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - 番茄钟/倒计时状态（由后端 pomodoro-tick / countdown-tick 事件驱动，事件监听仍在主组件 onMounted）
 *     + 启动时恢复运行状态的两段后端查询（restorePomodoroState / restoreCountdownState，由主组件 onMounted 调用）
 *   - 健康提醒（久坐/喝水）alerting 状态（由后端 health-reminder-tick 事件驱动）
 *   - 硬件监控附属图标：开关/模式/指标数据 + 轮换定时器（startHwRotation/stopHwRotation，含兜底 watch）
 *   - 主岛轮换模式：网速/音乐 5s 轮换（startRotation/stopRotation）
 *   - 展示谓词：showPomodoroText / showCountdownText / showHardwareRing / isSplitMode，
 *     供主组件 displaySpeed / displayMusic 计算属性守卫使用（接入点必须位于其之前）
 * 后端事件监听（pomodoro-tick / countdown-tick / health-reminder-tick / monitor-stats 等）与
 * 展开交互编排（clickRtChip / expandHardware / collapseAllExpandedActivities 等）留在主组件；
 * 岛尺寸恢复动画（animateIslandSize 等）通过主组件中转，不在本域重复实现。
 */
import { ref, computed, watch, type Ref, type ComputedRef } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSettingRaw, setSettingRaw } from '../utils/settings';
import {
    NSD_POMODORO_VISIBLE,
    NSD_COUNTDOWN_VISIBLE,
    NSD_HW_ENABLED,
    NSD_HW_MODE,
    NSD_HW_DEFAULT_METRIC,
    NSD_ROTATION_MODE,
} from '../constants/storageKeys';

export function useRealtimeActivity(deps: {
    // 跨域守卫：消息通知 / 系统 toast / 音乐展开态优先级高于实时活动展示
    isMsgActive: Ref<boolean>;
    displaySysToast: Ref<boolean>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    isExpandedSize: ComputedRef<boolean>;
    isMusicCtlEnabled: Ref<boolean>;
}) {
    const {
        isMsgActive, displaySysToast, isMusicExpanded, isMusicExpanding,
        isExpandedSize, isMusicCtlEnabled,
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

    // 番茄钟计算属性
    const formattedIslandPomoTime = computed(() => {
        const m = Math.floor(pomodoroRemainingSecs.value / 60).toString().padStart(2, '0');
        const s = (pomodoroRemainingSecs.value % 60).toString().padStart(2, '0');
        return `${m}:${s}`;
    });

    const pomodoroPhaseClass = computed(() => {
        return pomodoroPhase.value === 'focus' ? 'phase-focus' : 'phase-break';
    });

    const showPomodoroText = computed(() => {
        if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
        if (isPomodoroVisible.value && !isMusicCtlEnabled.value) return true;
        if (isPomodoroVisible.value && isMusicCtlEnabled.value && isPomodoroExpanded.value) return true;
        if (isExpandedSize.value) return true;
        return false;
    });

    // 倒计时计算属性
    const formattedIslandCdTime = computed(() => {
        const m = Math.floor(countdownRemainingSecs.value / 60).toString().padStart(2, '0');
        const s = (countdownRemainingSecs.value % 60).toString().padStart(2, '0');
        return `${m}:${s}`;
    });

    const showCountdownText = computed(() => {
        if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
        if (isCountdownVisible.value && !isMusicCtlEnabled.value) return true;
        if (isCountdownVisible.value && isMusicCtlEnabled.value && isCountdownExpanded.value) return true;
        if (isExpandedSize.value) return true;
        return false;
    });

    const isSplitMode = computed(() => {
        if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
        if (isHardwareExpanded.value) return false;
        return isPomodoroVisible.value && isMusicCtlEnabled.value && !isPomodoroExpanded.value
            || isCountdownVisible.value && isMusicCtlEnabled.value && !isCountdownExpanded.value
            || hwEnabled.value && isMusicCtlEnabled.value && !isHardwareExpanded.value;
    });

    // 硬件监控（灵动岛附属图标，由 LiveActive 的开关驱动；数据来自后端 monitor-stats 推送）
    const hwEnabled = ref(getSettingRaw(NSD_HW_ENABLED) === 'true');
    const hwMode = ref(getSettingRaw(NSD_HW_MODE) || 'single');
    const hwDefaultMetric = ref(getSettingRaw(NSD_HW_DEFAULT_METRIC) || 'cpu');
    const hwCpuPct = ref(0);
    const hwMemPct = ref(0);
    const isHardwareExpanded = ref(false);

    // 轮换模式：当前显示的指标（CPU / 内存）
    const hwRotateMetric = ref<'cpu' | 'mem'>('cpu');
    let hwRotationTimer: number | null = null;

    // 当前激活的指标
    const hwActiveMetric = computed(() => {
        if (hwMode.value === 'rotation') return hwRotateMetric.value;
        return hwDefaultMetric.value;
    });

    // 单圆环/轮换模式的进度百分比
    const hwRingPct = computed(() => {
        return hwActiveMetric.value === 'cpu' ? hwCpuPct.value : hwMemPct.value;
    });

    // 圆环颜色
    const hwRingColor = computed(() => {
        if (hwActiveMetric.value === 'cpu') {
            return hwCpuPct.value >= 80 ? '#a855f7' : '#ffffff';
        }
        return hwMemPct.value >= 80 ? '#ff4757' : '#3b82f6';
    });

    // 硬件监控主要显示（类似 showPomodoroText 模式）
    const showHardwareRing = computed(() => {
        if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
        if (!hwEnabled.value) return false;
        if (!isMusicCtlEnabled.value) return true;
        if (isHardwareExpanded.value) return true;
        if (isExpandedSize.value) return true;
        return false;
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
        hwCpuPct,
        hwMemPct,
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
