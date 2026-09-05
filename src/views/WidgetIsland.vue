<template>
    <transition @enter="onEnter" @leave="onLeave" :css="false">
        <div v-show="isIslandVisible" :class="['island-container', { 'has-music-border': isGlowBorderEnabled }]"
            @mousedown="handleMouseDown" @mousemove="handleMouseMove" @mouseup="handleMouseUp"
            @mouseleave="handleMouseLeave" @mouseenter="handleMouseEnter" :style="islandStyle"
            @contextmenu="handleRightClick">

            <div class="rainbow-border-glow" v-if="isGlowBorderEnabled" :style="{ opacity: glowOpacity }"></div>

            <!-- 沉浸模式背景层：当前专辑封面的大幅模糊图 + 噪点 + 黑色遮罩 -->
            <div v-if="showCoverglassBg" class="coverglass-bg-container" :style="coverglassStyle">
                <div class="coverglass-bg-image" :style="{ backgroundImage: `url(${blurredCoverUrl})` }"></div>
                <div class="coverglass-noise-layer"></div>
                <div class="coverglass-mask-layer"></div>
            </div>

            <!-- 左侧宽度调整手柄 -->
            <div class="resize-handle left"
                v-if="!isPositionLocked && !isMusicExpanded && !isMusicExpanding && !isMsgActive && !displaySysToast"
                @mousedown.stop="handleResizeStart($event, 'left')">
            </div>

            <div class="island-core-content" :style="coreContentStyle"
                :class="{ 'is-split-layout': isSplitMode, 'resize-cursor-left': mouseNearEdge === 'left', 'resize-cursor-right': mouseNearEdge === 'right' }">
                <div class="left-capsule" :class="{ 'is-split': isSplitMode }">
                    <div class="inner-wrapper">
                    <transition mode="out-in" @enter="onInnerEnter" @leave="onInnerLeave" :css="false">
                        <IslandMsg v-if="isMsgActive" key="msg" :msg-title="msgTitle" :msg-app-name="msgAppName"
                            :msg-body="msgBody" :current-msg-icon="currentMsgIcon" @select="handleNotificationClick" />

                        <IslandSysToast v-else-if="displaySysToast" key="systoast" :sys-toast-type="sysToastType"
                            :sys-toast-text="sysToastText" @select="onSysToastClick" />

                        <IslandHealthAlert v-else-if="isHealthAlerting" key="health-alert"
                            :health-alert-label="healthAlertLabel" />

                        <IslandCountdown v-else-if="showCountdownText" key="countdown"
                            :formatted-island-cd-time="formattedIslandCdTime" :is-countdown-finished="isCountdownFinished" />

                        <IslandPomodoro v-else-if="showPomodoroText" key="pomodoro"
                            :formatted-island-pomo-time="formattedIslandPomoTime" :pomodoro-phase-class="pomodoroPhaseClass"
                            :pomodoro-remaining-cycles="pomodoroRemainingCycles" />

                        <IslandHardwareRing v-else-if="showHardwareRing" key="hardware" :hw-mode="hwMode"
                            :hw-cpu-pct="hwCpuPct" :hw-mem-pct="hwMemPct" :hw-ring-pct="hwRingPct"
                            :hw-ring-color="hwRingColor" :hw-active-metric="hwActiveMetric"
                            :hw-ring-outer="hwRingOuter" :hw-ring-inner="hwRingInner"
                            :hw-battery-pct="hwBatteryPct" :hw-disk-pct="hwDiskPct"
                            :is-hardware-expanded="isHardwareExpanded" @expand="clickRtChip('hardware')" />

                        <IslandMusic v-else-if="displayMusic" :key="'music_' + musicBoxKey" ref="islandMusicRef"
                            :is-music-expanded="isMusicExpanded" :is-playing="isPlaying" :display-cover-url="displayCoverUrl"
                            :current-lyric-text="currentLyricText" :collapsed-track-text="collapsedTrackText"
                            :expanded-lyric-text="expandedLyricText" :expanded-sub-text="expandedSubText"
                            :is-video-like-source="isVideoLikeSource" :display-music="displayMusic"
                            :current-track-info="currentTrackInfo" :timeline-status="timelineStatus"
                            :progress-position="progressPosition" :progress-end="progressEnd"
                            :can-seek="musicTimeline.canSeek" :progress-percent="progressPercent"
                            :is-dragging-progress="isDraggingProgress"
                            @activate="expandMusic" @prev="prevTrack" @toggle-play="togglePlay" @next="nextTrack"
                            @progress-pointerdown="onProgressPointerDown" @progress-pointermove="onProgressPointerMove"
                            @progress-pointerup="onProgressPointerUp" @progress-pointercancel="onProgressPointerCancel" />

                        <IslandSpeed v-else-if="displaySpeed" key="speed" :is-showing-upload="isShowingUpload"
                            :upload-speed="uploadSpeed" :download-speed="downloadSpeed" :is-high-upload="isHighUpload"
                            :is-high-download="isHighDownload" />
                    </transition>
                </div>

                <transition mode="out-in" @enter="onInnerEnter" @leave="onInnerLeave" :css="false">
                    <!-- 实时活动展开面板：由活动注册表按 panelRank 命中（复刻旧 v-else-if 链：
                         countdown → pomodoro → health → hardware → printer） -->
                    <component v-if="rtPanel" :is="rtPanel.component" :key="rtPanel.key"
                        v-bind="rtPanel.props" v-on="rtPanel.events" />

                    <IslandSpectrum v-else-if="showSpectrumIndicator" key="spectrum" :bars="[...spectrumData]"
                        :is-playing="isPlaying" :is-music-expanded="isMusicExpanded" :bar-color="spectrumBarColor" />

                    <IslandStatusDot v-else :network-status="networkStatus" key="dot" />
                </transition>
                </div>

                <transition name="pop">
                    <!-- 多实时活动并行：单一常驻小图标（候选集按 priority 排序，点击展开当前预览活动 + 轮换到下一个） -->
                    <IslandRtChip v-if="showRtChip" :rt-activities="rtActivities" :current-rt-index="currentRtIndex"
                        :chip-content="previewChip" :core-content-style="coreContentStyle" @activate="clickRtChip()" />
                </transition>
            </div>

            <!-- 右侧宽度调整手柄 -->
            <div class="resize-handle right"
                v-if="!isPositionLocked && !isMusicExpanded && !isMusicExpanding && !isMsgActive && !displaySysToast"
                @mousedown.stop="handleResizeStart($event, 'right')">
            </div>
        </div>
    </transition>
</template>

<script setup lang="ts">
import { ref, shallowRef, triggerRef, onMounted, onUnmounted, computed, watch, type CSSProperties } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, currentMonitor, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/window';
import { listen, emit } from '@tauri-apps/api/event';
import { formatSpeed } from '../utils/format';
import {
    NSD_AUTO_HIDE_DELAY, NSD_AUTO_HIDE_ENABLED,
    NSD_AUTO_COLLAPSE_DELAY, NSD_AUTO_COLLAPSE_ENABLED,
    NSD_ISLAND_OPACITY, NSD_ISLAND_THEME,
    NSD_MUSIC_CTRL, NSD_GLOW_BORDER,
    NSD_PIN_TASKBAR, NSD_POSITION_LOCKED,
    NSD_MSG_MODE,
    NSD_ISLAND_POSITION, NSD_MSG_NOTIFY,
    NSD_TARGET_PLAYER,
    NSD_POMODORO_VISIBLE,
    NSD_COUNTDOWN_VISIBLE,
    NSD_HW_ENABLED,
    NSD_HW_MODE,
    NSD_HW_DEFAULT_METRIC,
    NSD_HW_RING_OUTER,
    NSD_HW_RING_INNER,
    NSD_ACTIVITY_PRIORITY,
    NSD_SYSMSG_ENABLED,
    NSD_SPECTRUM_COLOR_MODE,
    NSD_SPECTRUM_CUSTOM_COLOR,
    NSD_SPRING_STYLE,
    NSD_BORDER_RADIUS,
    NSD_ALWAYS_ON_TOP,
    NSD_BASE_WIDTH,
    NSD_MUSIC_BASE_WIDTH,
    NSD_MUSIC_EXPANDED_WIDTH,
    NSD_MSG_EXPANDED_WIDTH,
    NSD_APP_SCALE,
    NSD_LYRIC_DELAY,
} from '../constants/storageKeys';
import { getSettingRaw, setSettingRaw } from '../utils/settings';
import { useMusicSync } from '../composables/useMusicSync';
import { useLyrics } from '../composables/useLyrics';
import { useIslandAnimation } from '../composables/useIslandAnimation';
import { useRealtimeActivity } from '../composables/useRealtimeActivity';
import { useNotifications, type ToastItem, type AccessStatus } from '../composables/useNotifications';
import IslandSpeed from '../components/island/IslandSpeed.vue';
import IslandPomodoro from '../components/island/IslandPomodoro.vue';
import IslandCountdown from '../components/island/IslandCountdown.vue';
import IslandHealthAlert from '../components/island/IslandHealthAlert.vue';
import IslandMsg from '../components/island/IslandMsg.vue';
import IslandSysToast from '../components/island/IslandSysToast.vue';
import IslandHardwareRing from '../components/island/IslandHardwareRing.vue';
import IslandRtChip from '../components/island/IslandRtChip.vue';
import IslandMusic from '../components/island/IslandMusic.vue';
import IslandSpectrum from '../components/island/IslandSpectrum.vue';
import IslandStatusDot from '../components/island/IslandStatusDot.vue';
import type { CalendarEventInfo, PrintJob, PrintQueueState } from '../components/island/types';
// 活动注册表：候选 id / 元数据 / 活跃谓词 / 芯片与面板视图的单一来源（阶段 G）
import {
    RT_ACTIVITY_DEFS, RT_IDS, PANEL_DEFS_BY_RANK, getRtDef,
    type IslandActivityCtx, type PanelView, type ChipContent,
} from '../activities/registry';
import { useIslandPointer } from '../composables/useIslandPointer';
import { useIslandAutoHide } from '../composables/useIslandAutoHide';
import { useIslandContextMenu } from '../composables/useIslandContextMenu';

const isIslandVisible = ref(false);
// 记录是否开启了置于任务栏 / 锁定了位置（原声明在位置持久化区块，前移供尺寸动画 composable 接入）
const isPinnedToTaskbar = ref(getSettingRaw(NSD_PIN_TASKBAR) === 'true');
const isPositionLocked = ref(getSettingRaw(NSD_POSITION_LOCKED) === 'true');

// ===== 尺寸动画 composable 接入（弹簧形变动画 + 宽度持久化 + 自定义横向拖拽，逻辑从本组件拆出） =====
const {
    currentWidth, currentHeight, isSizeAnimating, animateIslandSize,
    MIN_WIDTH, MAX_WIDTH, saveIslandWidth, restoreIslandWidth, getExpandTargetWidth,
    isCustomDragging, startCustomHorizontalDrag, handleCustomDragEnd, cleanupIslandAnimation,
} = useIslandAnimation({ isPinnedToTaskbar });

// 打印队列相关变量（由后端 print-queue-tick 事件驱动；类型定义在 components/island/types.ts）
const printJobs = ref<PrintJob[]>([]);
const defaultPrinter = ref('');
const isPrintQueueActive = computed(() => printJobs.value.length > 0);
const isPrintQueueExpanded = ref(false);

// 日程同步相关变量（F：由后端 calendar-tick 事件驱动；系统日历 + 手动提醒的未来 24h 列表）
const calUpcoming = ref<CalendarEventInfo[]>([]);
const isCalendarExpanded = ref(false);

// 硬件监控附属图标可见性已合并到 showRtChip（多活动并行轮换），原 isHwAccessoryVisible 不再单独使用

// ===== 多实时活动并行：单图标轮换 + 点击展开 + X 回退 =====
// 活动的 id 集、图标/配色、默认优先级与活跃谓词统一来自活动注册表（activities/registry.ts）；
// 本组件只负责：候选集过滤（activityConfig × rtActive）、轮换下标、展开编排与尺寸动画。
// 岛上下文在 useRealtimeActivity 接入点之后赋值；候选集/面板等计算属性均为惰性求值，
// 首次求值（watch/渲染）晚于赋值，不存在使用早于初始化的问题
let islandCtx: IslandActivityCtx;

// 各活动的"当前活跃"谓词（沿用现有事件驱动标志；由注册表按 id 派发）
const rtActive = computed<Record<string, boolean>>(() => {
    const map: Record<string, boolean> = {};
    for (const def of RT_ACTIVITY_DEFS) {
        if (def.realtime) map[def.id] = def.isActive(islandCtx);
    }
    return map;
});

// 跨窗口同步来的配置（由 LiveActive emit('control-activity-config') 推送；启动时也从 localStorage 兜底读取）
const activityConfig = ref<Record<string, { enabled: boolean; priority: number }>>({});

// 候选集（enabled && active，按 priority 升序 + 注册表声明顺序平局打破）
const rtActivities = computed(() => {
    return RT_IDS
        .filter(id => activityConfig.value[id]?.enabled && rtActive.value[id])
        .map(id => {
            const def = getRtDef(id);
            return { id, priority: activityConfig.value[id].priority, icon: def.icon, accent: def.accent };
        })
        .sort((a, b) => {
            const pa = a.priority, pb = b.priority;
            return pa !== pb ? pa - pb : RT_IDS.indexOf(a.id) - RT_IDS.indexOf(b.id);
        });
});

// 当前小图标指向的候选下标；展开后预览的下一个活动下标
const currentRtIndex = ref(0);
// 当前展开的活动 id（null = 未展开任何实时活动）
const expandedRtId = ref<string | null>(null);
// 展开瞬间快照：决定 X 关闭后还原到音乐岛还是独立小图标态
const previousContext = ref<'music' | 'chip'>('chip');

// 小图标是否显示：有候选 且 (未展开 或 展开的不是当前预览活动)
// 同时排除：消息通知 / 系统 toast / 健康提醒"独占态"（健康提醒 active 时本身就有专属岛态，无需小图标）
// 注意：hardware 候选时若环已在主岛显示，视觉上会与小图标并存；用户点击环或小图标均可展开 hardware 详情（功能等价）
const showRtChip = computed(() => {
    if (rtActivities.value.length === 0) return false;
    const previewId = rtActivities.value[currentRtIndex.value]?.id;
    if (expandedRtId.value && expandedRtId.value === previewId) return false;
    // 健康提醒"alerting"独占岛态时，不显示小图标（避免重复）
    if (isHealthAlerting.value && expandedRtId.value !== 'health') return false;
    return true;
});

// 当前预览（最右小图标所代表）的活动 id（rtActivities[currentRtIndex]?.id 的快捷访问）
// 注：直接在 template / 计算属性中用 rtActivities[currentRtIndex]?.id，不再单独声明计算属性

// 点击小图标展开当前预览活动；若传入 targetId（如硬件环点击），则展开指定活动
// 同时推进 currentRtIndex 到下一个候选（用于下次小图标显示）
// 展开动作（展开态标记 + 尺寸动画）由活动注册表按 id 派发，本函数不再逐活动分支
function clickRtChip(targetId?: string) {
    const list = rtActivities.value;
    if (!list.length) return;
    // 先折叠所有已展开的实时活动，避免多活动并行时状态残留导致关闭按钮/切换异常
    collapseAllExpandedActivities();
    // 音乐面板处于展开/展开中时必须先复位其展开态：实时活动展开会把岛切回小岛高度，
    // 若 isMusicExpanded 残留，音乐控制面板无法随之折叠，且会卡住 showHardwareRing 等
    // 叠加显示逻辑（它们的守卫都依赖 !isMusicExpanded），导致显示异常
    resetMusicExpandedState();
    let idx: number;
    if (targetId) {
        idx = list.findIndex(a => a.id === targetId);
        if (idx < 0) {
            // targetId 不在候选集中（如用户未在控制台启用该活动），但仍允许展开（兜底）；
            // 未注册的活动无展开动作，仅标记展开态
            expandedRtId.value = targetId;
            previousContext.value = isPlaying.value ? 'music' : 'chip';
            RT_ACTIVITY_DEFS.find(def => def.id === targetId)?.expand?.(islandCtx);
            return;
        }
    } else {
        idx = currentRtIndex.value % list.length;
    }
    const target = list[idx];
    previousContext.value = isPlaying.value ? 'music' : 'chip';
    // 推进到下一个候选（下次小图标显示）
    currentRtIndex.value = (idx + 1) % list.length;
    // 触发展开：health 无手动展开（alerting 时由 health-reminder-tick 自动驱动，注册表不注册 expand）
    expandedRtId.value = target.id;
    getRtDef(target.id).expand?.(islandCtx);
}

function revertRealtime() {
    expandedRtId.value = null;
    // 关闭/回退后把小图标重新指向最高优先级候选（rtActivities 按优先级升序，下标 0 即最高），
    // 否则展开期间轮换上来的低优先级活动会一直占据小图标位，顺序设置形同虚设
    currentRtIndex.value = 0;
    // previousContext 决定还原到音乐岛或独立小图标态（chip 由 showRtChip 自然恢复）
    // 维持现有自动隐藏行为
    scheduleAutoHide();
}

// 启动时从 localStorage 读取优先级 map，初始化 activityConfig（与 LiveActive 推送双保险）
// NSD_ACTIVITY_PRIORITY 现行格式为 { id: number }；兼容旧格式 { id: { enabled, priority } }
// 显示与否最终由 rtActive 控制，故纯数字/缺项场景下 enabled 默认 true，避免启动时因未打开设置页而漏图标
function loadActivityConfigFromStorage() {
    let parsed: Record<string, any> | null = null;
    try {
        const raw = getSettingRaw(NSD_ACTIVITY_PRIORITY);
        if (raw) {
            const p = JSON.parse(raw);
            if (p && typeof p === 'object' && !Array.isArray(p)) {
                parsed = p as Record<string, any>;
            }
        }
    } catch (_e) {
        parsed = null;
    }

    const map: Record<string, { enabled: boolean; priority: number }> = {};
    RT_IDS.forEach((id) => {
        const defaultPriority = getRtDef(id).defaultPriority;
        const entry = parsed ? parsed[id] : undefined;
        if (typeof entry === 'number' && Number.isFinite(entry)) {
            // 现行格式：纯数字优先级
            map[id] = { enabled: true, priority: entry };
        } else if (entry && typeof entry === 'object') {
            // 旧格式：{ enabled, priority }；字段缺失时逐项回退
            const priority = typeof entry.priority === 'number' && Number.isFinite(entry.priority)
                ? entry.priority
                : defaultPriority;
            const enabled = typeof entry.enabled === 'boolean' ? entry.enabled : true;
            map[id] = { enabled, priority };
        } else {
            // 无配置 / 损坏条目：默认优先级，enabled=true（由 rtActive 决定是否真正显示）
            map[id] = { enabled: true, priority: defaultPriority };
        }
    });
    activityConfig.value = map;
}

// 关闭番茄钟展开（恢复岛尺寸 + 统一回退 expandedRtId）
const handlePomoClose = () => {
    isPomodoroExpanded.value = false;
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
    // 统一回退：清除 expandedRtId + 维持自动隐藏
    if (expandedRtId.value === 'pomodoro') {
        revertRealtime();
    }
};

// 关闭健康提醒
const handleDismissHealthAlert = async () => {
    if (healthAlertType.value === 'sitting') {
        await invoke('dismiss_sitting_alert').catch(() => {});
    } else {
        await invoke('dismiss_water_alert').catch(() => {});
    }
    isHealthAlerting.value = false;
    healthAlertLabel.value = '';
    // 恢复岛尺寸
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
    // 统一回退：清除 expandedRtId + 维持自动隐藏
    if (expandedRtId.value === 'health') {
        revertRealtime();
    }
};

// 倒计时控制函数
const handleCdTogglePauseResume = async () => {
    if (cdPaused.value) {
        await invoke('resume_countdown');
    } else {
        await invoke('pause_countdown');
    }
};

const handleCdClose = async () => {
    if (isCountdownFinished.value) {
        // 倒计时已结束 → 点击关闭即消音（终止重复响铃），由 countdown-tick idle 事件处理UI清理
        // 不直接操作 UI 状态，避免与自动隐藏等功能冲突
        await invoke('stop_countdown_alarm');
        isCountdownExpanded.value = false;
        if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
            const { h } = getBaseSize();
            const savedWidth = restoreIslandWidth();
            const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
            animateIslandSize(targetWidth, h);
        }
        // 统一回退
        if (expandedRtId.value === 'countdown') {
            revertRealtime();
        }
        return;
    }
    // 倒计时进行中 → 仅折叠展开态（与番茄钟行为一致），不停止倒计时
    isCountdownExpanded.value = false;
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
    // 统一回退
    if (expandedRtId.value === 'countdown') {
        revertRealtime();
    }
};

// 记录当前是否显示上行网速（用于轮换）
const isShowingUpload = ref(false);
let speedCycleTimer: number | null = null;

// ===== 消息展开宽度 / 音乐展开态标记（通知 composable 与音乐域守卫共用，声明位置随接入点前移） =====
const msgExpandedWidth = ref(Number(getSettingRaw(NSD_MSG_EXPANDED_WIDTH)) || 360);
const isMusicExpanded = ref(false);
const isMusicExpanding = ref(false); // 记录是否正在播放弹性按压展开动画
let musicExpandAnimTimer: number | null = null; // 用于接管展开时的定时器，防止冲突

// ===== 通知 composable 接入（消息队列 + 系统 toast 状态机 + 通知点击，逻辑从本组件拆出） =====
// scheduleAutoHide / getBaseSize / showSpectrumIndicator 定义在本文件后部，晚绑定桥接：
const {
    isMsgActive, msgTitle, msgAppName, msgBody, currentMsgIcon, msgQueue,
    displaySysToast, sysToastText, sysToastType,
    showToast, showSysmsgToast, onSysToastClick, handleNotificationClick,
    processMsgQueue, cleanupNotifications,
} = useNotifications({
    isIslandVisible, isPinnedToTaskbar, msgExpandedWidth, isMusicExpanded, isMusicExpanding,
    currentWidth, restoreIslandWidth, animateIslandSize,
    scheduleAutoHide: (delay?: number) => scheduleAutoHide(delay),
    getBaseSize: () => getBaseSize(),
    showSpectrumIndicator: () => showSpectrumIndicator.value,
    showRtChip: () => showRtChip.value,
});

// ===== 指针交互 composable 接入（宽度调整手柄 + 边缘光标 + 拖拽判定路由，逻辑从本组件拆出） =====
// mouseDownX / mouseDownY 供 expandMusic 做点击位移判定；定时器/文档级监听清理随 composable
const {
    mouseNearEdge, mouseDownX, mouseDownY,
    handleMouseDown, handleMouseMove, handleMouseUp, handleResizeStart,
} = useIslandPointer({
    isPositionLocked, isMusicExpanded, isMusicExpanding, isMsgActive, displaySysToast,
    isSizeAnimating, isPinnedToTaskbar, currentWidth, currentHeight,
    minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH,
    startCustomHorizontalDrag, handleCustomDragEnd, saveIslandWidth,
});

// ===== 系统动态感知（sysmsg）=====
// 总开关：默认 false；由控制台动态感知卡片控制（本地 + control-sysmsg-config 即时同步）
// 网络 toast / 状态灯由后端 NetworkMonitor 推送（sysmsg-event / network-status）
const isSysmsgEnabled = ref(getSettingRaw(NSD_SYSMSG_ENABLED) === 'true');

// 灵动岛自身的透明度变量（默认100%）
const islandOpacity = ref(Number(getSettingRaw(NSD_ISLAND_OPACITY) || '100'));

// 灵动岛自身主题色
const islandTheme = ref(getSettingRaw(NSD_ISLAND_THEME) || 'black');

// 个性化设置（默认值 = 现状，保证向后兼容）
const springStyle = ref<'stiff' | 'bouncy'>(
    (getSettingRaw(NSD_SPRING_STYLE) as 'stiff' | 'bouncy') || 'bouncy'
);
const borderRadius = ref(Number(getSettingRaw(NSD_BORDER_RADIUS)) || 100);
const isAlwaysOnTop = ref(getSettingRaw(NSD_ALWAYS_ON_TOP) !== 'false');
const baseWidth = ref(Number(getSettingRaw(NSD_BASE_WIDTH)) || 150);
const musicBaseWidth = ref(Number(getSettingRaw(NSD_MUSIC_BASE_WIDTH)) || 260);
const musicExpandedWidth = ref(Number(getSettingRaw(NSD_MUSIC_EXPANDED_WIDTH)) || 320);
const appScale = ref(Number(getSettingRaw(NSD_APP_SCALE)) || 1.0);

const applyAppScale = (scale: number) => {
    document.documentElement.style.zoom = String(scale);
};

const applyAlwaysOnTop = async (enabled: boolean) => {
    try {
        await getCurrentWindow().setAlwaysOnTop(enabled);
    } catch (e) {
        console.error(e);
    }
};

const refreshIslandSizeIfIdle = () => {
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
};

// 1. 瞬间判定当前是否处于大窗口状态
const isExpandedSize = computed(() => isMusicExpanded.value || isMsgActive.value);

// 2. 外层容器：状态一变，立马切成目标圆角
const islandStyle = computed<CSSProperties>(() => {
    const linear = islandOpacity.value / 100;
    const alpha = Math.pow(linear, 1 / 2.2);
    let bg = `rgba(0, 0, 0, ${alpha})`;
    let color = '#ffffff';

    if (islandTheme.value === 'white') {
        bg = `rgba(255, 255, 255, ${alpha})`;
        color = '#000000';
    } else if (showCoverglassBg.value) {
        // 沉浸模式：岛本体保持暗色底，模糊封面作为叠加层铺在上面
        bg = `rgba(20, 20, 20, ${alpha})`;
    }

    return {
        backgroundColor: bg,
        color: color,
        width: '100vw',
        height: '100vh',
        // 只要展开就是 24px，收起就是 100px
        borderRadius: isExpandedSize.value ? '24px' : (borderRadius.value === 12 ? '12px' : '100px'),
        position: 'relative',
    };
});

// 3. 内层核心：永远比外层小 2px
const coreContentStyle = computed(() => {
    const linear = islandOpacity.value / 100;
    const alpha = Math.pow(linear, 1 / 2.2);

    // 展开 22px，收起 98px
    const innerRadius = isExpandedSize.value ? '22px' : (borderRadius.value === 12 ? '10px' : '98px');

    if (islandTheme.value === 'white') {
        return {
            backgroundColor: `rgba(255, 255, 255, ${alpha})`,
            borderRadius: innerRadius
        };
    }
    if (showCoverglassBg.value) {
        // 沉浸模式：内层透出外层暗色底与模糊封面叠加层
        return {
            backgroundColor: `transparent`,
            borderRadius: innerRadius
        };
    }
    return {
        backgroundColor: `rgba(0, 0, 0, ${alpha})`,
        borderRadius: innerRadius
    };
});

const glowOpacity = computed(() => {
    const linear = islandOpacity.value / 100;
    return Math.pow(linear, 1 / 2.2);
});

const uploadSpeed = ref('0 KB/s');
const downloadSpeed = ref('0 KB/s');

// 记录当前是否属于大流量状态
const isHighDownload = ref(false);
const isHighUpload = ref(false);

// 网络状态指示灯：good(绿), warning(黄), error(红)
const networkStatus = ref<'good' | 'warning' | 'error'>('good');

const expandHardware = () => {
    if (isHardwareExpanded.value) return;
    suppressContentWatch = true;
    isHardwareExpanded.value = true;
    // 标记 hardware 为当前展开活动（previousContext 与 currentRtIndex 推进由 clickRtChip 统一处理）
    expandedRtId.value = 'hardware';
    // 展开时宽度不低于最小展开宽度（过窄时临时回调），只按需调整高度，避免岛宽被缩窄
    const { h } = getBaseSize();
    animateIslandSize(getExpandTargetWidth(), h);
    setTimeout(() => { suppressContentWatch = false; }, 600);
};

const collapseHardware = () => {
    if (!isHardwareExpanded.value) return;
    suppressContentWatch = true;
    isHardwareExpanded.value = false;
    // 同步多活动并行状态：清除 expandedRtId（统一回退路径）
    expandedRtId.value = null;
    // 与 revertRealtime 一致：小图标回到最高优先级候选，保证顺序设置生效
    currentRtIndex.value = 0;
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
    setTimeout(() => { suppressContentWatch = false; }, 600);
    // 维持自动隐藏行为
    scheduleAutoHide();
};

const expandPrintQueue = () => {
    if (!isPrintQueueActive.value || isPrintQueueExpanded.value) return;
    suppressContentWatch = true;
    isPrintQueueExpanded.value = true;
    expandedRtId.value = 'printer';
    const { w, h } = getBaseSize();
    // 最多展示两项高度，更多作业在详情内滚动。
    animateIslandSize(w + 220, h + Math.min(printJobs.value.length, 2) * 38);
    setTimeout(() => { suppressContentWatch = false; }, 600);
};

const collapsePrintQueue = (restore = true) => {
    if (!isPrintQueueExpanded.value) return;
    suppressContentWatch = true;
    isPrintQueueExpanded.value = false;
    if (expandedRtId.value === 'printer') {
        expandedRtId.value = null;
        // 与 revertRealtime 一致：小图标回到最高优先级候选，保证顺序设置生效
        currentRtIndex.value = 0;
    }
    if (restore) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
        scheduleAutoHide();
    }
    setTimeout(() => { suppressContentWatch = false; }, 600);
};

// 日程同步展开/折叠（F）：与硬件详情同款宽度兜底，列表在面板内滚动
const expandCalendar = () => {
    if (isCalendarExpanded.value) return;
    suppressContentWatch = true;
    isCalendarExpanded.value = true;
    expandedRtId.value = 'calendar';
    const { h } = getBaseSize();
    animateIslandSize(getExpandTargetWidth(), h);
    setTimeout(() => { suppressContentWatch = false; }, 600);
};

const collapseCalendar = () => {
    if (!isCalendarExpanded.value) return;
    suppressContentWatch = true;
    isCalendarExpanded.value = false;
    if (expandedRtId.value === 'calendar') {
        expandedRtId.value = null;
        // 与 revertRealtime 一致：小图标回到最高优先级候选，保证顺序设置生效
        currentRtIndex.value = 0;
    }
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
    setTimeout(() => { suppressContentWatch = false; }, 600);
    scheduleAutoHide();
};

// 统一折叠所有已展开的实时活动，避免多活动并行时状态残留导致关闭按钮/切换异常
// （折叠动作同样由注册表派发：collapseHardware/collapsePrintQueue 自带未展开短路，等价旧的条件调用；
//   health 由 isHealthAlerting 事件驱动，注册表不注册折叠）
const collapseAllExpandedActivities = () => {
    for (const def of RT_ACTIVITY_DEFS) {
        def.collapse?.(islandCtx);
    }
};

// 音乐控制功能开关
const isMusicCtlEnabled = ref(getSettingRaw(NSD_MUSIC_CTRL) === 'true');
// 流光边框默认状态完全镜像音乐控制器（只要音乐控制器开着它就开，关了就一起关闭）。
const isGlowBorderEnabled = ref(getSettingRaw(NSD_GLOW_BORDER) === 'true');

// ===== 自动隐藏 / 自动折叠 / 全屏隐藏 composable 接入（逻辑从本组件拆出） =====
// isPlaying（useMusicSync 输出）与 collapseMusic（音乐域）定义在后，经访问器晚绑定；
// isPendingCollapse 属于音乐展开域，由本组件持有 ref、与 composable 共享
const isPendingCollapse = ref(false);
const {
    isMouseOver, isAutoHideEnabled, autoHideDelay,
    isAutoCollapseEnabled, autoCollapseDelay,
    isAutoHideFullscreen, isAutoHiding, isHidingForFullscreen,
    scheduleAutoHide, handleMouseLeave, handleMouseEnter,
} = useIslandAutoHide({
    isIslandVisible, isMusicCtlEnabled, isMusicExpanded, isMusicExpanding,
    isPlaying: () => isPlaying.value,
    mouseNearEdge,
    isPendingCollapse,
    collapseMusic: () => collapseMusic(),
});

// 律动频谱
const spectrumData = shallowRef([0.35, 0.35, 0.35, 0.35, 0.35]);

// 频谱颜色模式：'album'(跟随专辑) | 'theme'(跟随主题) | 'custom'(自定义)
const spectrumColorMode = ref(getSettingRaw(NSD_SPECTRUM_COLOR_MODE) || 'album');
const spectrumCustomColor = ref(getSettingRaw(NSD_SPECTRUM_CUSTOM_COLOR) || '#b6e0ee');
// 从专辑封面提取的主色（album 模式使用，默认色作为兜底）
const DEFAULT_SPECTRUM_COLOR = '#b6e0ee';
const ALBUM_COLOR_TRANSITION_DURATION = 400;
const albumDominantColor = ref(DEFAULT_SPECTRUM_COLOR);
let albumColorAnimationFrame: number | null = null;
let albumColorExtractionVersion = 0;

type RgbColor = { r: number; g: number; b: number };

const parseHexColor = (color: string): RgbColor | null => {
    const match = /^#([\da-f]{2})([\da-f]{2})([\da-f]{2})$/i.exec(color);
    if (!match) return null;
    return {
        r: parseInt(match[1], 16),
        g: parseInt(match[2], 16),
        b: parseInt(match[3], 16),
    };
};

const toHexColor = ({ r, g, b }: RgbColor): string => {
    const toChannel = (value: number) => Math.round(value).toString(16).padStart(2, '0');
    return `#${toChannel(r)}${toChannel(g)}${toChannel(b)}`;
};

const interpolateColor = (from: string, to: string, progress: number): string => {
    const fromRgb = parseHexColor(from);
    const toRgb = parseHexColor(to);
    if (!fromRgb || !toRgb) return to;
    return toHexColor({
        r: fromRgb.r + (toRgb.r - fromRgb.r) * progress,
        g: fromRgb.g + (toRgb.g - fromRgb.g) * progress,
        b: fromRgb.b + (toRgb.b - fromRgb.b) * progress,
    });
};

const animateAlbumColor = (targetColor: string) => {
    if (albumColorAnimationFrame !== null) {
        cancelAnimationFrame(albumColorAnimationFrame);
        albumColorAnimationFrame = null;
    }

    const startColor = albumDominantColor.value;
    if (startColor === targetColor) return;

    const startTime = performance.now();
    const animate = (now: number) => {
        const progress = Math.min((now - startTime) / ALBUM_COLOR_TRANSITION_DURATION, 1);
        const easedProgress = 1 - Math.pow(1 - progress, 3);
        albumDominantColor.value = interpolateColor(startColor, targetColor, easedProgress);

        if (progress < 1) {
            albumColorAnimationFrame = requestAnimationFrame(animate);
        } else {
            albumDominantColor.value = targetColor;
            albumColorAnimationFrame = null;
        }
    };

    albumColorAnimationFrame = requestAnimationFrame(animate);
};

// 从封面图 URL 提取主色：canvas 缩小采样到 16x16 + RGB 量化到 4 档 + 取最频繁桶的平均色
// 跳过过暗/过亮像素，避免黑/白主导；失败或超时回退默认色
const extractAlbumColor = (url: string): Promise<string> => {
    return new Promise<string>((resolve) => {
        if (!url) { resolve(DEFAULT_SPECTRUM_COLOR); return; }
        const img = new Image();
        img.crossOrigin = 'anonymous';
        let settled = false;
        const done = (color: string) => { if (!settled) { settled = true; resolve(color); } };
        const timer = setTimeout(() => done(DEFAULT_SPECTRUM_COLOR), 3000);
        img.onload = () => {
            clearTimeout(timer);
            try {
                const size = 16;
                const canvas = document.createElement('canvas');
                canvas.width = size;
                canvas.height = size;
                const ctx = canvas.getContext('2d');
                if (!ctx) { done(DEFAULT_SPECTRUM_COLOR); return; }
                ctx.drawImage(img, 0, 0, size, size);
                const data = ctx.getImageData(0, 0, size, size).data;
                const bucket = new Map<string, { count: number, r: number, g: number, b: number }>();
                for (let i = 0; i < data.length; i += 4) {
                    const a = data[i + 3];
                    if (a < 128) continue; // 跳过透明像素
                    const r = data[i], g = data[i + 1], b = data[i + 2];
                    const max = Math.max(r, g, b), min = Math.min(r, g, b);
                    if (max < 30 || min > 225) continue; // 跳过过暗/过亮
                    const key = `${r >> 6}-${g >> 6}-${b >> 6}`;
                    const cur = bucket.get(key) || { count: 0, r: 0, g: 0, b: 0 };
                    cur.count++;
                    cur.r += r; cur.g += g; cur.b += b;
                    bucket.set(key, cur);
                }
                if (bucket.size === 0) { done(DEFAULT_SPECTRUM_COLOR); return; }
                let best: { count: number, r: number, g: number, b: number } | null = null;
                for (const v of bucket.values()) {
                    if (!best || v.count > best.count) best = v;
                }
                if (!best || best.count === 0) { done(DEFAULT_SPECTRUM_COLOR); return; }
                const r = Math.round(best.r / best.count);
                const g = Math.round(best.g / best.count);
                const b = Math.round(best.b / best.count);
                const toHex = (n: number) => n.toString(16).padStart(2, '0');
                done(`#${toHex(r)}${toHex(g)}${toHex(b)}`);
            } catch (e) {
                done(DEFAULT_SPECTRUM_COLOR);
            }
        };
        img.onerror = () => { clearTimeout(timer); done(DEFAULT_SPECTRUM_COLOR); };
        img.src = url;
    });
};

// 频谱条实际颜色：按当前模式计算
const spectrumBarColor = computed(() => {
    if (spectrumColorMode.value === 'custom') return spectrumCustomColor.value;
    if (spectrumColorMode.value === 'theme') {
        // 跟随主题：白色主题用深色条（深背景上反差），黑色主题用浅色条
        return islandTheme.value === 'white' ? '#1a1a1a' : '#b6e0ee';
    }
    return albumDominantColor.value; // album
});

// 从 MainPanel 抄过来的 CPU 静态模糊烘焙机
const bakeBlurImage = (url: string): Promise<string> => {
    return new Promise((resolve) => {
        const img = new Image();
        if (url.startsWith('http')) img.crossOrigin = 'anonymous';
        img.onload = () => {
            const canvas = document.createElement('canvas');
            canvas.width = 120; // 降低物理分辨率榨干性能
            canvas.height = 120;
            const ctx = canvas.getContext('2d');
            if (!ctx) return resolve(url);
            ctx.filter = 'blur(10px)';
            ctx.drawImage(img, -10, -10, 140, 140);
            try { resolve(canvas.toDataURL('image/jpeg', 0.6)); }
            catch (e) { resolve(url); }
        };
        img.onerror = () => resolve(url);
        img.src = url;
    });
};

// 烘焙并缓存沉浸模式模糊封面
const bakeAndStoreBlur = async (trackInfo: string, url: string) => {
    const baked = await bakeBlurImage(url);
    blurredCoverCache.set(trackInfo, baked);
    // 烘焙期间已切歌：只入缓存，不覆盖当前的沉浸背景
    if (currentTrackInfo.value === trackInfo) {
        blurredCoverUrl.value = baked;
    }
};

// 视频类/浏览器来源的应用 logo（coverUrl 为空时作为圆形封面兜底）
const appLogoFallback = computed(() => {
    const artist = currentArtistName.value;
    const id = currentAppIdStr.value;
    if (artist === 'bilibili' || id.includes('bilibili')) return bilibiliLogo;
    if (artist === 'edge' || id.includes('edge')) return edgeLogo;
    if (artist === 'chrome' || id.includes('chrome')) return chromeLogo;
    if (artist === 'potplayer' || id.includes('potplayer')) return potplayerLogo;
    return '';
});

// 圆形封面实际展示地址：网络/SMTC 封面优先，拿不到时回退应用 logo
const displayCoverUrl = computed(() => coverUrl.value || appLogoFallback.value);

// 记录消息模式开关状态
const isMsgModeEnabled = ref(getSettingRaw(NSD_MSG_MODE) === 'true');
// ===== 实时活动 composable 接入（番茄钟/倒计时/健康提醒/硬件监控/主岛轮换，逻辑从本组件拆出） =====
// 接入点必须位于 displaySpeed/displayMusic 计算属性之前：它们的守卫依赖本域输出的展示谓词
const {
    isPomodoroVisible, pomodoroRemainingSecs, pomodoroPhase, pomodoroRemainingCycles, isPomodoroExpanded,
    formattedIslandPomoTime, pomodoroPhaseClass, showPomodoroText,
    isCountdownVisible, countdownRemainingSecs, isCountdownExpanded, isCountdownFinished, cdPaused,
    formattedIslandCdTime, showCountdownText, isSplitMode,
    isHealthAlerting, healthAlertLabel, healthAlertType,
    hwEnabled, hwMode, hwDefaultMetric, hwCpuPct, hwMemPct, isHardwareExpanded,
    hwRingOuter, hwRingInner, hwBatteryPct, hwDiskPct,
    hwActiveMetric, hwRingPct, hwRingColor, showHardwareRing, startHwRotation, stopHwRotation,
    isRotationEnabled, currentRotIndex, startRotation, stopRotation,
    restorePomodoroState, restoreCountdownState,
} = useRealtimeActivity({
    isMsgActive, displaySysToast, isMusicExpanded, isMusicExpanding, isMusicCtlEnabled,
});

// ===== 活动注册表的岛上上下文（见顶部 islandCtx 声明） =====
// 动作一律以闭包注入：引用的处理器/getBaseSize 声明位置可能晚于本赋值，
// 但调用点全部在交互期/渲染期，不存在时序问题
islandCtx = {
    isPomodoroVisible, isPomodoroExpanded, isCountdownVisible, isCountdownExpanded,
    hwEnabled, isHardwareExpanded, isHealthAlerting,
    cdPaused, hwMode, hwDefaultMetric, hwCpuPct, hwMemPct, hwRingPct, hwRingColor,
    hwRingOuter, hwRingInner, hwBatteryPct, hwDiskPct,
    printJobs, defaultPrinter, isPrintQueueExpanded,
    calUpcoming, isCalendarExpanded,
    actions: {
        animateExpandSize: () => {
            const { h } = getBaseSize();
            animateIslandSize(getExpandTargetWidth(), h);
        },
        setPomodoroExpanded: expanded => { isPomodoroExpanded.value = expanded; },
        setCountdownExpanded: expanded => { isCountdownExpanded.value = expanded; },
        expandHardware: () => { expandHardware(); },
        collapseHardware: () => { collapseHardware(); },
        expandPrintQueue: () => { expandPrintQueue(); },
        collapsePrintQueue: restore => { collapsePrintQueue(restore ?? true); },
        expandCalendar: () => { expandCalendar(); },
        collapseCalendar: () => { collapseCalendar(); },
        toggleCountdownPauseResume: () => { handleCdTogglePauseResume(); },
        closeCountdownPanel: () => { handleCdClose(); },
        closePomodoroPanel: () => { handlePomoClose(); },
        dismissHealthAlert: () => { handleDismissHealthAlert(); },
    },
};

// 右侧展开面板：注册表按 panelRank 命中，返回首个命中的视图（null = 无面板，回落频谱/状态灯）
const rtPanel = computed<PanelView | null>(() => {
    for (const { panel } of PANEL_DEFS_BY_RANK) {
        const view = panel(islandCtx);
        if (view) return view;
    }
    return null;
});

// 芯片预览内容：注册表 chip 契约优先（硬件监控的动态圆环），缺省渲染活动静态图标
const previewChip = computed<ChipContent>(() => {
    const cur = rtActivities.value[currentRtIndex.value];
    if (!cur) return { kind: 'icon', icon: '' };
    return getRtDef(cur.id).chip?.(islandCtx) ?? { kind: 'icon', icon: cur.icon };
});
// 候选集收缩保护：若 expandedRtId 已不在候选集，则回退
watch(rtActivities, (list) => {
    if (expandedRtId.value && !list.find(a => a.id === expandedRtId.value)) {
        revertRealtime();
    }
    // currentRtIndex 越界保护
    if (list.length > 0 && currentRtIndex.value >= list.length) {
        currentRtIndex.value = currentRtIndex.value % list.length;
    }
    // 列表变化（新活动加入/离开、优先级重排）后，若预览恰好指向正在展开的活动，
    // showRtChip 会把小图标整体隐藏；这里推进一位让小图标保持可见
    if (expandedRtId.value && list.length > 0 && list[currentRtIndex.value]?.id === expandedRtId.value) {
        currentRtIndex.value = (currentRtIndex.value + 1) % list.length;
    }
});

// 使用计算属性智能判断当前该显示啥
const displaySpeed = computed(() => !isMsgActive.value && !displaySysToast.value && !showPomodoroText.value && !showCountdownText.value && !showHardwareRing.value && (isRotationEnabled.value ? currentRotIndex.value === 0 : !isMusicCtlEnabled.value));
const displayMusic = computed(() => !isMsgActive.value && !displaySysToast.value && !showPomodoroText.value && !showCountdownText.value && !showHardwareRing.value && (isRotationEnabled.value ? currentRotIndex.value === 1 : isMusicCtlEnabled.value));

// 辅助函数：获取当前状态应该拥有的默认大小
const getBaseSize = () => {
    if (isPomodoroVisible.value || isCountdownVisible.value || hwEnabled.value) return { w: 250, h: 38 };
    if (displaySpeed.value) return { w: baseWidth.value, h: 34 };
    return { w: musicBaseWidth.value, h: 42 };
};

let suppressContentWatch = false;

// 监听内容切换，触发丝滑动画
watch([displaySpeed, displayMusic, showPomodoroText, showCountdownText, showHardwareRing], () => {
    if (suppressContentWatch) return;
    // 只有在没有被临时弹窗
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
});

// 专门用于控制右侧常驻指示灯的独立计算属性（完全不受消息通知打断）
const showSpectrumIndicator = computed(() => {
    // 拆分模式（实时活动存在）下左侧胶囊已收窄至 calc(100% - 44px)，右侧圆钮独占 44px，
    // 二者不再重叠，因此保留频谱：让专辑图/歌名歌手/频谱三件套整体靠左，右侧为实时活动让位。
    return isRotationEnabled.value ? currentRotIndex.value === 1 : isMusicCtlEnabled.value;
});

// 频谱开关状态追踪，避免重复调用后端
let isSpectrumActive = false;

// 按需启停音频频谱捕获：音乐控制器模式开启且灵动岛可见时即激活后端 FFT。
// 改为以「音乐模式 + 可见」为启停条件（而非 isPlaying）：
//   1. 暂停时捕获仍在运行，但因无音频会自动回落到静默基准线（0.35），频谱条平滑归位，
//      彻底修复旧轮询逻辑移除后「暂停/切歌后频谱条卡死在最后高度」的回归；
//   2. 配合 immediate，窗口重建（省内存模式）后立刻恢复，无需等待下一次音乐轮询。
watch([showSpectrumIndicator, isIslandVisible], () => {
    const shouldActivate = showSpectrumIndicator.value && isIslandVisible.value;
    if (shouldActivate && !isSpectrumActive) {
        isSpectrumActive = true;
        invoke('start_audio_spectrum').catch(() => {});
    } else if (!shouldActivate && isSpectrumActive) {
        isSpectrumActive = false;
        invoke('stop_audio_spectrum').catch(() => {});
        // 关闭捕获时复位到静默基准线，避免残留上一首歌曲的波形
        spectrumData.value = [0.35, 0.35, 0.35, 0.35, 0.35];
    }
}, { immediate: true });

// 计算并吸附到左下角的方法
const snapToBottomLeft = async () => {
    try {
        const appWindow = getCurrentWindow();
        await new Promise((resolve) => setTimeout(resolve, 150));
        const monitor = await currentMonitor();

        if (monitor) {
            const scaleFactor = window.devicePixelRatio;

            const WINDOW_INIT_WIDTH = currentWidth.value;
            const WINDOW_INIT_HEIGHT = currentHeight.value;
            await appWindow.setSize(new PhysicalSize(Math.ceil(WINDOW_INIT_WIDTH * scaleFactor), Math.ceil(WINDOW_INIT_HEIGHT * scaleFactor)));

            const monitorLeftPhysical = monitor.position.x;
            const monitorTopPhysical = monitor.position.y;
            // 恢复使用 Tauri 最底层的硬件真实分辨率（绝对不会缩水）
            const monitorHeightPhysical = monitor.size.height;

            // X坐标: 屏幕最左侧 + 10px的边距
            const x = monitorLeftPhysical + (10 * scaleFactor);
            // Y坐标: 物理最底部 - 窗口高度 - 3px微调
            const y = monitorTopPhysical + monitorHeightPhysical - ((WINDOW_INIT_HEIGHT + 3) * scaleFactor);

            // 【终极绝杀核心】：绕过 Windows 系统的任务栏防遮挡机制
            // 在强制覆盖任务栏坐标之前，先隐身。
            await appWindow.hide();

            await appWindow.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));

            // 移动完成后，瞬间现身，生米煮成熟饭，Windows 也拦不住了！
            await appWindow.show();
        }
    } catch (error) {
        console.error('停靠左下角失败', error);
    }
};

const togglePlay = async () => {
    // 1. 前端先立刻切换图标，给用户极速的视觉反馈
    isPlaying.value = !isPlaying.value;

    // 2. 发送指令给 Rust 的 SMTC
    try {
        await invoke('control_system_media', { action: 'play_pause' });
    } catch (err) {
        console.error('播放控制失败:', err);
        // 如果底层控制失败了，再把图标状态回滚回去
        isPlaying.value = !isPlaying.value;
    }
};

const prevTrack = async () => {
    await invoke('control_system_media', { action: 'prev' });
};

const nextTrack = async () => {
    await invoke('control_system_media', { action: 'next' });
};

// 清理封面缓存并立即为当前歌曲重新拉取封面
const clearCoverCacheAndRefresh = async () => {
    bumpCoverFetchVersion();
    coverCache.clear();
    blurredCoverCache.clear();
    coverUrl.value = '';
    blurredCoverUrl.value = '';
    // 重置当前歌曲标识，确保 syncMusicStatus 会重新走封面获取逻辑
    currentTrackInfo.value = '';
    await syncMusicStatus();
};

const showInfo = ref(false);
// 默认显示内容动态从本地缓存读取
const getPlayerName = () => {
    const key = getSettingRaw(NSD_TARGET_PLAYER) || 'netease';
    const map: Record<string, string> = {
        'netease': '网易云音乐',
        'spotify': 'Spotify',
        'apple': 'Apple Music',
        'qqmusic': 'QQ音乐',
        'kugou': '酷狗音乐',
        'echo': 'Echo Music',
        'lx-music': '洛雪音乐',
        'smtc': 'SMTC',
        'bilibili': '哔哩哔哩',
        'edge': 'Microsoft Edge',
        'chrome': 'Google Chrome',
        'potplayer': 'PotPlayer',
        'justsolo': 'JustSolo',
    };
    return map[key] || '未知平台';
};

// SMTC 连上应用但没有有效标题时，把应用包名转成可读的应用名
const getConnectedAppName = (appId: string) => {
    const id = (appId || '').toLowerCase();
    if (id.includes('edge')) return 'Microsoft Edge';
    if (id.includes('chrome')) return 'Google Chrome';
    if (id.includes('bilibili')) return '哔哩哔哩';
    if (id.includes('cloudmusic') || id.includes('netease')) return '网易云音乐';
    if (id.includes('spotify')) return 'Spotify';
    if (id.includes('qqmusic')) return 'QQ音乐';
    if (id.includes('kugou')) return '酷狗音乐';
    if (id.includes('justsolo')) return 'JustSolo';
    if (id.includes('potplayer')) return 'PotPlayer';
    // 兜底：去掉 .exe 后缀后直接展示包名
    return id.replace(/\.exe$/i, '') || '未知应用';
};

// ===== 音乐同步 composable 接入（状态与逻辑从本组件拆出） =====
// useLyrics 与本域相互依赖（切歌要重置歌词、歌词要读播放态），晚绑定桥接：
let resetLyricStateImpl: () => void = () => {};
let fetchLyricsImpl: (song: string, artist: string) => Promise<void> = async () => {};

const {
    isPlaying, coverUrl, blurredCoverUrl, coverCache, blurredCoverCache,
    currentAppIdStr, currentIsBrowser, isBrowserMusic,
    currentSongName, currentArtistName, currentTrackInfo,
    bumpCoverFetchVersion, applyMusicInfo, applyNoTrack, syncMusicStatus,
} = useMusicSync({
    displayMusic, isIslandVisible, isMouseOver,
    getPlayerName, getConnectedAppName, bakeAndStoreBlur, scheduleAutoHide,
    resetLyricState: () => resetLyricStateImpl(),
    fetchLyricsForCurrentTrack: (song, artist) => fetchLyricsImpl(song, artist),
});

// 封面变化时，若处于 album 模式则将当前频谱颜色平滑过渡到新的取色结果
// （原位置在封面状态定义处，现随 composable 解构点后移以保证声明顺序）
watch(coverUrl, async (url) => {
    const extractionVersion = ++albumColorExtractionVersion;
    if (spectrumColorMode.value !== 'album') return;

    const targetColor = await extractAlbumColor(url);
    if (extractionVersion !== albumColorExtractionVersion
        || spectrumColorMode.value !== 'album'
        || url !== coverUrl.value) return;

    animateAlbumColor(targetColor);
});

// 定义一个用于强制刷新的 key
const musicBoxKey = ref(0);

// ===== F6 音乐进度条：展开态拉取 SMTC Timeline 并支持拖动定位 =====
type TimelineStatus = 'idle' | 'loading' | 'available' | 'unavailable';
type MusicTimelineResponse = { position_ms: number; end_ms: number; can_seek: boolean };

const musicTimeline = ref({ position: 0, end: 0, canSeek: false });
const timelineStatus = ref<TimelineStatus>('idle');
const timelineClock = ref(Date.now());
const timelineSyncedAt = ref(Date.now());
const isDraggingProgress = ref(false);
const dragPosition = ref(0);
let progressTimer: number | null = null;
let progressClockTimer: number | null = null;
let timelineMissCount = 0;
let isTimelineRequestInFlight = false;

const progressEnd = computed(() => musicTimeline.value.end);
// 拖动时显示临时位置；播放时在两次 SMTC 同步之间按本地时钟平滑推进。
const progressPosition = computed(() => {
    if (isDraggingProgress.value) return dragPosition.value;
    const elapsed = isPlaying.value ? Math.max(0, timelineClock.value - timelineSyncedAt.value) : 0;
    return Math.min(progressEnd.value, musicTimeline.value.position + elapsed);
});
const progressPercent = computed(() => progressEnd.value > 0
    ? Math.min(100, Math.max(0, (progressPosition.value / progressEnd.value) * 100))
    : 0);

const resetTimeline = (status: TimelineStatus = 'idle') => {
    isDraggingProgress.value = false;
    dragPosition.value = 0;
    musicTimeline.value = { position: 0, end: 0, canSeek: false };
    timelineStatus.value = status;
    timelineMissCount = 0;
    timelineSyncedAt.value = Date.now();
    timelineClock.value = timelineSyncedAt.value;
};

// 拉取播放进度（仅展开态且灵动岛可见时执行，折叠/隐藏时暂停）。
const fetchTimeline = async () => {
    if (!isMusicExpanded.value || !isIslandVisible.value || isDraggingProgress.value || isTimelineRequestInFlight) return;
    isTimelineRequestInFlight = true;
    try {
        const res = await invoke<MusicTimelineResponse | null>('get_music_timeline');
        if (res && res.end_ms > 0) {
            const now = Date.now();
            musicTimeline.value = {
                position: Math.min(res.position_ms, res.end_ms),
                end: res.end_ms,
                canSeek: res.can_seek,
            };
            timelineSyncedAt.value = now;
            timelineClock.value = now;
            timelineMissCount = 0;
            timelineStatus.value = 'available';
            // 同步 歌词同步时钟（拖动定位后歌词立即跟进，无需等下一个轮询周期）
            lyricTimelinePos.value = res.position_ms;
            lyricTimelineEnd.value = res.end_ms;
            lyricSyncedAt.value = now;
            // 校准后按新位置重排歌词调度（处理 seek / 时钟漂移）
            advanceLyric();
        } else {
            // SMTC 偶尔会在切歌时短暂返回空 Timeline，连续失败后才判定不可用。
            timelineMissCount++;
            if (timelineMissCount >= 3) timelineStatus.value = 'unavailable';
        }
    } catch (e) {
        timelineMissCount++;
        if (timelineMissCount >= 3) timelineStatus.value = 'unavailable';
    } finally {
        isTimelineRequestInFlight = false;
    }
};

const startProgressTimer = () => {
    stopProgressTimer();
    if (timelineStatus.value !== 'available') timelineStatus.value = 'loading';
    fetchTimeline();
    progressTimer = setInterval(fetchTimeline, 1000) as unknown as number;
    progressClockTimer = setInterval(() => {
        timelineClock.value = Date.now();
    }, 250) as unknown as number;
};

const stopProgressTimer = () => {
    if (progressTimer) {
        clearInterval(progressTimer);
        progressTimer = null;
    }
    if (progressClockTimer) {
        clearInterval(progressClockTimer);
        progressClockTimer = null;
    }
};

// 拖动定位：将指针横坐标换算为播放位置（进度条 DOM 在 IslandMusic 内，经 expose 读取）
const updateDragPosition = (e: PointerEvent) => {
    const bar = islandMusicRef.value?.progressBarRef ?? null;
    if (!bar || progressEnd.value === 0) return;
    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    dragPosition.value = Math.round(ratio * progressEnd.value);
};

const onProgressPointerDown = (e: PointerEvent) => {
    if (progressEnd.value === 0 || !musicTimeline.value.canSeek) return;
    e.preventDefault();
    isDraggingProgress.value = true;
    dragPosition.value = progressPosition.value;
    try { (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId); } catch (_) {}
    updateDragPosition(e);
};

const onProgressPointerMove = (e: PointerEvent) => {
    if (!isDraggingProgress.value) return;
    updateDragPosition(e);
};

const onProgressPointerCancel = (e?: PointerEvent) => {
    if (!isDraggingProgress.value) return;
    isDraggingProgress.value = false;
    dragPosition.value = musicTimeline.value.position;
    if (e) {
        try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch (_) {}
    }
};

const onProgressPointerUp = async (e: PointerEvent) => {
    if (!isDraggingProgress.value) return;
    const finalPos = dragPosition.value;
    const previousPosition = musicTimeline.value.position;
    isDraggingProgress.value = false;
    try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch (_) {}

    const now = Date.now();
    musicTimeline.value = { ...musicTimeline.value, position: finalPos };
    timelineSyncedAt.value = now;
    timelineClock.value = now;
    try {
        await invoke('seek_music', { positionMs: finalPos });
        await fetchTimeline();
    } catch (err) {
        musicTimeline.value = { ...musicTimeline.value, position: previousPosition };
        timelineSyncedAt.value = Date.now();
        timelineClock.value = timelineSyncedAt.value;
        console.error('拖动定位失败:', err);
        await fetchTimeline();
    }
};

// 展开态 + 灵动岛可见 时启动进度条轮询；折叠/隐藏时暂停并清空数据
watch([isMusicExpanded, isIslandVisible], () => {
    if (isMusicExpanded.value && isIslandVisible.value) {
        startProgressTimer();
    } else {
        stopProgressTimer();
        if (!isMusicExpanded.value) {
            // 折叠时清空，避免下次展开瞬间残留旧歌曲进度。
            resetTimeline();
        }
    }
});

watch(currentTrackInfo, () => {
    if (!isMusicExpanded.value) return;
    resetTimeline('loading');
    fetchTimeline();
});

// ===== 来源判定：视频类来源（B站/浏览器视频/PotPlayer视频）只显示标题、不拉歌词 =====
// PotPlayer 无歌手元数据时，后端会把歌手占位为 "potplayer"：此时不做歌词匹配，标题常驻显示
const isPotplayerSource = computed(() => currentArtistName.value === 'potplayer');

// 视频类播放源：B站/PotPlayer 恒为视频类；浏览器拿到封面或歌词才算音乐，否则视为视频
const isVideoLikeSource = computed(() => {
    if (isPotplayerSource.value) return true;
    if (currentAppIdStr.value.includes('bilibili')) return true;
    if (currentIsBrowser.value) return !isBrowserMusic.value;
    return false;
});

// 折叠态兜底文本：音乐显示"标题 - 歌手"，视频类只显示标题
const collapsedTrackText = computed(() =>
    isVideoLikeSource.value ? currentSongName.value : currentTrackInfo.value
);

// ===== 网络歌词：LRC 解析 + 同步显示（折叠态单行） =====
// 歌词延迟（秒）：正值表示歌词整体延后显示
const nsdLyricDelay = ref(Number(getSettingRaw(NSD_LYRIC_DELAY) || '0'));

// ===== 歌词 composable 接入（LRC 解析 + 同步时钟，逻辑从本组件拆出） =====
const {
    currentLyricText, expandedLyricText,
    lyricTimelinePos, lyricTimelineEnd, lyricSyncedAt,
    fetchLyricsForCurrentTrack, resetLyricState, advanceLyric, stopLyricClock,
} = useLyrics({
    isPlaying, displayMusic, isVideoLikeSource,
    currentIsBrowser, currentSongName, currentArtistName,
    nsdLyricDelay, isMusicExpanded,
    onBrowserMusicDetected: () => { isBrowserMusic.value = true; },
});
resetLyricStateImpl = resetLyricState;
fetchLyricsImpl = fetchLyricsForCurrentTrack;
// 展开态副标题位："歌手 - 歌名"
const expandedSubText = computed(() => `${currentArtistName.value} - ${currentSongName.value}`);

// ===== 沉浸模式（coverglass）：显示条件与背景层样式 =====
// 只要媒体活跃，背景就一直存在；消息通知期间也保持，避免临时回落成黑色
const showCoverglassBg = computed(() =>
    islandTheme.value === 'coverglass' &&
    isMusicCtlEnabled.value &&
    !!blurredCoverUrl.value
);

// 沉浸背景层：智能规避黑边与遮挡
const coverglassStyle = computed<CSSProperties>(() => {
    if (isGlowBorderEnabled.value) {
        // 流光边框开启时：往内缩进 2px 给边框让路，并匹配内层圆角
        const innerRadius = isExpandedSize.value ? '22px' : (borderRadius.value === 12 ? '10px' : '98px');
        return { top: '2px', left: '2px', right: '2px', bottom: '2px', borderRadius: innerRadius };
    }
    // 流光边框关闭时：无死角铺满整个灵动岛，并匹配外层大圆角
    return {
        top: '0', left: '0', right: '0', bottom: '0',
        borderRadius: isExpandedSize.value ? '24px' : (borderRadius.value === 12 ? '12px' : '100px'),
    };
});

// ===== 音乐岛子组件（IslandMusic）实例引用 =====
// 文本溢出滚动测量（折叠/展开态）已随 IslandMusic 内聚；进度条 DOM 通过其 expose 暴露，
// 拖动定位逻辑（updateDragPosition / onProgressPointer*）经 islandMusicRef 读取几何信息
const islandMusicRef = ref<InstanceType<typeof IslandMusic> | null>(null);

let musicFallbackTimer: number | undefined;

// 网络状态灯由后端 network-status 事件驱动（见 onMounted 中的 listen）

// 调整窗口位置到正确位置
const adjustWindowPosition = async () => {
    try {
        const appWindow = getCurrentWindow();
        await new Promise((resolve) => setTimeout(resolve, 150));
        const monitor = await currentMonitor();

        if (monitor) {
            const scaleFactor = window.devicePixelRatio;

            const WINDOW_INIT_WIDTH = currentWidth.value;   // 默认 260
            const WINDOW_INIT_HEIGHT = currentHeight.value; // 默认 42
            await appWindow.setSize(new PhysicalSize(Math.ceil(WINDOW_INIT_WIDTH * scaleFactor), Math.ceil(WINDOW_INIT_HEIGHT * scaleFactor)));

            const monitorWidthPhysical = monitor.size.width;
            const monitorLeftPhysical = monitor.position.x;
            const monitorTopPhysical = monitor.position.y;

            // 2. 重新获取设定后的真实物理尺寸，用于精准居中
            const windowSize = await appWindow.innerSize();
            const windowWidthPhysical = windowSize.width;

            const x = monitorLeftPhysical + (monitorWidthPhysical - windowWidthPhysical) / 2;
            const y = monitorTopPhysical + (12 * scaleFactor);

            await appWindow.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
        }
    } catch (error) {
        console.error('调整窗口位置失败:', error);
    } finally {
        try {
            await getCurrentWindow().show();
        } catch (e) {
            console.error(e);
        }
    }
};

// 核心动画实现：基于你的 AE 公式转化
const onEnter = (el: Element, done: () => void) => {
    const HTMLElement = el as HTMLElement;
    HTMLElement.style.transformOrigin = 'center top'; // 类似苹果灵动岛从顶部展开
    let start = performance.now();

    const stiff = springStyle.value === 'stiff';
    const freq = stiff ? 3.2 : 2.0;
    const decay = stiff ? 18 : 10.5;
    const duration = stiff ? 350 : 600;

    const animate = (time: number) => {
        let t = (time - start) / 1000;
        let progress = (time - start) / duration;

        // 数学方程：1 - cos(2πft) * e^(-dt)
        let scale = 1 - Math.cos(freq * t * 2 * Math.PI) * Math.exp(-decay * t);
        let opacity = Math.min(1, progress * 4); // 快速淡入

        HTMLElement.style.transform = `scale(${scale})`;
        HTMLElement.style.opacity = opacity.toString();

        if (progress < 1) {
            requestAnimationFrame(animate);
        } else {
            // 重置为最终干净的状态
            HTMLElement.style.transform = `scale(1)`;
            HTMLElement.style.opacity = '1';
            done();
        }
    };
    requestAnimationFrame(animate);
};

const onLeave = (el: Element, done: () => void) => {
    const HTMLElement = el as HTMLElement;
    HTMLElement.style.transformOrigin = 'center top';
    let start = performance.now();

    const duration = 300; // 收起动画通常更干脆、更快

    const animate = (time: number) => {
        let progress = (time - start) / duration;

        // 离开动画：快速平滑回缩
        // 使用 easing 曲线或简化的衰减
        let scale = 1 - Math.pow(progress, 3); // 快速内缩
        let opacity = 1 - progress * 1.5;

        HTMLElement.style.transform = `scale(${Math.max(0, scale)})`;
        HTMLElement.style.opacity = Math.max(0, opacity).toString();

        if (progress < 1) {
            requestAnimationFrame(animate);
        } else {
            done();
            // 等待 DOM 动画播放完成后再隐藏窗口
            getCurrentWindow().hide().catch(console.error);
            // 只有用户主动关闭时才同步状态到控制台，自动隐藏/全屏隐藏不改变开关
            if (!isAutoHiding.value && !isHidingForFullscreen.value) {
                emit('island-status-sync', { visible: false });
            }
            isAutoHiding.value = false;
            isHidingForFullscreen.value = false;
        }
    };
    requestAnimationFrame(animate);
};

// 位置持久化（锁定时保存，启动时恢复）
const saveIslandPosition = async () => {
    try {
        const appWindow = getCurrentWindow();
        const pos = await appWindow.outerPosition();
        setSettingRaw(NSD_ISLAND_POSITION, JSON.stringify({ x: pos.x, y: pos.y }));
    } catch (e) {
        console.error('保存位置失败:', e);
    }
};

const restoreIslandPosition = async (): Promise<boolean> => {
    try {
        const saved = getSettingRaw(NSD_ISLAND_POSITION);
        if (saved) {
            const { x, y } = JSON.parse(saved);
            await getCurrentWindow().setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
            return true;
        }
    } catch (e) {
        console.error('恢复位置失败:', e);
    }
    return false;
};

// ===== 右键菜单 composable 接入（打开设置/流光边框/重置位置/重置宽度/锁定/关闭，逻辑从本组件拆出） =====
const { isMenuOpen, handleRightClick } = useIslandContextMenu({
    isIslandVisible, isMusicExpanded, isMusicExpanding, isMsgActive, displaySysToast,
    isGlowBorderEnabled, isPinnedToTaskbar, isPositionLocked, currentWidth,
    showToast, adjustWindowPosition, saveIslandPosition, saveIslandWidth,
    getBaseSize, animateIslandSize,
});

const onInnerEnter = (el: Element, done: () => void) => {
    const htmlEl = el as HTMLElement;
    let start = performance.now();

    // 统一使用简单的渐变淡入 (200毫秒)
    const duration = 180;
    htmlEl.style.transformOrigin = 'center';
    htmlEl.style.opacity = '0';
    htmlEl.style.transform = 'none'; // 确保没有位移

    const animate = (time: number) => {
        let progress = (time - start) / duration;
        htmlEl.style.opacity = Math.min(1, progress).toString();

        if (progress < 1) {
            requestAnimationFrame(animate);
        } else {
            htmlEl.style.opacity = '1';
            done();
        }
    };
    requestAnimationFrame(animate);
};

const onInnerLeave = (el: Element, done: () => void) => {
    const htmlEl = el as HTMLElement;
    let start = performance.now();
    const duration = 140;

    const animate = (time: number) => {
        let progress = (time - start) / duration;
        let opacity = 1 - progress;

        htmlEl.style.opacity = Math.max(0, opacity).toString();

        if (progress < 1) {
            requestAnimationFrame(animate);
        } else {
            done();
        }
    };
    requestAnimationFrame(animate);
};

// 动画锁与等待队列标志
let isAnimationLocked = false;

// 音乐控制器自动收缩方法
const collapseMusic = () => {
    if (!isMusicExpanded.value && !isMusicExpanding.value) return;

    // 【核心逻辑】：如果正在猛烈展开中，绝对不打断！把收缩请求挂起，等它展开完自动执行。
    if (isAnimationLocked) {
        isPendingCollapse.value = true;
        return;
    }

    isMusicExpanded.value = false;
    isMusicExpanding.value = false;
    isPendingCollapse.value = false; // 清除队列

    if (musicExpandAnimTimer) {
        clearTimeout(musicExpandAnimTimer);
        musicExpandAnimTimer = null;
    }

    // 折叠时恢复用户自定义的宽度，而不是使用默认宽度
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
};

// 直接复位音乐展开态（不播放收缩动画、不恢复岛尺寸）：
// 供切换到实时活动展开态时调用，最终岛尺寸由实时活动自己的展开动画决定。
// 不能复用 collapseMusic：它受 isAnimationLocked 保护会把收缩请求挂起为
// isPendingCollapse，稍后补发时会把实时活动刚展开的尺寸覆盖回小岛尺寸
const resetMusicExpandedState = () => {
    if (!isMusicExpanded.value && !isMusicExpanding.value) return;
    if (musicExpandAnimTimer) {
        clearTimeout(musicExpandAnimTimer);
        musicExpandAnimTimer = null;
    }
    isMusicExpanded.value = false;
    isMusicExpanding.value = false;
    isPendingCollapse.value = false;
    // 若正处于 expandMusic 的展开周期，其解锁回调可能随上面的定时器一起被清除，
    // 这里必须手动解锁，否则后续 collapseMusic 只会挂起、永远不再执行
    isAnimationLocked = false;
};

// force_window_topmost：窗口 blur 事件驱动置顶（替代原 speedTimer 中每 800ms 轮询）
const handleForceTopmost = () => {
    if (isPinnedToTaskbar.value && isIslandVisible.value && !isMenuOpen.value && !isCustomDragging.value) {
        invoke('force_window_topmost').catch(() => { });
    }
};

// 音乐控制器点击展开方法
const expandMusic = (e: MouseEvent) => {
    if (Math.abs(e.clientX - mouseDownX.value) > 5 || Math.abs(e.clientY - mouseDownY.value) > 5) return;
    if ((e.target as HTMLElement).closest('.ctl-btn')) return;

    if (isMusicExpanded.value || isMusicExpanding.value) return;

    isMusicExpanding.value = true;
    isPendingCollapse.value = false;  // 重置待办任务
    isAnimationLocked = true;   // 上锁！宣布进入神圣不可侵犯的展开周期

    // 1. 弹性按压动画 (先微微变小)
    animateIslandSize(245, 38);

    // 2. 延迟 120 毫秒后，打断缩小，直接猛烈展开
    musicExpandAnimTimer = window.setTimeout(() => {
        isMusicExpanded.value = true;
        isMusicExpanding.value = false;
        animateIslandSize(musicExpandedWidth.value, 150);

        // 3. 根据 Rust 端的弹簧衰减频率，约 400ms 后动画彻底结束，此时解锁
        setTimeout(() => {
            isAnimationLocked = false;

            // 检查：如果在展开的这 520ms 里，用户鼠标已经移走了，那就立刻补发收缩命令。
            if (isPendingCollapse) {
                isPendingCollapse.value = false;
                collapseMusic();
            }
        }, 400);
    }, 120);
};

// 鼠标离开/进入的自动折叠与自动隐藏处理已随 useIslandAutoHide 拆出

watch(displayMusic, (newVal: boolean) => {
    if (!newVal) {
        collapseMusic(); // 一旦音乐岛被隐藏（不管是因为轮换还是手动关了），立刻收缩
    }
});

// 视频类/浏览器来源的应用 logo（import 引用，打包后路径才会被 Vite 正确重写）
import bilibiliLogo from '../assets/bilibili-logo.png';
import edgeLogo from '../assets/edge-logo.png';
import chromeLogo from '../assets/chrome-logo.png';
import potplayerLogo from '../assets/potplayer-logo.jpg';
// 统一保存 Tauri listen 返回的 unlisten 函数，组件卸载时清理，防止事件订阅残留
const unlistenFns: Array<() => void> = [];

onMounted(async () => {
    // 启动时应用个性化缩放与置顶
    applyAppScale(appScale.value);
    await applyAlwaysOnTop(isAlwaysOnTop.value);

    // widget 可能在主面板未创建或省内存销毁后独立运行，需自行恢复目标播放器
    await invoke('set_target_player', {
        player: getSettingRaw(NSD_TARGET_PLAYER) || 'netease',
    }).catch(() => {});

    // 启动时从 localStorage 读取实时活动优先级配置（与 LiveActive 推送双保险）
    loadActivityConfigFromStorage();

    window.addEventListener('blur', collapseMusic);

    // force_window_topmost：窗口 blur 事件驱动置顶（函数定义已提升到模块级，供 onUnmounted 清理）
    window.addEventListener('blur', handleForceTopmost);

    document.addEventListener('contextmenu', (e) => {
        e.preventDefault();
    }, { capture: true }); // 使用捕获阶段，确保先于 Tauri 底层拦截

    // 音乐控制器状态监听器
    await listen<{ enabled: boolean }>('control-music-ctl', (event) => {
        const isEnabled = event.payload.enabled;
        isMusicCtlEnabled.value = isEnabled;

        if (isEnabled) {
            // 判断是不是"首次"（本地有没有存过流光边框的数据）
            if (getSettingRaw(NSD_GLOW_BORDER) === null) {
                isGlowBorderEnabled.value = true; // 自动开启流光边框
                setSettingRaw(NSD_GLOW_BORDER, 'true'); // 存入记忆，以后就不算"首次"了
            }

            showInfo.value = false;
            musicBoxKey.value++;

            // 音乐控制器开启时，如果没有音乐播放，延迟隐藏灵动岛
            // scheduleAutoHide 内部会校验全部条件
            scheduleAutoHide();
        }
    });

    // 监听系统动态感知（sysmsg）结构化事件：后端统一推送，前端按需弹通知
    await listen<{ kind: string; level: string; text: string }>('sysmsg-event', (event) => {
        if (isSysmsgEnabled.value) {
            showSysmsgToast(event.payload);
        }
    });

    // 后端 NetworkMonitor 推送状态灯（good / warning / error）
    await listen<{ status: string }>('network-status', (event) => {
        const s = event.payload?.status;
        if (s === 'good' || s === 'warning' || s === 'error') {
            networkStatus.value = s;
        }
    });

    // 跨窗口同步动态感知总开关（网络 toast 门控已下沉后端）
    await listen<{ enabled: boolean }>('control-sysmsg-config', (event) => {
        isSysmsgEnabled.value = event.payload.enabled;
    });

    // 监听来自控制台的透明度同步指令
    await listen<{ opacity: number }>('control-island-opacity', (event) => {
        islandOpacity.value = event.payload.opacity;
    });

    // 监听来自控制台的主题同步指令
    await listen<{ theme: string }>('control-island-theme', (event) => {
        islandTheme.value = event.payload.theme;
    });

    // 监听来自控制台的歌词延迟同步指令（正值=歌词延后）
    await listen<{ delay: number }>('control-lyric-delay', (event) => {
        if (typeof event.payload.delay === 'number' && Number.isFinite(event.payload.delay)) {
            nsdLyricDelay.value = event.payload.delay;
        }
    });

    // 监听个性化中心的打包设置同步
    await listen<{
        springStyle?: 'stiff' | 'bouncy';
        borderRadius?: number;
        isAlwaysOnTop?: boolean;
        baseWidth?: number;
        musicBaseWidth?: number;
        musicExpandedWidth?: number;
        msgExpandedWidth?: number;
        appScale?: number;
    }>('sync-dynamic-settings', async (event) => {
        const p = event.payload;
        if (p.springStyle === 'stiff' || p.springStyle === 'bouncy') {
            springStyle.value = p.springStyle;
        }
        if (typeof p.borderRadius === 'number') {
            borderRadius.value = p.borderRadius;
        }
        if (typeof p.baseWidth === 'number') {
            baseWidth.value = p.baseWidth;
        }
        if (typeof p.musicBaseWidth === 'number') {
            musicBaseWidth.value = p.musicBaseWidth;
        }
        if (typeof p.musicExpandedWidth === 'number') {
            musicExpandedWidth.value = p.musicExpandedWidth;
        }
        if (typeof p.msgExpandedWidth === 'number') {
            msgExpandedWidth.value = p.msgExpandedWidth;
        }
        if (typeof p.appScale === 'number') {
            appScale.value = p.appScale;
            applyAppScale(p.appScale);
        }
        if (typeof p.isAlwaysOnTop === 'boolean') {
            isAlwaysOnTop.value = p.isAlwaysOnTop;
            await applyAlwaysOnTop(p.isAlwaysOnTop);
        }
        refreshIslandSizeIfIdle();
    });

    // 监听来自控制台的频谱颜色同步指令（模式 + 自定义色）
    await listen<{ mode: string; color: string }>('control-spectrum-color', async (event) => {
        spectrumColorMode.value = event.payload.mode;
        spectrumCustomColor.value = event.payload.color;
        // 切到 album 模式且封面已加载时，立即重新取色
        if (event.payload.mode === 'album' && coverUrl.value) {
            albumDominantColor.value = await extractAlbumColor(coverUrl.value);
        }
    });

    // 监听来自控制台的清理封面缓存指令
    await listen('clear-cover-cache', async () => {
        try {
            await clearCoverCacheAndRefresh();
        } catch (e) {
            console.error('清理封面缓存失败', e);
        }
    });

    // 监听置于任务栏开关
    await listen<{ enabled: boolean }>('control-pin-taskbar', async (event) => {
        isPinnedToTaskbar.value = event.payload.enabled;
        if (isPinnedToTaskbar.value) {
            await snapToBottomLeft(); // 开启时：飞到左下角
        } else {
            await adjustWindowPosition(); // 关闭时：等同于点"重置位置"，飞回顶部居中
        }
        // 如果位置已锁定，模式切换后重新保存新位置（避免下次启动恢复到过期坐标）
        if (isPositionLocked.value) {
            await saveIslandPosition();
        }
    });

    // 监听来自设置面板的位置锁定信号
    await listen<{ locked: boolean }>('control-position-lock', async (event) => {
        isPositionLocked.value = event.payload.locked;
        // 锁定时保存当前位置，以便下次启动恢复
        if (isPositionLocked.value) {
            await saveIslandPosition();
        }
    });

    // 监听消息模式开关
    await listen<{ enabled: boolean }>('control-msg-mode', async (event) => {
        isMsgModeEnabled.value = event.payload.enabled;
        if (isMsgModeEnabled.value && !isMsgActive.value) {
            // 开启消息模式且当前无消息时，延迟隐藏
            // 统一交给 scheduleAutoHide 守卫判断（仅在音乐控制器开启+无音乐播放时才隐藏）
            scheduleAutoHide();
        } else if (!isMsgModeEnabled.value) {
            // 如果关闭了消息模式，立刻恢复显示
            await getCurrentWindow().show();
            isIslandVisible.value = true;

            // 通知控制台恢复开关状态，让主面板的开关同步变绿（开启）
            await emit('island-status-sync', { visible: true });
        }
    });

    // 监听轮换模式开关
    await listen<{ enabled: boolean }>('control-rotation-mode', (event) => {
        isRotationEnabled.value = event.payload.enabled;
        if (isRotationEnabled.value) {
            startRotation();
        } else {
            stopRotation();
            currentRotIndex.value = 0; // 关闭时重置回网速
        }
    });

    // 监听自动隐藏设置
    await listen<{ enabled: boolean, delay: number }>('control-auto-hide', (event) => {
        isAutoHideEnabled.value = event.payload.enabled;
        autoHideDelay.value = event.payload.delay;
        setSettingRaw(NSD_AUTO_HIDE_ENABLED, String(isAutoHideEnabled.value));
        setSettingRaw(NSD_AUTO_HIDE_DELAY, String(autoHideDelay.value));
    });

    // 监听全屏自动隐藏设置
    await listen<{ enabled: boolean }>('control-autohide-fs', (event) => {
        isAutoHideFullscreen.value = event.payload.enabled;
    });

    // fullscreen-changed 事件监听已随 useIslandAutoHide 拆出（自带挂载与清理）

    // 打印队列：订阅后先读取快照，避免后端启动 emit 早于前端窗口订阅。
    unlistenFns.push(await listen<PrintQueueState>('print-queue-tick', (event) => {
        const state = event.payload;
        printJobs.value = Array.isArray(state?.jobs) ? state.jobs : [];
        defaultPrinter.value = state?.defaultPrinter || '';
        if (!printJobs.value.length && isPrintQueueExpanded.value) {
            collapsePrintQueue();
            if (expandedRtId.value === 'printer') {
                revertRealtime();
            }
        }
    }));
    try {
        const state = await invoke<PrintQueueState>('get_printer_state');
        printJobs.value = Array.isArray(state?.jobs) ? state.jobs : [];
        defaultPrinter.value = state?.defaultPrinter || '';
    } catch (_e) {
        // 后端打印模块不可用时保持空队列。
    }

    // 监听后端番茄钟 tick 事件
    await listen<any>('pomodoro-tick', async (event) => {
        const p = event.payload;
        if (p.active === false) {
            // 番茄钟结束 → 隐藏
            const wasExpanded = expandedRtId.value === 'pomodoro';
            isPomodoroVisible.value = false;
            isPomodoroExpanded.value = false;
            setSettingRaw(NSD_POMODORO_VISIBLE, 'false');
            if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
                const { h } = getBaseSize();
                const savedWidth = restoreIslandWidth();
                const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                animateIslandSize(targetWidth, h);
            }
            // 结束态自动回退：若展开的正是 pomodoro，触发回退（恢复 previousContext）
            if (wasExpanded) {
                revertRealtime();
            }
            return;
        }
        // 更新显示状态
        pomodoroRemainingSecs.value = p.remaining_secs;
        pomodoroPhase.value = p.phase;
        pomodoroRemainingCycles.value = p.remaining_cycles;
        // 确保可见
        if (!isPomodoroVisible.value) {
            isPomodoroVisible.value = true;
            setSettingRaw(NSD_POMODORO_VISIBLE, 'true');
        }
        // 暂停时不调整展开状态，但不影响显示
    });

    // 监听番茄钟阶段切换事件（用于显示 toast 提示）
    await listen<any>('pomodoro-phase-change', async (event) => {
        const p = event.payload;
        showToast(p.message ?? (p.phase === 'break' ? '专注结束，休息一下吧！' : '休息结束，继续专注！'), 'app');
    });

    // 监听番茄钟完成事件
    await listen<any>('pomodoro-complete', async () => {
        showToast('所有番茄钟已完成！🎉', 'app');
    });

    // 监听倒计时 tick 事件
    await listen<any>('countdown-tick', async (event) => {
        const p = event.payload;
        if (p.active === false && p.phase === 'idle') {
            const wasExpanded = expandedRtId.value === 'countdown';
            isCountdownVisible.value = false;
            isCountdownExpanded.value = false;
            isCountdownFinished.value = false;
            cdPaused.value = false;
            setSettingRaw(NSD_COUNTDOWN_VISIBLE, 'false');
            if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
                const { h } = getBaseSize();
                const savedWidth = restoreIslandWidth();
                const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                animateIslandSize(targetWidth, h);
            }
            // 结束态自动回退：若展开的正是 countdown，触发回退
            if (wasExpanded) {
                revertRealtime();
            }
            return;
        }
        countdownRemainingSecs.value = p.remaining_secs;
        cdPaused.value = p.paused || false;
        isCountdownFinished.value = p.phase === 'finished';
        if (!isCountdownVisible.value) {
            isCountdownVisible.value = true;
            setSettingRaw(NSD_COUNTDOWN_VISIBLE, 'true');
        }
    });

    // 监听倒计时完成事件
    await listen<any>('countdown-complete', async () => {
        isCountdownFinished.value = true;
        showToast('⏰ 倒计时结束', 'app');
    });

    // 监听日程同步 tick 事件（F：系统日历 + 手动提醒的未来 24h 列表，列表变化或每 30 秒推送）
    await listen<{ upcoming: CalendarEventInfo[] }>('calendar-tick', (event) => {
        calUpcoming.value = Array.isArray(event.payload?.upcoming) ? event.payload.upcoming : [];
    });
    // 启动恢复：拉取一次日程快照（窗口重建后无需等待下一个 30s tick）
    try {
        const state: any = await invoke('calendar_get_state');
        calUpcoming.value = Array.isArray(state?.upcoming) ? state.upcoming : [];
    } catch (_e) {}

    // 监听健康提醒 tick 事件
    await listen<any>('health-reminder-tick', async (event) => {
        const p = event.payload;
        const wasAlerting = isHealthAlerting.value;
        // 处理久坐提醒
        if (p.sitting && p.sitting.alerting) {
            isHealthAlerting.value = true;
            healthAlertLabel.value = p.sitting.label || '该起来走走了';
            healthAlertType.value = 'sitting';
        }
        // 处理喝水提醒（如果久坐也在提醒，优先显示久坐，但喝水也设置 alerting）
        if (p.water && p.water.alerting && !p.sitting?.alerting) {
            isHealthAlerting.value = true;
            healthAlertLabel.value = p.water.label || '该喝水了';
            healthAlertType.value = 'water';
        }
        // 两个都未提醒时关闭
        if ((!p.sitting || !p.sitting.alerting) && (!p.water || !p.water.alerting)) {
            isHealthAlerting.value = false;
            healthAlertLabel.value = '';
            // 结束态自动回退：若展开的正是 health，触发回退
            if (wasAlerting && expandedRtId.value === 'health') {
                revertRealtime();
            }
        } else if (!wasAlerting) {
            // 新进入 alerting：标记 expandedRtId = 'health'（让小图标隐藏，避免重复）
            // previousContext 不需快照（health 是后端驱动的展开，用户未点击）
            expandedRtId.value = 'health';
        }
    });

    // 监听实时活动控制台指令（非番茄钟活动）

    // 监听自动折叠设置
    await listen<{ enabled: boolean, delay: number }>('control-auto-collapse', (event) => {
        isAutoCollapseEnabled.value = event.payload.enabled;
        autoCollapseDelay.value = event.payload.delay;
        setSettingRaw(NSD_AUTO_COLLAPSE_ENABLED, String(isAutoCollapseEnabled.value));
        setSettingRaw(NSD_AUTO_COLLAPSE_DELAY, String(autoCollapseDelay.value));
    });

    // 启动时如果开了轮换，就跑起来
    if (isRotationEnabled.value) {
        startRotation();
    }

    // 初始化位置追踪
    const appWindow = getCurrentWindow();
    try {
        await appWindow.innerPosition();
    } catch (e) { }

    // 在启动调整位置前，根据当前的实际状态，校准初始宽高
    const { w, h } = getBaseSize();
    // 优先恢复用户自定义的宽度
    const savedWidth = restoreIslandWidth();
    currentWidth.value = savedWidth !== null ? savedWidth : w;
    currentHeight.value = h;

    // 立即设置窗口大小，确保宽度恢复生效
    try {
        const appWindow = getCurrentWindow();
        const scaleFactor = window.devicePixelRatio;
        await appWindow.setSize(new PhysicalSize(Math.ceil(currentWidth.value * scaleFactor), Math.ceil(currentHeight.value * scaleFactor)));
    } catch (error) {
        console.error('设置初始窗口大小失败:', error);
    }

    // 根据本地记录决定启动时的定位方式
    if (isPositionLocked.value) {
        // 已锁定位置：尝试恢复上次保存的坐标
        const restored = await restoreIslandPosition();
        if (!restored) {
            // 没有保存过位置，回退到默认定位
            if (isPinnedToTaskbar.value) {
                await snapToBottomLeft();
            } else {
                await adjustWindowPosition();
            }
        }
    } else {
        // 未锁定：使用默认定位
        if (isPinnedToTaskbar.value) {
            await snapToBottomLeft();
        } else {
            await adjustWindowPosition();
        }
    }

    // 先显示透明的 Tauri 窗口，再触发 Vue 的灵动岛入场弹簧动画
    // 如果没开消息模式，才在启动时直接显示灵动岛
    if (!isMsgModeEnabled.value) {
        await getCurrentWindow().show();
        isIslandVisible.value = true;
    }

    // 监听来自 LiveActive 的硬件监控开关（跨窗口事件，统一由 NSD_HW_ENABLED 驱动）
    await listen<any>('control-hardware-mon', (event) => {
        const p = event.payload || {};
        // 优先使用事件直接携带的完整配置，缺失时回退到 localStorage，保证健壮性
        if (typeof p.enabled === 'boolean') {
            hwEnabled.value = p.enabled;
            setSettingRaw(NSD_HW_ENABLED, String(p.enabled));
        }
        if (typeof p.mode === 'string') {
            hwMode.value = p.mode;
            setSettingRaw(NSD_HW_MODE, p.mode);
        }
        if (typeof p.defaultMetric === 'string') {
            hwDefaultMetric.value = p.defaultMetric;
            setSettingRaw(NSD_HW_DEFAULT_METRIC, p.defaultMetric);
        }
        // E：双圆环外/内环指标随事件同步（存 localStorage 的值已在 composable 做合法性校验兜底）
        if (typeof p.ringOuter === 'string') {
            hwRingOuter.value = p.ringOuter;
            setSettingRaw(NSD_HW_RING_OUTER, p.ringOuter);
        }
        if (typeof p.ringInner === 'string') {
            hwRingInner.value = p.ringInner;
            setSettingRaw(NSD_HW_RING_INNER, p.ringInner);
        }
        // 控制轮换定时器
        if (hwEnabled.value && hwMode.value === 'rotation') {
            startHwRotation();
        } else {
            stopHwRotation();
        }
    });

    // 监听来自 LiveActive 的实时活动配置（多活动并行轮换：enabled + priority）
    unlistenFns.push(await listen<Record<string, { enabled: boolean; priority: number }>>('control-activity-config', (event) => {
        const p = event.payload || {};
        // 合并到 activityConfig（事件优先，缺失的 id 保留原值）
        const merged: Record<string, { enabled: boolean; priority: number }> = { ...activityConfig.value };
        for (const id of RT_IDS) {
            const entry = p[id];
            if (entry && typeof entry.enabled === 'boolean' && typeof entry.priority === 'number') {
                merged[id] = { enabled: entry.enabled, priority: entry.priority };
            }
        }
        activityConfig.value = merged;
    }));

    // 监听后端推送的 monitor-stats 事件（硬件 + 网速统一）
    unlistenFns.push(await listen<any>('monitor-stats', (event) => {
        const p = event.payload;
        if (typeof p.cpu_pct === 'number') hwCpuPct.value = p.cpu_pct;
        if (typeof p.mem_pct === 'number') hwMemPct.value = p.mem_pct;
        // E：电池/磁盘扩展数据源（battery_pct=255 表示无电池，由指标工具置灰兜底）
        if (typeof p.battery_pct === 'number') hwBatteryPct.value = p.battery_pct;
        if (typeof p.disk_pct === 'number') hwDiskPct.value = p.disk_pct;
        if (typeof p.download_speed === 'number') {
            downloadSpeed.value = formatSpeed(p.download_speed);
        }
        if (typeof p.upload_speed === 'number') {
            uploadSpeed.value = formatSpeed(p.upload_speed);
        }
        if (typeof p.download_bytes === 'number' && typeof p.upload_bytes === 'number') {
            // 高流量指示（原 fetchSpeedStats 轮询中的逻辑，现并入推送监听器）
            const rxDiff = p.download_speed || 0;
            const txDiff = p.upload_speed || 0;
            const highDown = rxDiff >= 1024 * 1024;
            const highUp = txDiff >= 1024 * 1024;
            isHighDownload.value = highDown;
            isHighUpload.value = highUp;
        }
    }));

    // 启动网速显示轮换定时器（每 5 秒切换上传/下载）
    speedCycleTimer = window.setInterval(() => {
        if (displaySpeed.value) {
            isShowingUpload.value = !isShowingUpload.value;
        }
    }, 5000);

    // 硬件监控后台始终推送 monitor-stats，前端不再控制 emit 开关

    // 硬件监控轮换模式启动定时器
    if (hwEnabled.value && hwMode.value === 'rotation') {
        startHwRotation();
    }

    // 网速与硬件统一由后端 monitor-stats 推送驱动，前端不再轮询（避免与推送互相覆盖导致 0B/s 跳变）

    // 音乐状态事件驱动：后端会话绑定管理器推送 music-info-changed / music-playback-changed
    unlistenFns.push(await listen<{ song: string; artist: string; playing: boolean; appId: string } | null>('music-info-changed', (event) => {
        const p = event.payload;
        if (p) {
            applyMusicInfo(p.song, p.artist, p.playing, p.appId);
        } else {
            applyNoTrack();
        }
    }));
    unlistenFns.push(await listen<{ playing: boolean }>('music-playback-changed', (event) => {
        const playing = event.payload.playing;
        isPlaying.value = playing;
        // 显示/隐藏调度（对齐 applyMusicInfo 中的调度语义）
        if (displayMusic.value) {
            if (playing && !isIslandVisible.value) {
                getCurrentWindow().show();
                isIslandVisible.value = true;
            } else if (!playing && isIslandVisible.value && !isMouseOver.value) {
                scheduleAutoHide();
            }
        }
        // 进度时钟启停：恢复播放时重置本地时钟基准（暂停期间 elapsed 已冻结，重置避免跳变）
        if (playing) {
            timelineSyncedAt.value = Date.now();
            timelineClock.value = timelineSyncedAt.value;
        }
    }));

    // 启动快照一次 + 45s 低频兜底轮询（浏览器/视频类来源 SMTC 事件经常延迟或不发）
    await syncMusicStatus();
    musicFallbackTimer = window.setInterval(() => {
        if (isMusicCtlEnabled.value || isRotationEnabled.value) {
            syncMusicStatus();
        }
    }, 45000);

    // 监听控制台发来的显隐调度指令
    unlistenFns.push(await listen<{ show: boolean }>('control-island-visibility', async (event) => {
        if (event.payload.show) {
            // 1. 先让透明的 OS 窗口容器显示，此时内部 DOM 因 v-show="false"，视觉上仍是隐形的
            await getCurrentWindow().show();
            await applyAlwaysOnTop(isAlwaysOnTop.value);
            // 2. 给予 40ms 的浏览器渲染帧缓冲，再撕开 Vue 的 v-show 状态，强制触发 enter 动画
            setTimeout(() => {
                isIslandVisible.value = true;
            }, 40);
        } else {
            // 控制台关闭指令 -> 触发常规离开动画
            isIslandVisible.value = false;
        }
    }));

    // 实时监听来自 Rust 底层发来的清透像素流，无缝同步给 Vue 的响应式 DOM 宽高
    unlistenFns.push(await listen<number[]>("island-resize", (event) => {
        const [w, h] = event.payload;
        currentWidth.value = w;
        currentHeight.value = h;
    }));

    // B8: 监听后端推来的频谱数据（替代 50ms setInterval 轮询，显著减少 IPC 调用次数）
    unlistenFns.push(await listen<number[]>("spectrum-data", (event) => {
        const p = event.payload;
        const arr = spectrumData.value;
        if (p && p.length === 5) {
            arr[0] = p[0]; arr[1] = p[1]; arr[2] = p[2]; arr[3] = p[3]; arr[4] = p[4];
            triggerRef(spectrumData);
        }
    }));

    // 消息通知增量事件（替代 5s 轮询）：后端监听线程主动推送，前端入队逐条展示
    unlistenFns.push(await listen<{ items: ToastItem[] }>('notification-event', (e) => {
        for (const it of e.payload.items) msgQueue.value.push(it);
        processMsgQueue();
    }));

    // 权限状态：denied/unavailable 时弹灵动岛 toast 提示（可点击跳设置）
    unlistenFns.push(await listen<AccessStatus>('notification-status', (e) => {
        if (e.payload === 'denied' || e.payload === 'unavailable') {
            showToast(
                e.payload === 'unavailable'
                    ? '系统通知不可用，请在 Windows 设置中开启通知访问'
                    : '未授予通知访问权限，点击前往系统设置开启',
                'notify-permission'
            );
        }
    }));

    // 启动即触发后端监听（若用户开启了消息通知），由后端状态机负责增量推送 + 轮询兜底
    if (getSettingRaw(NSD_MSG_NOTIFY) === 'true') {
        invoke('set_notification_listening', { enabled: true }).catch(() => {});
    }

    // 初始化时的折叠态滚动测量已随 IslandMusic 子组件的 onMounted 处理

    // 恢复番茄钟/倒计时运行状态：查询后端当前状态（逻辑在 useRealtimeActivity 内）
    await restorePomodoroState();
    await restoreCountdownState();
});

onUnmounted(() => {
    albumColorExtractionVersion++;
    if (albumColorAnimationFrame !== null) {
        cancelAnimationFrame(albumColorAnimationFrame);
        albumColorAnimationFrame = null;
    }
    window.removeEventListener('blur', collapseMusic);
    window.removeEventListener('blur', handleForceTopmost);
    // 清理自定义横向拖拽的文档级监听器（逻辑在 useIslandAnimation 内）
    cleanupIslandAnimation();
    stopRotation();
    stopHwRotation();
    if (musicFallbackTimer !== undefined) {
        clearInterval(musicFallbackTimer);
        musicFallbackTimer = undefined;
    }
    stopProgressTimer();
    stopLyricClock();
    // 使进行中的 toast 等待立即失效，避免卸载后继续改状态（逻辑在 useNotifications 内）
    cleanupNotifications();
    // 组件卸载时关闭频谱捕获，避免后端空跑
    invoke('stop_audio_spectrum').catch(() => {});
    if (speedCycleTimer) clearInterval(speedCycleTimer);
    // 清理所有 Tauri 事件订阅，防止卸载后残留监听器累积
    unlistenFns.forEach(fn => { try { fn(); } catch (_) {} });
    unlistenFns.length = 0;
    // 释放封面缓存
    coverCache.clear();
    blurredCoverCache.clear();
});
</script>

<style scoped>
*,
*::before,
*::after {
    box-sizing: border-box;
    border: none !important;
    outline: none !important;
}

:root {
    -webkit-app-region: drag;
}

:global(html),
:global(body) {
    background-color: transparent !important;
    background: transparent !important;
    overflow: hidden;
    margin: 0;
    padding: 0;
    border: none !important;
}

/* 外层包裹层：负责裁切多余的流光 */
.island-container {
    /* 移除 position: absolute; top: 0; */
    margin: 0 auto;
    /* 让它在窗口内水平居中 */
    border-radius: 100px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    user-select: none;
    -webkit-user-select: none;
    overflow: hidden;
    background: transparent;
    transition: background 0.4s ease;
    box-sizing: border-box;
    transform: translateZ(0);
    will-change: width, height, border-radius;
    contain: strict;
}

/* 隐藏在底层的巨大旋转渐变层 */
.rainbow-border-glow {
    position: absolute;
    width: 500px;
    height: 500px;

    /* 修正旋转中心偏移问题 */
    top: calc(50% - 250px);
    left: calc(50% - 250px);

    z-index: 1;

    /* 重新绘制的完美对称环形渐变，清透不发脏 */
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='500' height='500'%3E%3Cdefs%3E%3Cfilter id='b' x='-50%25' y='-50%25' width='200%25' height='200%25'%3E%3CfeGaussianBlur in='SourceGraphic' stdDeviation='60'/%3E%3C/filter%3E%3C/defs%3E%3Cg filter='url(%23b)'%3E%3Ccircle cx='250' cy='90' r='150' fill='%23ff3b30'/%3E%3Ccircle cx='390' cy='170' r='150' fill='%23ff9500'/%3E%3Ccircle cx='390' cy='330' r='150' fill='%234cd964'/%3E%3Ccircle cx='250' cy='410' r='150' fill='%23007aff'/%3E%3Ccircle cx='110' cy='330' r='150' fill='%235856d6'/%3E%3Ccircle cx='110' cy='170' r='150' fill='%23ff2d55'/%3E%3C/g%3E%3C/svg%3E");
    background-size: cover;

    /* 10秒一圈刚刚好，柔和且不怎么吃 GPU */
    animation: rainbow-rotate 10s linear infinite;
    will-change: transform;
}

/* 核心遮罩内容块：挡在旋转渐变层的上方 */
.island-core-content {
    position: relative;
    z-index: 2;
    width: 100%;
    height: 100%;
    border-radius: 98px;
    transform: translateZ(0);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    overflow: hidden;
}

/* 顺时针匀速旋转 */
@keyframes rainbow-rotate {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}

/* 灵动岛沉浸模式专属样式 */
.coverglass-bg-container {
    position: absolute;
    z-index: 1;
    /* 压在 0层 流光之上，但在 2层 核心内容之下 */
    pointer-events: none;
    overflow: hidden;
}

.coverglass-bg-image {
    position: absolute;
    top: -10%;
    left: -10%;
    width: 120%;
    height: 120%;
    background-size: cover;
    background-position: center;
    opacity: 0.9;
    transition: background-image 0.8s ease;
    transform: translateZ(0);
    /* 开启硬件加速 */
}

.coverglass-noise-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    opacity: 0.15;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='256' height='256'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='2.5' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
}

.coverglass-mask-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    /* 铺一层浅黑色遮罩，确保白色的文字和图标绝对清晰可读 */
    background: rgba(0, 0, 0, 0.45);
}

/* 确保岛内的核心内容层压在背景图上方（audio-spectrum / status-dot 已随子组件迁出） */
.inner-wrapper {
    position: relative;
    z-index: 2;
}

[data-tauri-drag-region] {
    -webkit-app-region: drag;
    cursor: grab;
}

[data-tauri-drag-region]:active {
    cursor: grabbing;
}

/* 状态点样式已随 IslandStatusDot.vue 迁出 */

/* 音乐岛样式已随 IslandMusic.vue 迁出（滚动测量逻辑同时内聚至该组件） */

/* 宽度调整手柄样式 */
.resize-handle {
    position: absolute;
    top: 0;
    width: 6px;
    height: 100%;
    z-index: 100;
    cursor: ew-resize;
    transition: opacity 0.2s ease, background-color 0.2s ease;
    opacity: 0;
}

.resize-handle:hover {
    opacity: 1;
    background-color: rgba(255, 255, 255, 0.3);
}

:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .resize-handle:hover {
    background-color: rgba(0, 0, 0, 0.2);
}

.resize-handle.left {
    left: 0;
    border-radius: 100px 0 0 100px;
}

.resize-handle.right {
    right: 0;
    border-radius: 0 100px 100px 0;
}

/* 展开状态下调整手柄的圆角 */
.island-container:has(.island-core-content[style*="border-radius: 22px"]) .resize-handle {
    border-radius: 0;
}

.island-container:has(.island-core-content[style*="border-radius: 22px"]) .resize-handle.left {
    border-radius: 24px 0 0 24px;
}

.island-container:has(.island-core-content[style*="border-radius: 22px"]) .resize-handle.right {
    border-radius: 0 24px 24px 0;
}

/* 正在调整时的样式 */
.resize-handle:active {
    opacity: 1;
    background-color: rgba(255, 255, 255, 0.4);
}

:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .resize-handle:active {
    background-color: rgba(0, 0, 0, 0.3);
}

/* 光标样式 */
.island-core-content.resize-cursor-left {
    cursor: w-resize;
}

.island-core-content.resize-cursor-right {
    cursor: e-resize;
}

/* 实时活动：分离式灵动岛布局 */
.island-core-content.is-split-layout {
    padding: 0 !important;
    background: transparent !important;
}

/* 左侧主胶囊 */
.left-capsule {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 100%;
    padding: 0 14px;
    position: relative;
    overflow: hidden;
    transition: width 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.left-capsule.is-split {
    width: calc(100% - 44px);
}

/* 多实时活动并行：单一常驻小图标（已拆分至 IslandRtChip.vue，样式随迁） */

.pop-enter-active,
.pop-leave-active {
    transition: all 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.pop-enter-from,
.pop-leave-to {
    opacity: 0;
    transform: scale(0.4) translateX(-30px);
}
</style>
