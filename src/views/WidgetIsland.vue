<template>
    <transition @enter="onEnter" @leave="onLeave" :css="false">
        <div v-show="isIslandVisible" :class="['island-container', { 'has-music-border': isGlowBorderEnabled }]"
            @mousedown="handleMouseDown" @mousemove="handleMouseMove" @mouseup="handleMouseUp"
            @mouseleave="handleMouseLeave" @mouseenter="handleMouseEnter" :style="islandStyle"
            @contextmenu="handleRightClick">

            <div class="rainbow-border-glow" v-if="isGlowBorderEnabled" :style="{ opacity: glowOpacity }"></div>

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
                        <div v-if="isMsgActive" class="msg-box" key="msg" @click="handleNotificationClick"
                            style="cursor: pointer;">
                            <div class="msg-avatar">
                                <img :src="currentMsgIcon" alt="消息图标" class="msg-avatar-img">
                            </div>
                            <div class="msg-text-wrapper">
                                <div class="msg-title">
                                    <span class="sender-name">{{ msgTitle }}</span>
                                    <span class="app-name">{{ msgAppName }}</span>
                                </div>
                                <div class="msg-body">{{ msgBody }}</div>
                            </div>
                        </div>

                        <div v-else-if="displaySysToast" class="system-toast-box" key="systoast">

                            <div v-if="sysToastType === 'app'" class="toast-icon app-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" opacity="0.3" />
                                    <path d="M8 12.5l3 3 5-6" stroke-width="2.5" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                </svg>
                            </div>

                            <div v-else-if="sysToastType === 'lock'" class="toast-icon sys-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                    <path d="M8 12V9a4 4 0 0 1 8 0v3" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                </svg>
                            </div>

                            <div v-else-if="sysToastType === 'unlock'" class="toast-icon sys-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <rect x="4" y="12" width="16" height="8" rx="2" ry="2" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                    <path d="M8 12V9a4 4 0 0 1 8 0" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                </svg>
                            </div>

                            <div v-else-if="sysToastType === 'battery-charge'" class="toast-icon battery-charge-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                    <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                    <polygon points="11 7 8 12 12 12 11 17 14 12 10 12 11 7" stroke-width="1.5"
                                        stroke-linejoin="round" />
                                </svg>
                            </div>

                            <div v-else-if="sysToastType === 'battery-low'" class="toast-icon battery-low-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <rect x="2" y="7" width="16" height="10" rx="2" ry="2" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                    <line x1="22" y1="11" x2="22" y2="13" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                    <line x1="6" y1="12" x2="9" y2="12" stroke-width="4" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                </svg>
                            </div>

                            <div v-else class="toast-icon sys-icon">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <circle cx="12" cy="12" r="10" stroke-width="2" stroke-linecap="round"
                                        stroke-linejoin="round" opacity="0.3" />
                                    <g transform="translate(6, 5.5) scale(0.5)">
                                        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" stroke-width="4"
                                            stroke-linecap="round" stroke-linejoin="round" />
                                        <path d="M13.73 21a2 2 0 0 1-3.46 0" stroke-width="4" stroke-linecap="round"
                                            stroke-linejoin="round" />
                                    </g>
                                </svg>
                            </div>
                            <div class="toast-text">{{ sysToastText }}</div>
                        </div>

                        <div v-else-if="showCountdownText" class="countdown-text-box" key="countdown">
                            <svg viewBox="0 0 24 24" class="countdown-svg"
                                fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <circle cx="12" cy="12" r="10"></circle>
                                <polyline points="12 6 12 12 16 14"></polyline>
                            </svg>
                            <div class="countdown-info">
                                <span v-if="isCountdownFinished" class="countdown-finished-text">倒计时结束</span>
                                <span v-else class="countdown-time">{{ formattedIslandCdTime }}</span>
                            </div>
                        </div>

                        <div v-else-if="showPomodoroText" class="pomodoro-text-box" key="pomodoro">
                            <svg viewBox="0 0 24 24" class="pomodoro-svg"
                                :class="pomodoroPhaseClass" fill="none" stroke="currentColor"
                                stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <circle cx="12" cy="12" r="10"></circle>
                                <polyline points="12 6 12 12 16 14"></polyline>
                            </svg>
                            <div class="pomodoro-info">
                                <span class="pomodoro-time" :class="pomodoroPhaseClass">{{ formattedIslandPomoTime }}</span>
                                <span class="pomodoro-cycle-badge" v-if="pomodoroRemainingCycles > 0">{{ pomodoroRemainingCycles }}</span>
                            </div>
                        </div>

                        <div v-else-if="displayMusic" class="music-ctl-box" :class="{ 'expanded': isMusicExpanded }"
                            :key="'music_' + musicBoxKey" @click="expandMusic" style="cursor: pointer;">
                            <div class="music-top-row">
                                <div class="album-cover" :class="{ 'is-playing': isPlaying }">
                                    <div class="cover-inner"
                                        :style="coverUrl ? { backgroundImage: `url(${coverUrl})`, backgroundSize: 'cover' } : {}">
                                    </div>
                                </div>
                                <div class="music-info-mask-box" ref="maskBoxRef">
                                    <div class="music-info-text single-line" :class="{ 'fade-out': isMusicExpanded }">
                                        <span class="scroll-inner" ref="textInnerRef"
                                            :class="{ 'is-scrolling': scrollDist > 0 }"
                                            :style="scrollDist > 0 ? { '--scroll-dist': scrollDist + 'px', '--scroll-duration': scrollDuration } : {}">
                                            {{ currentTrackInfo }}
                                        </span>
                                    </div>
                                    <div class="music-info-text double-line" :class="{ 'fade-in': isMusicExpanded }">
                                        <div class="song-title">{{ currentSongName }}</div>
                                        <div class="song-artist">{{ currentArtistName }}</div>
                                    </div>
                                </div>
                            </div>
                            <transition name="fade">
                                <div class="music-controls" v-show="isMusicExpanded">
                                    <button class="ctl-btn" @click.stop="prevTrack">
                                        <svg viewBox="0 0 24 24" fill="currentColor">
                                            <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
                                        </svg>
                                    </button>
                                    <button class="ctl-btn play-btn" @click.stop="togglePlay">
                                        <svg v-if="isPlaying" viewBox="0 0 24 24" fill="currentColor">
                                            <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                                        </svg>
                                        <svg v-else viewBox="0 0 24 24" fill="currentColor"
                                            style="transform: translateX(1px);">
                                            <path d="M8 5v14l11-7z" />
                                        </svg>
                                    </button>
                                    <button class="ctl-btn" @click.stop="nextTrack">
                                        <svg viewBox="0 0 24 24" fill="currentColor">
                                            <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
                                        </svg>
                                    </button>
                                </div>
                            </transition>
                            <transition name="fade">
                                <div class="music-progress" v-show="isMusicExpanded" @click.stop>
                                    <template v-if="timelineStatus === 'available'">
                                        <div class="progress-time-row">
                                            <span class="progress-time">{{ formatTime(progressPosition) }}</span>
                                            <span class="progress-time">{{ formatTime(progressEnd) }}</span>
                                        </div>
                                        <div class="progress-bar" ref="progressBarRef"
                                            :class="{ disabled: !musicTimeline.canSeek }"
                                            :aria-disabled="!musicTimeline.canSeek"
                                            @pointerdown.stop="onProgressPointerDown"
                                            @pointermove="onProgressPointerMove"
                                            @pointerup="onProgressPointerUp"
                                            @pointercancel="onProgressPointerCancel"
                                            @lostpointercapture="onProgressPointerCancel">
                                            <div class="progress-filled" :class="{ dragging: isDraggingProgress }"
                                                :style="{ width: progressPercent + '%' }"></div>
                                            <div class="progress-thumb" :style="{ left: progressPercent + '%' }"></div>
                                        </div>
                                        <div class="progress-remaining">-{{ formatTime(progressEnd - progressPosition) }}</div>
                                    </template>
                                    <div v-else class="progress-placeholder">
                                        {{ timelineStatus === 'loading' ? '正在读取播放进度…' : '当前播放器未提供播放进度' }}
                                    </div>
                                </div>
                            </transition>
                        </div>

                        <div v-else-if="displaySpeed" class="speed-box" key="speed">
                            <transition name="speed-fade" mode="out-in">
                                <div v-if="isShowingUpload" class="speed-item" key="upload">
                                    <span :class="['label', { 'high-traffic': isHighUpload }]">⬆</span>
                                    <span class="value">{{ uploadSpeed }}</span>
                                </div>
                                <div v-else class="speed-item" key="download">
                                    <span :class="['label', { 'high-traffic': isHighDownload }]">⬇</span>
                                    <span class="value">{{ downloadSpeed }}</span>
                                </div>
                            </transition>
                        </div>
                    </transition>
                </div>

                <transition mode="out-in" @enter="onInnerEnter" @leave="onInnerLeave" :css="false">
                    <!-- 倒计时展开态：暂停/开始 + 关闭 -->
                    <div v-if="isCountdownExpanded" class="cd-expanded-controls" key="cd-controls">
                        <button class="cd-pause-btn" @click.stop="handleCdTogglePauseResume" :title="cdPaused ? '继续' : '暂停'">
                            <svg v-if="cdPaused" viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                                <path d="M8 5v14l11-7z" />
                            </svg>
                            <svg v-else viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                                <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                            </svg>
                        </button>
                        <div class="island-close-btn cd-close-btn" @click.stop="handleCdClose">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </div>
                    </div>

                    <!-- 番茄钟展开态：关闭 -->
                    <div v-else-if="isPomodoroExpanded" class="island-close-btn" @click.stop="isPomodoroExpanded = false; handlePomoClose()" key="close-btn">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </div>

                    <!-- 硬件监控展开态：CPU / 内存 详情 + 关闭 -->
                    <div v-else-if="isHardwareExpanded && hwEnabled" class="hw-expanded-detail" key="hw-detail">
                        <div class="hw-detail-row">
                            <span class="hw-detail-label">CPU</span>
                            <span class="hw-detail-val" :class="{ 'high': hwCpuPct >= 80 }">{{ Math.round(hwCpuPct) }}%</span>
                        </div>
                        <div class="hw-detail-row">
                            <span class="hw-detail-label">RAM</span>
                            <span class="hw-detail-val" :class="{ 'high': hwMemPct >= 80 }">{{ Math.round(hwMemPct) }}%</span>
                        </div>
                        <div class="hw-close-btn-x" @click.stop="collapseHardware">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </div>
                    </div>

                    <div v-else-if="showSpectrumIndicator" class="audio-spectrum"
                        :class="{ 'is-playing': isPlaying, 'expanded': isMusicExpanded }" key="spectrum">
                        <span class="bar" v-for="(val, index) in spectrumData" :key="index"
                            :style="{ transform: `scaleY(${val})` }"></span>
                    </div>

                    <div v-else :class="['status-dot', networkStatus]" key="dot"></div>
                </transition>
                </div>

                <transition name="pop">
                    <div class="right-circle" :style="coreContentStyle" v-if="isSplitMode"
                        @click.stop="isPomodoroExpanded = true" style="cursor: pointer;">
                        <svg viewBox="0 0 24 24" class="pomodoro-svg" fill="none" stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round" style="position: relative; z-index: 2;">
                            <circle cx="12" cy="12" r="10"></circle>
                            <polyline points="12 6 12 12 16 14"></polyline>
                        </svg>
                    </div>

                    <!-- 倒计时右侧圆钮（灵动岛拆分模式 + 倒计时可见 + 音乐控制器开启 + 倒计时未展开） -->
                    <div class="right-circle cd-right-circle" :style="coreContentStyle"
                        v-if="isCountdownVisible && isMusicCtlEnabled && !isCountdownExpanded && !isMsgActive && !displaySysToast && !isMusicExpanded && !isMusicExpanding"
                        @click.stop="isCountdownExpanded = true" style="cursor: pointer;">
                        <svg viewBox="0 0 24 24" class="countdown-svg" fill="none" stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round" style="position: relative; z-index: 2;">
                            <circle cx="12" cy="12" r="10"></circle>
                            <polyline points="12 6 12 12 16 14"></polyline>
                        </svg>
                    </div>

                    <!-- 硬件监控右侧圆钮（灵动岛拆分模式 + 硬件监控开启 + 未展开） -->
                    <div class="right-circle hw-right-circle" :style="coreContentStyle"
                        v-if="isHwAccessoryVisible"
                        @click.stop="expandHardware" style="cursor: pointer;">
                        <svg viewBox="0 0 24 24" class="hw-chip-svg" :class="{ 'high': hwMaxPct >= 80 }"
                            fill="none" stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round" style="position: relative; z-index: 2;">
                            <rect x="4" y="4" width="16" height="16" rx="2" />
                            <rect x="9" y="9" width="6" height="6" />
                            <line x1="9" y1="1" x2="9" y2="4" />
                            <line x1="15" y1="1" x2="15" y2="4" />
                            <line x1="9" y1="20" x2="9" y2="23" />
                            <line x1="15" y1="20" x2="15" y2="23" />
                            <line x1="20" y1="9" x2="23" y2="9" />
                            <line x1="20" y1="14" x2="23" y2="14" />
                            <line x1="1" y1="9" x2="4" y2="9" />
                            <line x1="1" y1="14" x2="4" y2="14" />
                        </svg>
                    </div>
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
import { ref, onMounted, onUnmounted, computed, watch, nextTick, type CSSProperties } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, currentMonitor, PhysicalPosition, LogicalPosition, PhysicalSize } from '@tauri-apps/api/window'; import { Menu, MenuItem } from '@tauri-apps/api/menu';
import { listen, emit } from '@tauri-apps/api/event';
import { formatSpeed } from '../utils/format';
import {
    NSD_AUTO_HIDE_DELAY, NSD_AUTO_HIDE_ENABLED,
    NSD_AUTO_COLLAPSE_DELAY, NSD_AUTO_COLLAPSE_ENABLED,
    NSD_ISLAND_OPACITY, NSD_ISLAND_THEME,
    NSD_MUSIC_CTRL, NSD_GLOW_BORDER,
    NSD_PIN_TASKBAR, NSD_POSITION_LOCKED,
    NSD_MSG_MODE, NSD_ROTATION_MODE,
    NSD_ISLAND_WIDTH, NSD_ISLAND_POSITION, NSD_MSG_NOTIFY,
    NSD_TARGET_PLAYER, NSD_AUTO_HIDE_FS,
    NSD_POMODORO_VISIBLE,
    NSD_COUNTDOWN_VISIBLE,
    NSD_HW_ENABLED,
} from '../constants/storageKeys';

const isIslandVisible = ref(false);
const isMenuOpen = ref(false);

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

// 全屏自动隐藏相关
const isAutoHideFullscreen = ref(localStorage.getItem(NSD_AUTO_HIDE_FS) === 'true');
let wasVisibleBeforeFullscreen = false;
let isHidingForFullscreen = false;

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
    return isPomodoroVisible.value && isMusicCtlEnabled.value && !isPomodoroExpanded.value;
});

// 硬件监控附属图标可见性（镜像倒计时：音乐控制器开启 + 硬件监控开启 + 未展开）
const isHwAccessoryVisible = computed(() => {
    if (isMsgActive.value || displaySysToast.value || isMusicExpanded.value || isMusicExpanding.value) return false;
    if (!hwEnabled.value || !isMusicCtlEnabled.value || isHardwareExpanded.value) return false;
    return true;
});

// 硬件负载峰值（用于附属图标高负载配色）
const hwMaxPct = computed(() => Math.max(hwCpuPct.value, hwMemPct.value));

// 关闭番茄钟展开（恢复岛尺寸）
const handlePomoClose = () => {
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
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
        // 倒计时已结束 → 仅让后端停止，由 countdown-tick idle 事件处理UI清理
        // 不直接操作 UI 状态，避免与自动隐藏等功能冲突
        await invoke('stop_countdown');
        isCountdownExpanded.value = false;
        if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
            const { h } = getBaseSize();
            const savedWidth = restoreIslandWidth();
            const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
            animateIslandSize(targetWidth, h);
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
};

// 自动隐藏相关变量
const isMouseOver = ref(false);
let autoHideTimer: number | null = null;
const autoHideDelay = ref(Number(localStorage.getItem(NSD_AUTO_HIDE_DELAY) || '2000')); // 默认2秒
const isAutoHideEnabled = ref(localStorage.getItem(NSD_AUTO_HIDE_ENABLED) === 'true'); // 自动隐藏功能开关

// 自动折叠相关变量（灵动岛展开后，鼠标离开自动折叠回小岛状态）
let autoCollapseTimer: number | null = null;
const autoCollapseDelay = ref(Number(localStorage.getItem(NSD_AUTO_COLLAPSE_DELAY) || '2000')); // 默认2秒
const isAutoCollapseEnabled = ref(localStorage.getItem(NSD_AUTO_COLLAPSE_ENABLED) === 'true'); // 自动折叠功能开关

// 记录当前是否显示上行网速（用于轮换�?
const isShowingUpload = ref(false);
let speedCycleTimer: number | null = null;

// 控制 DOM 真正的高宽变量与消息数据
const currentWidth = ref(150);
const currentHeight = ref(34);
const isMsgActive = ref(false);
const msgTitle = ref('');
const msgAppName = ref('');
const msgBody = ref('');
const msgAumid = ref('');

// 系统操作通知专用变量
const displaySysToast = ref(false);
const sysToastText = ref('');
const sysToastType = ref<'app' | 'sys' | 'battery-charge' | 'battery-low' | 'lock' | 'unlock'>('app');
const toastQueue = ref<{ text: string, type: 'app' | 'sys' | 'battery-charge' | 'battery-low' | 'lock' | 'unlock' }[]>([]);
let isProcessingToast = false;

// 队列处理函数
const processToastQueue = async () => {
    // 如果正在处理，或者队列为空，则直接返�?
    if (isProcessingToast || toastQueue.value.length === 0) return;

    // 优先级判断：如果当前正在显示消息通知(最高优先级)，则挂起等待
    if (isMsgActive.value) return;

    isProcessingToast = true;
    const nextToast = toastQueue.value.shift();

    if (nextToast) {
        sysToastText.value = nextToast.text;
        sysToastType.value = nextToast.type;
        displaySysToast.value = true;
        
        // 自动恢复显示：当有系统通知时，如果灵动岛被隐藏，则自动恢复显示
        if (!isIslandVisible.value) {
            getCurrentWindow().show();
            isIslandVisible.value = true;
        }

        // 停留显示
        await new Promise(resolve => setTimeout(resolve, 2000));

        displaySysToast.value = false;
        // 等待离开动画播完 (约200ms) 再处理下一个
        await new Promise(resolve => setTimeout(resolve, 200));

        // 系统 toast 结束后，重新评估自动隐藏
        // 场景：音乐控制器开启 + 无音乐播放时，toast 弹出强制显示了灵动岛，
        // toast 结束后应恢复自动隐藏
        scheduleAutoHide();
    }

    isProcessingToast = false;
    processToastQueue(); // 递归检查是否还有下一个通知
};

// 监听系统通知显示状态，解决网速模式下尺寸过小导致文字溢出/遮挡指示灯的问题
watch(displaySysToast, (newVal) => {
    if (newVal) {
        // 当有系统操作通知出现时，强制展开到默认标准尺�?
        animateIslandSize(260, 42);
    } else {
        // 通知消失时，恢复到当前状态该有的尺寸
        // （前提是没有被应用消息或音乐面板霸占�?
        if (!isMsgActive.value && !isMusicExpanded.value && !isMusicExpanding.value) {
            const { h } = getBaseSize();
            const savedWidth = restoreIslandWidth();
            const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
            animateIslandSize(targetWidth, h);
        }
    }
});

// 暴露给外部调用的触发函数
const showToast = (text: string, type: 'app' | 'sys' | 'battery-charge' | 'battery-low' | 'lock' | 'unlock' = 'app') => {
    toastQueue.value.push({ text, type });
    processToastQueue();
};

// 监听消息通知状态，一旦消息通知消失，立刻唤醒可能被挂起的操作通知队列
watch(isMsgActive, (newVal) => {
    if (!newVal) {
        processToastQueue();
    }
});

// 记录音乐岛是否处于展开状�?
const isMusicExpanded = ref(false);
const isMusicExpanding = ref(false); // 记录是否正在播放弹性按压展开动画
let musicExpandAnimTimer: number | null = null; // 用于接管展开时的定时器，防止冲突

// 灵动岛自身的透明度变量（默认100�?
const islandOpacity = ref(Number(localStorage.getItem(NSD_ISLAND_OPACITY) || '100'));

// 灵动岛自身主题色
const islandTheme = ref(localStorage.getItem(NSD_ISLAND_THEME) || 'black');

// 1. 瞬间判定当前是否处于大窗口状�?
const isExpandedSize = computed(() => isMusicExpanded.value || isMsgActive.value);

// 2. 外层容器：状态一变，立马切成目标圆角
const islandStyle = computed<CSSProperties>(() => {
    const linear = islandOpacity.value / 100;
    const alpha = Math.pow(linear, 1 / 2.2);
    const baseStyle = islandTheme.value === 'white' ? {
        backgroundColor: `rgba(255, 255, 255, ${alpha})`,
        color: '#000000'
    } : {
        backgroundColor: `rgba(0, 0, 0, ${alpha})`,
        color: '#ffffff'
    };

    return {
        ...baseStyle,
        width: '100vw',
        height: '100vh',
        // 只要展开就是 24px，收起就�?100px
        borderRadius: isExpandedSize.value ? '24px' : '100px',
        position: 'relative',
    };
});

// 3. 内层核心：永远比外层�?2px
const coreContentStyle = computed(() => {
    const linear = islandOpacity.value / 100;
    const alpha = Math.pow(linear, 1 / 2.2);

    // 展开 22px，收�?98px
    const innerRadius = isExpandedSize.value ? '22px' : '98px';

    if (islandTheme.value === 'white') {
        return {
            backgroundColor: `rgba(255, 255, 255, ${alpha})`,
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

// 记录当前是否属于大流量状�?
const isHighDownload = ref(false);
const isHighUpload = ref(false);

// 网络状态指示灯：good(�?, warning(�?, error(�?
const networkStatus = ref<'good' | 'warning' | 'error'>('good');

// 硬件监控（灵动岛附属图标，由 LiveActive 的开关驱动；数据来自后端 monitor-stats 推送）
const hwEnabled = ref(localStorage.getItem(NSD_HW_ENABLED) === 'true');
const hwCpuPct = ref(0);
const hwMemPct = ref(0);
const isHardwareExpanded = ref(false);

const expandHardware = () => {
    if (isHardwareExpanded.value) return;
    isHardwareExpanded.value = true;
    const { h } = getBaseSize();
    animateIslandSize(200, h);
};

const collapseHardware = () => {
    if (!isHardwareExpanded.value) return;
    isHardwareExpanded.value = false;
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
};

// 音乐控制功能开�?
const isMusicCtlEnabled = ref(localStorage.getItem(NSD_MUSIC_CTRL) === 'true');
const isPlaying = ref(false);
// 流光边框默认状态完全镜像音乐控制器（只要音乐控制器开着它就开，关了就一起关�?
const isGlowBorderEnabled = ref(localStorage.getItem(NSD_GLOW_BORDER) === 'true');

// 律动频谱
const spectrumData = ref([0.35, 0.35, 0.35, 0.35, 0.35]);

// 封面url
const coverUrl = ref('');
const coverCache = new Map<string, string>();

// 记录是否开启了置于任务�?
const isPinnedToTaskbar = ref(localStorage.getItem(NSD_PIN_TASKBAR) === 'true');
// 记录是否锁定了位置，并存到本�?
const isPositionLocked = ref(localStorage.getItem(NSD_POSITION_LOCKED) === 'true');

// 宽度调整相关状�?
const isResizing = ref(false);
const resizeSide = ref<'left' | 'right' | null>(null);
let resizeStartX = 0;
let resizeStartWidth = 0;
const MIN_WIDTH = 100; // 最小宽度
const MAX_WIDTH = 500; // 最大宽度

// 鼠标是否在边缘区域（用于光标样式�?
const mouseNearEdge = ref<'left' | 'right' | null>(null);

// 计算是否可以调整宽度
const canResize = computed(() => {
    return !isPositionLocked.value && !isMusicExpanded.value && !isMusicExpanding.value && !isMsgActive.value && !displaySysToast.value;
});
// 记录消息模式开关状�?
const isMsgModeEnabled = ref(localStorage.getItem(NSD_MSG_MODE) === 'true');
// 轮换功能核心逻辑
const isRotationEnabled = ref(localStorage.getItem(NSD_ROTATION_MODE) === 'true');
const currentRotIndex = ref(0); // 0=网速 1=音乐
let rotationTimer: number | null = null;

// 使用计算属性智能判断当前该显示�?
const displaySpeed = computed(() => !isMsgActive.value && !displaySysToast.value && !showPomodoroText.value && !showCountdownText.value && (isRotationEnabled.value ? currentRotIndex.value === 0 : !isMusicCtlEnabled.value));
const displayMusic = computed(() => !isMsgActive.value && !displaySysToast.value && !showPomodoroText.value && !showCountdownText.value && (isRotationEnabled.value ? currentRotIndex.value === 1 : isMusicCtlEnabled.value));

// 辅助函数：获取当前状态应该拥有的默认大小
const getBaseSize = () => {
    if (isPomodoroVisible.value || isCountdownVisible.value) return { w: 250, h: 38 };
    if (displaySpeed.value) return { w: 150, h: 34 };
    return { w: 260, h: 42 };
};

// 监听内容切换，触发丝滑动画�?
watch([displaySpeed, displayMusic, showPomodoroText, showCountdownText], () => {
    // 只有在没有被临时弹窗（消息、音乐展开）霸占时，才执行基础大小切换
    if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
    }
});

// 专门用于控制右侧常驻指示灯的独立计算属性（完全不受消息通知打断�?
const showSpectrumIndicator = computed(() => {
    return isRotationEnabled.value ? currentRotIndex.value === 1 : isMusicCtlEnabled.value;
});

// 频谱开关状态追踪，避免重复调用后端
let isSpectrumActive = false;

// 按需启停音频频谱捕获：仅在前端需要显示频谱时才激活后�?FFT 运算，空闲时�?CPU 零分�?
watch([isPlaying, showSpectrumIndicator], () => {
    const shouldActivate = isPlaying.value && showSpectrumIndicator.value;
    if (shouldActivate && !isSpectrumActive) {
        isSpectrumActive = true;
        invoke('set_spectrum_active', { active: true }).catch(() => {});
    } else if (!shouldActivate && isSpectrumActive) {
        isSpectrumActive = false;
        invoke('set_spectrum_active', { active: false }).catch(() => {});
    }
});

const startRotation = () => {
    if (rotationTimer) clearInterval(rotationTimer);
    rotationTimer = window.setInterval(() => {
        currentRotIndex.value = (currentRotIndex.value + 1) % 2;
    }, 5000); // 5秒轮换一次
};

// 统一的自动隐藏定时器管理函数
// 自动隐藏仅在以下条件全部满足时触发：
//   1. 自动隐藏开关已开启 (isAutoHideEnabled)
//   2. 音乐控制器模式已打开 (isMusicCtlEnabled)
//   3. 没有音乐在播放 (!isPlaying)
// 其余任何情况（如临时 toast、通知弹出、鼠标离开但非音乐模式等）均不隐藏
const scheduleAutoHide = (delay?: number) => {
    // 前置守卫：不满足条件时直接返回，不设定定时器
    if (!isAutoHideEnabled.value || !isMusicCtlEnabled.value || isPlaying.value) {
        return;
    }
    if (autoHideTimer) {
        clearTimeout(autoHideTimer);
        autoHideTimer = null;
    }
    autoHideTimer = window.setTimeout(() => {
        // 定时器到期时再次全量检查条件（防止定时期间状态变化导致误隐藏）
        if (!isMouseOver.value
            && isIslandVisible.value
            && isAutoHideEnabled.value
            && isMusicCtlEnabled.value
            && !isPlaying.value) {
            isAutoHiding = true;
            isIslandVisible.value = false;
        }
    }, delay ?? autoHideDelay.value);
};

const stopRotation = () => {
    if (rotationTimer) {
        clearInterval(rotationTimer);
        rotationTimer = null;
    }
};

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

            // X坐标: 屏幕最左侧 + 10px的边�?
            const x = monitorLeftPhysical + (10 * scaleFactor);
            // Y坐标: 物理最底部 - 窗口高度 - 3px微调
            const y = monitorTopPhysical + monitorHeightPhysical - ((WINDOW_INIT_HEIGHT + 3) * scaleFactor);

            // 【终极绝杀核心】：绕过 Windows 系统的任务栏防遮挡机�?
            // 在强制覆盖任务栏坐标之前，先隐身�?
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

    // 2. 发送指令给 Rust �?SMTC
    try {
        await invoke('control_system_media', { action: 'play_pause' });
    } catch (err) {
        console.error('播放控制失败:', err);
        // 如果底层控制失败了，再把图标状态回滚回�?
        isPlaying.value = !isPlaying.value;
    }
};

const prevTrack = async () => {
    await invoke('control_system_media', { action: 'prev' });
};

const nextTrack = async () => {
    await invoke('control_system_media', { action: 'next' });
};

// 核心同步函数：塞入到你的 fetchSpeedStats 同一频次的定时器�?
const syncMusicStatus = async () => {
    try {
        // 1. 调用 Rust 提取网易云标�?[歌名, 歌手, 是否在播放]
        const res = await invoke<[string, string, boolean] | null>('fetch_netease_music_info');

        if (res) {
            const [song, artist, playing] = res;

            // 新增这两行为了展开后的双行显示分别赋�?
            currentSongName.value = song;
            currentArtistName.value = artist || '未知歌手';

            // 拼接新的歌曲信息
            const newTrackInfo = artist ? `${song} - ${artist}` : song;

            if (currentTrackInfo.value !== newTrackInfo) {
                currentTrackInfo.value = newTrackInfo;

                // 优先读取缓存（LRU：命中时刷新插入顺序，超限时淘汰最旧条目）
                if (coverCache.has(newTrackInfo)) {
                    // 命中：先删再设，将该条目移到 Map 末尾（最新）
                    const cached = coverCache.get(newTrackInfo)!;
                    coverCache.delete(newTrackInfo);
                    coverCache.set(newTrackInfo, cached);
                    coverUrl.value = cached;
                } else {
                    try {
                        const realCoverUrl = await invoke<string>('get_random_cover_url', {
                            songName: song,
                            artistName: artist
                        });
                        coverUrl.value = realCoverUrl;
                        // 写入缓存，超限逐条淘汰最旧条目（LRU�?
                        while (coverCache.size >= 50) {
                            const oldest = coverCache.keys().next().value;
                            if (oldest !== undefined) coverCache.delete(oldest);
                        }
                        coverCache.set(newTrackInfo, realCoverUrl);
                    } catch (coverErr) {
                        console.error('所有封面源均获取失败', coverErr);
                        // 使用本地图标或纯色背景，不要再用外部 URL 作为错误兜底
                        coverUrl.value = '';
                    }
                }
            }

            isPlaying.value = playing;

            // 音乐播放器模式：有音乐就显示，没音乐就隐藏
            if (displayMusic.value) {
                if (playing && !isIslandVisible.value) {
                    // 有音乐播放且灵动岛被隐藏，自动恢复显示
                    getCurrentWindow().show();
                    isIslandVisible.value = true;
                } else if (!playing && isIslandVisible.value && !isMouseOver.value) {
                    // 音乐停止播放且鼠标不在灵动岛上，延迟隐藏
                    // scheduleAutoHide 内部会校验音乐控制器模式 + 自动隐藏开关
                    scheduleAutoHide();
                }
            }
        } else {
            // 没检测到播放时，清空状态
            currentTrackInfo.value = `未在播放歌曲 - ${getPlayerName()}`;
            isPlaying.value = false;
            coverUrl.value = ''; // 没歌时清空，显示默认的优美渐变色

            // 音乐播放器模式：音乐停止时隐藏灵动岛
            if (isIslandVisible.value && !isMouseOver.value) {
                scheduleAutoHide();
            }
        }
    } catch (err) {
        console.error('音乐信息获取失败:', err);
    }
};

const showInfo = ref(false);
// 默认显示内容动态从本地缓存读取
const getPlayerName = () => {
    const key = localStorage.getItem(NSD_TARGET_PLAYER) || 'netease';
    const map: Record<string, string> = { 
        'netease': '网易云音乐', 
        'spotify': 'Spotify', 
        'apple': 'Apple Music', 
        'qqmusic': 'QQ音乐', 
        'kugou': '酷狗音乐', 
        'echo': 'Echo Music',
        'lx-music': '洛雪音乐',
        'smtc': 'SMTC',
    };
    return map[key] || '未知平台';
};

// 定义一个用于强制刷新的 key
const musicBoxKey = ref(0);

// ===== F6 音乐进度条：展开态拉�?SMTC Timeline 并支持拖动定�?=====
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
const progressBarRef = ref<HTMLElement | null>(null);

const progressEnd = computed(() => musicTimeline.value.end);
// 拖动时显示临时位置；播放时在两次 SMTC 同步之间按本地时钟平滑推进�?
const progressPosition = computed(() => {
    if (isDraggingProgress.value) return dragPosition.value;
    const elapsed = isPlaying.value ? Math.max(0, timelineClock.value - timelineSyncedAt.value) : 0;
    return Math.min(progressEnd.value, musicTimeline.value.position + elapsed);
});
const progressPercent = computed(() => progressEnd.value > 0
    ? Math.min(100, Math.max(0, (progressPosition.value / progressEnd.value) * 100))
    : 0);

// 毫秒�?m:ss
const formatTime = (ms: number) => {
    if (ms < 0 || isNaN(ms)) ms = 0;
    const totalSec = Math.floor(ms / 1000);
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
};

const resetTimeline = (status: TimelineStatus = 'idle') => {
    isDraggingProgress.value = false;
    dragPosition.value = 0;
    musicTimeline.value = { position: 0, end: 0, canSeek: false };
    timelineStatus.value = status;
    timelineMissCount = 0;
    timelineSyncedAt.value = Date.now();
    timelineClock.value = timelineSyncedAt.value;
};

// 拉取播放进度（仅展开态且灵动岛可见时执行，折�?隐藏时暂停）�?
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
        } else {
            // SMTC 偶尔会在切歌时短暂返回空 Timeline，连续失败后才判定不可用�?
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
    }, 100) as unknown as number;
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

// 拖动定位：将指针横坐标换算为播放位置
const updateDragPosition = (e: PointerEvent) => {
    if (!progressBarRef.value || progressEnd.value === 0) return;
    const rect = progressBarRef.value.getBoundingClientRect();
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

// 展开�?+ 灵动岛可�?时启动进度条轮询；折�?隐藏时暂停并清空数据
watch([isMusicExpanded, isIslandVisible], () => {
    if (isMusicExpanded.value && isIslandVisible.value) {
        startProgressTimer();
    } else {
        stopProgressTimer();
        if (!isMusicExpanded.value) {
            // 折叠时清空，避免下次展开瞬间残留旧歌曲进度�?
            resetTimeline();
        }
    }
});

// 定义双行文本所需的单独变�?
const currentSongName = ref('未在播放歌曲');
const currentArtistName = ref(getPlayerName());
const currentTrackInfo = ref(`未在播放歌曲 - ${getPlayerName()}`);

watch(currentTrackInfo, () => {
    if (!isMusicExpanded.value) return;
    resetTimeline('loading');
    fetchTimeline();
});

// 音乐滚动相关变量
const maskBoxRef = ref<HTMLElement | null>(null);
const textInnerRef = ref<HTMLElement | null>(null);
const scrollDist = ref(0);
const scrollDuration = ref('0s');

// 核心计算函数：判断文本是否超出容器，并动态调整滚动速度和时�?
const calculateScroll = () => {
    if (!textInnerRef.value || !maskBoxRef.value) return;

    // 展开状态下不执行滚�?
    if (isMusicExpanded.value) {
        scrollDist.value = 0;
        return;
    }

    // 核心修复 1：使�?getBoundingClientRect() 获取无视父级限制的真实渲染宽�?
    const textWidth = textInnerRef.value.getBoundingClientRect().width;
    const containerWidth = maskBoxRef.value.clientWidth;

    if (textWidth > containerWidth) {
        // 核心修复 1：使�?Math.ceil() 强制取整，绝对不允许出现小数像素�?
        scrollDist.value = Math.ceil(textWidth - containerWidth + 20);

        // 按照 30px/s 的速度阅读，计算纯移动时间
        const timeToMove = scrollDist.value / 30;

        // 将首尾各停留�?1s 左右（基�?0%占比计算）融入总时长中，确保匀�?
        const totalDuration = timeToMove / 0.6;

        scrollDuration.value = `${Math.max(totalDuration, 4.5)}s`;
    } else {
        scrollDist.value = 0;
    }
};

// 核心修复 2：监听数组必须带�?displayMusic，并�?nextTick 后加上微小延迟，防止 v-else-if 导致宽度拿到 0
watch([currentTrackInfo, displayMusic, isMusicExpanded], async () => {
    await nextTick();
    setTimeout(() => {
        if (displayMusic.value) {
            calculateScroll();
        } else {
            // 切到其他界面（比如网速）时，归零重置
            scrollDist.value = 0;
        }
    }, 100);
});

let lastRx = 0;
let lastTx = 0;
let speedTimer: number;
let pingTimer: number;
let musicTimer: number;
let msgTimer: number | null = null;
let notifyTimer: number;

// 防抖控制变量
let lowTrafficStartTime = Date.now();
const RED_DELAY_MS = 5000;

// 计算流量数字，并实时更新大流量状�?
const fetchSpeedStats = async () => {
    try {
        const [currentRx, currentTx] = await invoke<[number, number]>('get_network_stats');
        if (lastRx !== 0) {
            const rxDiff = currentRx - lastRx;
            const txDiff = currentTx - lastTx;

            downloadSpeed.value = formatSpeed(rxDiff);
            uploadSpeed.value = formatSpeed(txDiff);

            // 1MB = 1048576 字节
            const limit = 1024 * 1024;
            const currentDownloadHigh = rxDiff >= limit;
            const currentUploadHigh = txDiff >= limit;

            isHighDownload.value = currentDownloadHigh;
            isHighUpload.value = currentUploadHigh;

            // 维护低流量持续时�?
            if (currentDownloadHigh || currentUploadHigh) {
                // 如果目前依然是大流量，重置计时器
                lowTrafficStartTime = Date.now();
            }
        }
        lastRx = currentRx;
        lastTx = currentTx;
    } catch (error) {
        console.error('流量获取失败:', error);
    }
};

// 通过真实延迟控制状态灯（加入大流量避让判断�?
const checkNetworkLatency = async () => {
    try {
        const latency = await invoke<number>('get_network_latency');

        // 只要能拿到延迟数字，说明网络肯定是通的
        if (latency < 150) {
            networkStatus.value = 'good';      // 延迟优秀，绿灯
        } else {
            networkStatus.value = 'warning';   // 延迟较高不稳定，黄色
        }
    } catch (error) {
        // 当Rust抛出超时异常时，说明网络可能断开连接

        // 1. 如果当前正处于大流量状态，绝不变红，降级显示为黄灯
        if (isHighDownload.value || isHighUpload.value) {
            networkStatus.value = 'warning';
            return;
        }

        // 2. 如果流量刚刚消失，判断距离大流量结束是否超过了设定的缓冲时间
        const timeSinceLowTraffic = Date.now() - lowTrafficStartTime;
        if (timeSinceLowTraffic < RED_DELAY_MS) {
            // 还在缓冲期内，判定为大流量带来的余波卡顿，依然保持黄�?
            networkStatus.value = 'warning';
        } else {
            // 已经下了好几秒都没流量了，结果还连不上，说明是真的断网了，变红！
            networkStatus.value = 'error';
        }
    }
};

// 监听网络状态变化，触发系统通知
watch(networkStatus, (newStatus, oldStatus) => {
    // 忽略初始化时的变化，确保是真的状态翻�?
    if (oldStatus && oldStatus !== newStatus) {
        if (newStatus === 'error') {
            showToast('网络连接已断开', 'sys');
        } else if (newStatus === 'good' && oldStatus === 'error') {
            showToast('网络已恢复连接', 'sys');
        }
    }
});

// 调整窗口位置到正确位�?
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

            // 2. 重新获取设定后的真实物理尺寸，用于精准居�?
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

// 核心动画实现：基于你�?AE 公式转化
const onEnter = (el: Element, done: () => void) => {
    const HTMLElement = el as HTMLElement;
    HTMLElement.style.transformOrigin = 'center top'; // 类似苹果灵动岛从顶部展开
    let start = performance.now();

    const freq = 2.0;
    const decay = 10.5; // 适度拉高阻力
    const duration = 600;

    const animate = (time: number) => {
        let t = (time - start) / 1000;
        let progress = (time - start) / duration;

        // 数学方程�? - cos(2πft) * e^(-dt)
        let scale = 1 - Math.cos(freq * t * 2 * Math.PI) * Math.exp(-decay * t);
        let opacity = Math.min(1, progress * 4); // 快速淡入

        HTMLElement.style.transform = `scale(${scale})`;
        HTMLElement.style.opacity = opacity.toString();

        if (progress < 1) {
            requestAnimationFrame(animate);
        } else {
            // 重置为最终干净的状�?
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

        // 离开动画：快速平滑回�?
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
            if (!isAutoHiding && !isHidingForFullscreen) {
                emit('island-status-sync', { visible: false });
            }
            isAutoHiding = false;
            isHidingForFullscreen = false;
        }
    };
    requestAnimationFrame(animate);
};

let mouseDownX = 0;
let mouseDownY = 0;
let isMouseDown = false;
let isAutoHiding = false; // 标记当前隐藏是否由自动隐藏触发（区别于用户主动关闭）

// 自定义横向拖拽相关状态（任务栏模式下仅允许横向移动）
let isCustomDragging = false;
let customDragStartScreenX = 0;
let customDragStartWindowX = 0;
let customDragStartWindowY = 0;
let customDragMonitor: { position: { x: number; y: number }; size: { width: number; height: number } } | null = null;
let customDragWindowWidth = 0;

const handleMouseDown = (event: MouseEvent) => {
    if ((event.target as HTMLElement).closest('.ctl-btn')) return;
    if ((event.target as HTMLElement).closest('.resize-handle')) return;

    // 检测是否在边缘区域，如果是则开始宽度调�?
    if (!isPositionLocked.value && !isMusicExpanded.value && !isMusicExpanding.value && !isMsgActive.value && !displaySysToast.value) {
        if (isNearEdge(event, 'left')) {
            handleResizeStart(event, 'left');
            return;
        }
        if (isNearEdge(event, 'right')) {
            handleResizeStart(event, 'right');
            return;
        }
    }

    // 无论有没有锁定，都必须老老实实记录坐标，给后面的"点击展开"提供判断依据�?
    mouseDownX = event.clientX;
    mouseDownY = event.clientY;
    isMouseDown = true;
};

// ===== 宽度调整相关函数 =====
const handleResizeStart = (event: MouseEvent, side: 'left' | 'right') => {
    // 位置锁定时禁止调�?
    if (isPositionLocked.value) return;

    // 音乐展开、消息通知等状态下禁止调整
    if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) return;

    event.preventDefault();
    event.stopPropagation();

    isResizing.value = true;
    resizeSide.value = side;
    resizeStartX = event.screenX;
    resizeStartWidth = currentWidth.value;

    document.addEventListener('mousemove', handleResizeMove);
    document.addEventListener('mouseup', handleResizeEnd);
};

const handleResizeMove = async (event: MouseEvent) => {
    if (!isResizing.value || !resizeSide.value) return;

    const scaleFactor = window.devicePixelRatio;
    const deltaXLogical = event.screenX - resizeStartX;

    let newWidth: number;
    if (resizeSide.value === 'right') {
        newWidth = resizeStartWidth + deltaXLogical;
    } else {
        newWidth = resizeStartWidth - deltaXLogical;
    }

    // 边界约束
    newWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth));

    // 更新灵动岛宽�?
    try {
        const appWindow = getCurrentWindow();
        await appWindow.setSize(new PhysicalSize(Math.ceil(newWidth * scaleFactor), Math.ceil(currentHeight.value * scaleFactor)));

        // 如果是左侧调整，需要同时移动窗口位置以保持右侧固定
        if (resizeSide.value === 'left') {
            const pos = await appWindow.outerPosition();
            const widthDelta = (newWidth - currentWidth.value) * scaleFactor;
            await appWindow.setPosition(new PhysicalPosition(Math.round(pos.x + widthDelta), Math.round(pos.y)));
        }

        // 更新当前宽度
        currentWidth.value = newWidth;
    } catch (error) {
        console.error('调整宽度失败:', error);
    }
};

const handleResizeEnd = () => {
    isResizing.value = false;
    resizeSide.value = null;
    document.removeEventListener('mousemove', handleResizeMove);
    document.removeEventListener('mouseup', handleResizeEnd);
};

// 保存用户自定义的宽度
const saveIslandWidth = () => {
    localStorage.setItem(NSD_ISLAND_WIDTH, String(currentWidth.value));
};

// 恢复用户自定义的宽度
const restoreIslandWidth = () => {
    const saved = localStorage.getItem(NSD_ISLAND_WIDTH);
    if (saved) {
        const width = parseInt(saved, 10);
        if (width >= MIN_WIDTH && width <= MAX_WIDTH) {
            return width;
        }
    }
    return null;
};

// 检测鼠标是否在灵动岛边缘（用于显示调整光标�?
const isNearEdge = (event: MouseEvent, side: 'left' | 'right'): boolean => {
    if (isPositionLocked.value) return false;

    const target = event.currentTarget as HTMLElement;
    if (!target) return false;

    const rect = target.getBoundingClientRect();
    const EDGE_THRESHOLD = 8; // 边缘检测阈值（像素）

    if (side === 'left') {
        return event.clientX - rect.left <= EDGE_THRESHOLD;
    } else {
        return rect.right - event.clientX <= EDGE_THRESHOLD;
    }
};

const handleMouseMove = async (event: MouseEvent) => {
    // 宽度调整模式
    if (isResizing.value) {
        await handleResizeMove(event);
        return;
    }

    // 检测鼠标是否在边缘区域（用于光标样式）
    if (canResize.value) {
        const target = event.currentTarget as HTMLElement;
        if (target) {
            const rect = target.getBoundingClientRect();
            const EDGE_THRESHOLD = 8;
            const leftDist = event.clientX - rect.left;
            const rightDist = rect.right - event.clientX;

            if (leftDist <= EDGE_THRESHOLD && leftDist >= 0) {
                mouseNearEdge.value = 'left';
            } else if (rightDist <= EDGE_THRESHOLD && rightDist >= 0) {
                mouseNearEdge.value = 'right';
            } else {
                mouseNearEdge.value = null;
            }
        }
    } else {
        mouseNearEdge.value = null;
    }

    if (!isMouseDown) return;

    // 1. 全局动画锁：任何变形动画期间，绝对禁止拖�?
    if (isSizeAnimating) return;

    // 2. 状态锁：音乐展开、消息通知、系统提示期间，统统禁止拖拽�?
    if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) {
        // 发现企图拖拽，立刻打断施�?
        isMouseDown = false;
        return;
    }

    // 3. 位置已锁定时，禁止一切拖�?
    if (isPositionLocked.value) return;

    // 4. 任务栏模�?+ 已解锁：仅允许横向拖拽（自定义实现，约束 Y 轴不变）
    if (isPinnedToTaskbar.value) {
        if (Math.abs(event.clientX - mouseDownX) > 5) {
            isMouseDown = false;
            await startCustomHorizontalDrag(event);
        }
        return;
    }

    // 5. 岛模�?+ 已解锁：自由拖拽（原�?startDragging，X/Y 均可移动�?
    if (Math.abs(event.clientX - mouseDownX) > 5 || Math.abs(event.clientY - mouseDownY) > 5) {
        isMouseDown = false;
        try {
            await getCurrentWindow().startDragging();
        } catch (error) {
            console.error('拖拽失败:', error);
        }
    }
};

const handleMouseUp = () => {
    // 宽度调整结束时保存宽�?
    if (isResizing.value) {
        handleResizeEnd();
        saveIslandWidth();
        return;
    }
    isMouseDown = false;
    handleCustomDragEnd();
};

// ===== 自定义横向拖拽（任务栏模式下仅允�?X 轴移动）=====
const startCustomHorizontalDrag = async (event: MouseEvent) => {
    try {
        const appWindow = getCurrentWindow();
        // 获取窗口当前物理坐标，作为拖拽起�?
        const pos = await appWindow.outerPosition();
        customDragStartWindowX = pos.x;
        customDragStartWindowY = pos.y;
        customDragStartScreenX = event.screenX;

        // 获取显示器信息与窗口宽度，用于边界约�?
        customDragMonitor = await currentMonitor();
        const size = await appWindow.innerSize();
        customDragWindowWidth = size.width;

        isCustomDragging = true;

        // 添加文档级监听器，确保鼠标移出灵动岛窗口后仍能持续追�?
        document.addEventListener('mousemove', handleCustomDragMove);
        document.addEventListener('mouseup', handleCustomDragEnd);
    } catch (e) {
        console.error('横向拖拽初始化失败', e);
    }
};

const handleCustomDragMove = async (event: MouseEvent) => {
    if (!isCustomDragging) return;

    const scaleFactor = window.devicePixelRatio;
    const deltaXLogical = event.screenX - customDragStartScreenX;
    const deltaXPhysical = deltaXLogical * scaleFactor;
    let newX = customDragStartWindowX + deltaXPhysical;

    // 边界约束：防止拖出屏幕左右边�?
    if (customDragMonitor) {
        const monitorLeft = customDragMonitor.position.x;
        const monitorRight = customDragMonitor.position.x + customDragMonitor.size.width;
        newX = Math.max(monitorLeft, Math.min(newX, monitorRight - customDragWindowWidth));
    }

    try {
        await getCurrentWindow().setPosition(
            new PhysicalPosition(Math.round(newX), Math.round(customDragStartWindowY))
        );
    } catch (e) {
        console.error('横向拖拽失败:', e);
    }
};

const handleCustomDragEnd = () => {
    if (!isCustomDragging) return;
    isCustomDragging = false;
    document.removeEventListener('mousemove', handleCustomDragMove);
    document.removeEventListener('mouseup', handleCustomDragEnd);
};

// ===== 位置持久化（锁定时保存，启动时恢复）=====
const saveIslandPosition = async () => {
    try {
        const appWindow = getCurrentWindow();
        const pos = await appWindow.outerPosition();
        localStorage.setItem(NSD_ISLAND_POSITION, JSON.stringify({ x: pos.x, y: pos.y }));
    } catch (e) {
        console.error('保存位置失败:', e);
    }
};

const restoreIslandPosition = async (): Promise<boolean> => {
    try {
        const saved = localStorage.getItem(NSD_ISLAND_POSITION);
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

const handleRightClick = async (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation(); // 阻止冒泡

    // 如果音乐灵动岛正在展开或已完全展开，强制禁止呼出右键菜�?
    if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) {
        return;
    }

    // 打开设置
    const openSettingsItem = await MenuItem.new({
        text: '打开设置',
        id: 'open_settings',
        action: async () => {
            await emit('open-settings-panel');
            showToast('已打开设置');
        }
    });

    // 切换流光边框
    const toggleGlowBorderItem = await MenuItem.new({
        text: isGlowBorderEnabled.value ? '关闭流光边框' : '开启流光边框',
        id: 'toggle_glow_border',
        enabled: true,
        action: () => {
            isGlowBorderEnabled.value = !isGlowBorderEnabled.value;
            localStorage.setItem(NSD_GLOW_BORDER, String(isGlowBorderEnabled.value));
            showToast(isGlowBorderEnabled.value ? '已开启流光边框' : '已关闭流光边框');
        }
    });

    // 重置位置
    const resetPositionItem = await MenuItem.new({
        text: isPinnedToTaskbar.value ? '重置位置 (已锁定)' : '重置位置',
        id: 'reset_position',
        enabled: !isPinnedToTaskbar.value,
        action: async () => {
            try {
                await adjustWindowPosition();
                // 如果已锁定，重置后重新保存新位置
                if (isPositionLocked.value) {
                    await saveIslandPosition();
                }
                showToast('已重置位置');
            } catch (error) {
                console.error(error);
            }
        }
    });

    // 重置宽度
    const resetWidthItem = await MenuItem.new({
        text: '重置宽度',
        id: 'reset_width',
        enabled: !isPositionLocked.value,
        action: async () => {
            try {
                // 删除保存的自定义宽度
                localStorage.removeItem(NSD_ISLAND_WIDTH);
                // 恢复到默认宽�?
                const { w, h } = getBaseSize();
                currentWidth.value = w;
                animateIslandSize(w, h);
                showToast('已重置宽度');
            } catch (error) {
                console.error(error);
            }
        }
    });

    // 锁定位置菜单�?
    const toggleLockItem = await MenuItem.new({
        text: isPositionLocked.value ? '解锁 (当前已锁定)' : '锁定',
        id: 'toggle_lock',
        enabled: !isPinnedToTaskbar.value,
        action: async () => {
            isPositionLocked.value = !isPositionLocked.value;
            localStorage.setItem(NSD_POSITION_LOCKED, String(isPositionLocked.value));
            // 锁定时保存当前位置和宽度，以便下次启动恢�?
            if (isPositionLocked.value) {
                await saveIslandPosition();
                saveIslandWidth();
            }
            // 同步状态给设置面板
            await emit('position-lock-sync', { locked: isPositionLocked.value });
            // 根据状态触�?lock �?unlock 专属通知
            showToast(
                isPositionLocked.value ? '锁定位置成功' : '位置已解锁',
                isPositionLocked.value ? 'lock' : 'unlock'
            );
        }
    });

    // 关闭灵动�?
    const closeItem = await MenuItem.new({
        text: '关闭',
        id: 'close',
        action: () => {
            isIslandVisible.value = false;
        }
    });

    // 使用客户端坐标转逻辑坐标（避免无边框裁剪带来的漂移）
    const position = new LogicalPosition(
        event.clientX,
        event.clientY
    );

    // 3. 创建菜单并按顺序追加进去
    const menu = await Menu.new();
    await menu.append(openSettingsItem);
    await menu.append(toggleGlowBorderItem);
    await menu.append(resetPositionItem);
    await menu.append(resetWidthItem);
    await menu.append(toggleLockItem);
    await menu.append(closeItem);

    // 4. 弹出菜单
    try {
        isMenuOpen.value = true; // 👈 弹出前，告诉系统菜单打开了
        await menu.popup(position);
    } catch (error) {
        console.error('菜单弹出失败:', error);
    } finally {
        isMenuOpen.value = false; // 👈 无论用户是点击了菜单，还是点空白处取消了，都会瞬间恢复置顶状态
    }
};

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

// 记录全局灵动岛是否正在执行形变动�?
let isSizeAnimating = false;
let sizeAnimTimer: number | null = null;

// 灵动岛核心代码！（完美防漂移+防裁�?防打断抖动）
const animateIslandSize = async (targetWidth: number, targetHeight: number) => {
    try {
        // 1. 触发形变前：立刻上锁
        isSizeAnimating = true;
        if (sizeAnimTimer) clearTimeout(sizeAnimTimer);

        // 2. 设定 500ms 后自动解锁（覆盖大多数弹簧动画的持续时间�?
        sizeAnimTimer = window.setTimeout(() => {
            isSizeAnimating = false;
        }, 500);

        const appWindow = getCurrentWindow();
        const realSize = await appWindow.innerSize();
        const scaleFactor = window.devicePixelRatio;

        const realStartW = realSize.width / scaleFactor;
        const realStartH = realSize.height / scaleFactor;

        await invoke('start_island_animation', {
            startWidth: realStartW,
            startHeight: realStartH,
            targetWidth: targetWidth,
            targetHeight: targetHeight,
            isPinned: isPinnedToTaskbar.value
        });
    } catch (err) {
        console.error('呼叫 Rust 动画失败:', err);
        // 如果调用失败，安全起见立刻解锁，防止死锁
        isSizeAnimating = false;
    }
};

// ===== F11 通知点击打开：关闭通知显示并启动来源应�?=====
// 关闭消息通知，恢复灵动岛到通知弹出前的状�?
const dismissMsgNotification = () => {
    if (!isMsgActive.value) return;
    if (msgTimer) {
        clearTimeout(msgTimer);
        msgTimer = null;
    }
    isMsgActive.value = false;
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
    // 手动关闭通知后重新评估自动隐藏
    scheduleAutoHide();
};

// 点击灵动岛上的通知：立即返回通知弹出前状态，并打开来源应用
const handleNotificationClick = async () => {
    const aumid = msgAumid.value;
    // 先关闭通知显示，恢复灵动岛状�?
    dismissMsgNotification();
    // 再启动来源应�?
    if (aumid) {
        try {
            await invoke('launch_app_by_aumid', { aumid });
        } catch (e) {
            console.error('打开来源应用失败:', e);
        }
    }
};

// 动画锁与等待队列标志
let isAnimationLocked = false;
let isPendingCollapse = false;

// 音乐控制器自动收缩方�?
const collapseMusic = () => {
    if (!isMusicExpanded.value && !isMusicExpanding.value) return;

    // 【核心逻辑】：如果正在猛烈展开中，绝对不打断！把收缩请求挂起，等它展开完自动执行�?
    if (isAnimationLocked) {
        isPendingCollapse = true;
        return;
    }

    isMusicExpanded.value = false;
    isMusicExpanding.value = false;
    isPendingCollapse = false; // 清除队列

    if (musicExpandAnimTimer) {
        clearTimeout(musicExpandAnimTimer);
        musicExpandAnimTimer = null;
    }

    // 折叠时恢复用户自定义的宽度，而不是使用默认宽�?
    const { h } = getBaseSize();
    const savedWidth = restoreIslandWidth();
    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
    animateIslandSize(targetWidth, h);
};

// force_window_topmost：窗口 blur 事件驱动置顶（替代原 speedTimer 中每 800ms 轮询）
const handleForceTopmost = () => {
    if (isPinnedToTaskbar.value && isIslandVisible.value && !isMenuOpen.value && !isCustomDragging) {
        invoke('force_window_topmost').catch(() => { });
    }
};

// 音乐控制器点击展开方法
const expandMusic = (e: MouseEvent) => {
    if (Math.abs(e.clientX - mouseDownX) > 5 || Math.abs(e.clientY - mouseDownY) > 5) return;
    if ((e.target as HTMLElement).closest('.ctl-btn')) return;

    if (isMusicExpanded.value || isMusicExpanding.value) return;

    isMusicExpanding.value = true;
    isPendingCollapse = false;  // 重置待办任务
    isAnimationLocked = true;   // 上锁！宣布进入神圣不可侵犯的展开周期

    // 1. 弹性按压动�?(先微微变�?
    animateIslandSize(245, 38);

    // 2. 延迟 120 毫秒后，打断缩小，直接猛烈展开
    musicExpandAnimTimer = window.setTimeout(() => {
        isMusicExpanded.value = true;
        isMusicExpanding.value = false;
        animateIslandSize(320, 150);

        // 3. 根据 Rust 端的弹簧衰减频率，约 400ms 后动画彻底结束，此时解锁
        setTimeout(() => {
            isAnimationLocked = false;

            // 检查：如果在展开的这 520ms 里，用户鼠标已经移走了，那就立刻补发收缩命令�?
            if (isPendingCollapse) {
                isPendingCollapse = false;
                collapseMusic();
            }
        }, 400);
    }, 120);
};

// 鼠标离开灵动岛时：自动折叠或自动隐藏
const handleMouseLeave = () => {
    // 清除鼠标边缘检测状�?
    mouseNearEdge.value = null;
    isMouseOver.value = false;

    // 1. 自动折叠逻辑：当灵动岛展开时，鼠标离开后延迟折叠回小岛状�?
    if (isAutoCollapseEnabled.value && (isMusicExpanded.value || isMusicExpanding.value)) {
        // 启动自动折叠定时�?
        if (autoCollapseTimer) {
            clearTimeout(autoCollapseTimer);
            autoCollapseTimer = null;
        }
        autoCollapseTimer = window.setTimeout(() => {
            if (!isMouseOver.value && (isMusicExpanded.value || isMusicExpanding.value)) {
                collapseMusic();
            }
        }, autoCollapseDelay.value);
    }

    // 2. 自动隐藏逻辑：统一交给 scheduleAutoHide 内部守卫判断
    //    仅在「自动隐藏开关开启 + 音乐控制器模式打开 + 无音乐播放」时才隐藏
    scheduleAutoHide();
};

// 鼠标重新移入灵动岛时：立刻打断收缩企�?
const handleMouseEnter = () => {
    // 如果之前移出留下了收缩案底，但动画还没播完鼠标又回来了，直接取消这个案底
    isPendingCollapse = false;
    isMouseOver.value = true;

    // 取消自动隐藏定时�?
    if (autoHideTimer) {
        clearTimeout(autoHideTimer);
        autoHideTimer = null;
    }

    // 取消自动折叠定时�?
    if (autoCollapseTimer) {
        clearTimeout(autoCollapseTimer);
        autoCollapseTimer = null;
    }
};

watch(displayMusic, (newVal: boolean) => {
    if (!newVal) {
        collapseMusic(); // 一旦音乐岛被隐藏（不管是因为轮换还是手动关了），立刻收缩
    }
});

// 引入你的默认图标作为兜底
import defaultLogo from '../assets/logo.png';
const currentMsgIcon = ref(defaultLogo);

// 图标映射�?
const getAppIcon = (appName: string) => {
    const name = appName.toLowerCase();

    if (name.includes('qq')) {
        // 使用 new URL �?Vite 知道你要引入这个资源
        return new URL('../assets/qq.png', import.meta.url).href;
    }
    if (name.includes('钉钉') || name.includes('dingtalk')) {
        return new URL('../assets/dingtalk.png', import.meta.url).href;
    }
    if (name.includes('mail') || name.includes('邮件')) {
        return new URL('../assets/mail.png', import.meta.url).href;
    }
    if (name.includes('wechat') || name.includes('微信')) {
        return new URL('../assets/wechat.png', import.meta.url).href;
    }

    return defaultLogo;
};

onMounted(async () => {
    // widget 可能在主面板未创建或省内存销毁后独立运行，需自行恢复目标播放器�?
    await invoke('set_target_player', {
        player: localStorage.getItem(NSD_TARGET_PLAYER) || 'netease',
    }).catch(() => {});

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
            // 判断是不�?首次"（本地有没有存过流光边框的数据）
            if (localStorage.getItem(NSD_GLOW_BORDER) === null) {
                isGlowBorderEnabled.value = true; // 自动开启流光边框
                localStorage.setItem(NSD_GLOW_BORDER, 'true'); // 存入记忆，以后就不算"首次"了
            }

            showInfo.value = false;
            musicBoxKey.value++;

            // 音乐控制器开启时，如果没有音乐播放，延迟隐藏灵动岛
            // scheduleAutoHide 内部会校验全部条件
            scheduleAutoHide();
        }
    });

    // 监听系统底层事件（音量、电源）
    await listen<string>('system-event', (event) => {
        showToast(event.payload, 'sys');
    });

    await listen<{ state: 'charging' | 'discharging', percent: number }>('battery-event', (event) => {
        const { state, percent } = event.payload;

        if (state === 'charging') {
            showToast(`已接入电源，当前电量 ${percent}%`, 'battery-charge');
        } else if (state === 'discharging' && percent <= 20) {
            // 这里还可以加入防抖：只在刚掉�?20%�?0%�?% 等关键节点触发一次，避免疯狂弹窗
            showToast(`电池电量低，剩余 ${percent}%`, 'battery-low');
        }
    });

    // 监听来自控制台的透明度同步指�?
    await listen<{ opacity: number }>('control-island-opacity', (event) => {
        islandOpacity.value = event.payload.opacity;
    });

    // 监听来自控制台的主题同步指令
    await listen<{ theme: string }>('control-island-theme', (event) => {
        islandTheme.value = event.payload.theme;
    });

    // 监听置于任务栏开�?
    await listen<{ enabled: boolean }>('control-pin-taskbar', async (event) => {
        isPinnedToTaskbar.value = event.payload.enabled;
        if (isPinnedToTaskbar.value) {
            await snapToBottomLeft(); // 开启时：飞到左下角
        } else {
            await adjustWindowPosition(); // 关闭时：等同于点"重置位置"，飞回顶部居�?
        }
        // 如果位置已锁定，模式切换后重新保存新位置（避免下次启动恢复到过期坐标�?
        if (isPositionLocked.value) {
            await saveIslandPosition();
        }
    });

    // 监听来自设置面板的位置锁定信�?
    await listen<{ locked: boolean }>('control-position-lock', async (event) => {
        isPositionLocked.value = event.payload.locked;
        // 锁定时保存当前位置，以便下次启动恢复
        if (isPositionLocked.value) {
            await saveIslandPosition();
        }
    });

    // 监听消息模式开�?
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

    // 监听轮换模式开�?
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
        localStorage.setItem(NSD_AUTO_HIDE_ENABLED, String(isAutoHideEnabled.value));
        localStorage.setItem(NSD_AUTO_HIDE_DELAY, String(autoHideDelay.value));
    });

    // 监听全屏自动隐藏设置
    await listen<{ enabled: boolean }>('control-autohide-fs', (event) => {
        isAutoHideFullscreen.value = event.payload.enabled;
    });

    // 监听 Rust 发来的全屏状态变化
    await listen<boolean>('fullscreen-changed', async (event) => {
        const isFullscreen = event.payload;
        if (!isAutoHideFullscreen.value) return;
        if (isFullscreen) {
            if (isIslandVisible.value) {
                wasVisibleBeforeFullscreen = true;
                isHidingForFullscreen = true;
                isIslandVisible.value = false;
            }
        } else {
            if (wasVisibleBeforeFullscreen) {
                await getCurrentWindow().show();
                setTimeout(() => {
                    isIslandVisible.value = true;
                    emit('island-status-sync', { visible: true });
                }, 40);
                wasVisibleBeforeFullscreen = false;
            }
        }
    });

    // 监听后端番茄钟 tick 事件
    await listen<any>('pomodoro-tick', async (event) => {
        const p = event.payload;
        if (p.active === false) {
            // 番茄钟结束 → 隐藏
            isPomodoroVisible.value = false;
            isPomodoroExpanded.value = false;
            localStorage.setItem(NSD_POMODORO_VISIBLE, 'false');
            if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
                const { h } = getBaseSize();
                const savedWidth = restoreIslandWidth();
                const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                animateIslandSize(targetWidth, h);
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
            localStorage.setItem(NSD_POMODORO_VISIBLE, 'true');
        }
        // 暂停时不调整展开状态，但不影响显示
    });

    // 监听番茄钟阶段切换事件（用于显示 toast 提示）
    await listen<any>('pomodoro-phase-change', async (event) => {
        const p = event.payload;
        if (p.phase === 'break') {
            showToast('专注结束，休息一下吧！', 'app');
        } else {
            showToast('休息结束，继续专注！', 'app');
        }
    });

    // 监听番茄钟完成事件
    await listen<any>('pomodoro-complete', async () => {
        showToast('所有番茄钟已完成！🎉', 'app');
    });

    // 监听倒计时 tick 事件
    await listen<any>('countdown-tick', async (event) => {
        const p = event.payload;
        if (p.active === false && p.phase === 'idle') {
            isCountdownVisible.value = false;
            isCountdownExpanded.value = false;
            isCountdownFinished.value = false;
            cdPaused.value = false;
            localStorage.setItem(NSD_COUNTDOWN_VISIBLE, 'false');
            if (!isMsgActive.value && !displaySysToast.value && !isMusicExpanded.value && !isMusicExpanding.value) {
                const { h } = getBaseSize();
                const savedWidth = restoreIslandWidth();
                const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                animateIslandSize(targetWidth, h);
            }
            return;
        }
        countdownRemainingSecs.value = p.remaining_secs;
        cdPaused.value = p.paused || false;
        isCountdownFinished.value = p.phase === 'finished';
        if (!isCountdownVisible.value) {
            isCountdownVisible.value = true;
            localStorage.setItem(NSD_COUNTDOWN_VISIBLE, 'true');
        }
    });

    // 监听倒计时完成事件
    await listen<any>('countdown-complete', async () => {
        isCountdownFinished.value = true;
        showToast('⏰ 倒计时结束', 'app');
    });

    // 监听实时活动控制台指令（非番茄钟活动）

    // 监听自动折叠设置
    await listen<{ enabled: boolean, delay: number }>('control-auto-collapse', (event) => {
        isAutoCollapseEnabled.value = event.payload.enabled;
        autoCollapseDelay.value = event.payload.delay;
        localStorage.setItem(NSD_AUTO_COLLAPSE_ENABLED, String(isAutoCollapseEnabled.value));
        localStorage.setItem(NSD_AUTO_COLLAPSE_DELAY, String(autoCollapseDelay.value));
    });

    // 启动时如果开了轮换，就跑起来
    if (isRotationEnabled.value) {
        startRotation();
    }

    // 初始化位置追�?
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

    // 立即设置窗口大小，确保宽度恢复生�?
    try {
        const appWindow = getCurrentWindow();
        const scaleFactor = window.devicePixelRatio;
        await appWindow.setSize(new PhysicalSize(Math.ceil(currentWidth.value * scaleFactor), Math.ceil(currentHeight.value * scaleFactor)));
    } catch (error) {
        console.error('设置初始窗口大小失败:', error);
    }

    // 根据本地记录决定启动时出现在�?
    if (isPositionLocked.value) {
        // 已锁定位置：尝试恢复上次保存的坐�?
        const restored = await restoreIslandPosition();
        if (!restored) {
            // 没有保存过位置，回退到默认定�?
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

    // 先显示透明�?Tauri 窗口，再触发 Vue 的灵动岛入场弹簧动画
    // 如果没开消息模式，才在启动时直接显示灵动�?
    if (!isMsgModeEnabled.value) {
        await getCurrentWindow().show();
        isIslandVisible.value = true;
    }

    // 监听来自 LiveActive 的硬件监控开关（跨窗口事件，统一由 NSD_HW_ENABLED 驱动）
    await listen<{ enabled: boolean }>('control-hardware-mon', (event) => {
        hwEnabled.value = event.payload.enabled;
        localStorage.setItem(NSD_HW_ENABLED, String(event.payload.enabled));
    });

    // 监听后端推送的 monitor-stats 事件（硬件 + 网速统一）
    await listen<any>('monitor-stats', (event) => {
        const p = event.payload;
        if (typeof p.cpu_pct === 'number') hwCpuPct.value = p.cpu_pct;
        if (typeof p.mem_pct === 'number') hwMemPct.value = p.mem_pct;
        if (typeof p.download_speed === 'number') {
            downloadSpeed.value = formatSpeed(p.download_speed);
        }
        if (typeof p.upload_speed === 'number') {
            uploadSpeed.value = formatSpeed(p.upload_speed);
        }
        if (typeof p.download_bytes === 'number' && typeof p.upload_bytes === 'number') {
            // 高流量指示
            const rxDiff = p.download_speed || 0;
            const txDiff = p.upload_speed || 0;
            isHighDownload.value = rxDiff >= 1024 * 1024;
            isHighUpload.value = txDiff >= 1024 * 1024;
        }
    });

    fetchSpeedStats();
    checkNetworkLatency();

    checkNetworkLatency();

    // 启动网速显示轮换定时器（每 5 秒切换上传/下载）
    speedCycleTimer = window.setInterval(() => {
        if (displaySpeed.value) {
            isShowingUpload.value = !isShowingUpload.value;
        }
    }, 5000);

    // 在挂载时通知后端是否 emit（仅当硬件监控开启时推送）
    invoke('set_hardware_emit', { enabled: hwEnabled.value }).catch(() => {});

    // 启动网速轮询定时器（硬件已迁移至推送，不再在此轮询）
    speedTimer = setInterval(async () => {
        fetchSpeedStats();
    }, 1500) as unknown as number;


    // 2. 中频定时器：专门负责音乐状态同步（每 3000ms 刷新一次即可）
    musicTimer = setInterval(() => {
        if (isMusicCtlEnabled.value || isRotationEnabled.value) {
            syncMusicStatus();
        }
    }, 3000);


    // 3. 低频定时器：专门轮询系统通知（通知不需要抢时间，5秒换来极低的资源占用）
    notifyTimer = setInterval(async () => {
        const enabled = localStorage.getItem(NSD_MSG_NOTIFY) === 'true';
        if (!enabled) return;

        try {
            const res = await invoke<any>('fetch_latest_notification');
            if (res) {
                msgAumid.value = res.aumid;

                // 标题只存发送者（如果没有单独标题就显�?'新通知'�?
                msgTitle.value = (res.title && res.title !== res.app_name) ? res.title : '新通知';
                // 单独把程序名存起�?
                msgAppName.value = res.app_name;
                // 内容兜底逻辑保持不变
                msgBody.value = res.body || (res.title === res.app_name ? '收到一条新通知' : res.title);

                currentMsgIcon.value = getAppIcon(res.app_name);

                if (!isMsgActive.value) {
                    isMsgActive.value = true;
                    // 自动恢复显示：当有消息活动时，如果灵动岛被隐藏，则自动恢复显�?
                    if (!isIslandVisible.value) {
                        getCurrentWindow().show();
                        isIslandVisible.value = true;
                    }
                    if (isMsgModeEnabled.value && !isIslandVisible.value) {
                        getCurrentWindow().show();
                        isIslandVisible.value = true;
                    }
                    if (!isPinnedToTaskbar.value) {
                        animateIslandSize(360, 65);
                    }
                }

                if (msgTimer) clearTimeout(msgTimer);
                msgTimer = setTimeout(() => {
                    isMsgActive.value = false;
                    const { h } = getBaseSize();
                    const savedWidth = restoreIslandWidth();
                    const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                    animateIslandSize(targetWidth, h);
                    // 消息通知结束后重新评估自动隐藏
                    // scheduleAutoHide 内部会校验：自动隐藏开关 + 音乐控制器模式 + 无音乐播放
                    scheduleAutoHide();
                }, 5000);
            }
        } catch (err) {
            console.error(err);
        }
    }, 5000);

    // 调大Ping间隔：从5.5秒调大到10秒
    pingTimer = setInterval(checkNetworkLatency, 10000) as unknown as number;

    // 监听控制台发来的显隐调度指令
    await listen<{ show: boolean }>('control-island-visibility', async (event) => {
        if (event.payload.show) {
            // 1. 先让透明�?OS 窗口容器显示，此时内�?DOM �?v-show="false"，视觉上仍是隐形�?
            await getCurrentWindow().show();
            await getCurrentWindow().setAlwaysOnTop(true);
            // 2. 给予 40ms 的浏览器渲染帧缓冲，再撕开 Vue �?v-show 状态，强制触发 enter 动画
            setTimeout(() => {
                isIslandVisible.value = true;
            }, 40);
        } else {
            // 控制台关闭指�?-> 触发常规离开动画
            isIslandVisible.value = false;
        }
    });

    // 实时监听来自 Rust 底层发来的清透像素流，无缝同步给 Vue 的响应式 DOM 宽高
    await listen<number[]>("island-resize", (event) => {
        const [w, h] = event.payload;
        currentWidth.value = w;
        currentHeight.value = h;
    });

    // B8: 监听后端推来的频谱数据（替代 50ms setInterval 轮询，显著减�?IPC 调用次数�?
    await listen<number[]>("spectrum-data", (event) => {
        spectrumData.value = event.payload;
    });

    // 初始化时触发一次计�?
    setTimeout(() => {
        calculateScroll();
    }, 700);

    // 恢复番茄钟运行状态：查询后端当前状态
    try {
        const state: any = await invoke('get_pomodoro_state');
        if (state.active) {
            pomodoroRemainingSecs.value = state.remaining_secs;
            pomodoroPhase.value = state.phase;
            pomodoroRemainingCycles.value = state.remaining_cycles;
            isPomodoroVisible.value = true;
            localStorage.setItem(NSD_POMODORO_VISIBLE, 'true');
        }
    } catch (_e) {}

    // 恢复倒计时运行状态：查询后端当前状态
    try {
        const state: any = await invoke('get_countdown_state');
        if (state.active) {
            countdownRemainingSecs.value = state.remaining_secs;
            cdPaused.value = state.paused || false;
            isCountdownFinished.value = state.phase === 'finished';
            isCountdownVisible.value = true;
            localStorage.setItem(NSD_COUNTDOWN_VISIBLE, 'true');
        }
    } catch (_e) {}
});

onUnmounted(() => {
    window.removeEventListener('blur', collapseMusic);
    window.removeEventListener('blur', handleForceTopmost);
    // 清理自定义横向拖拽的文档级监听器
    document.removeEventListener('mousemove', handleCustomDragMove);
    document.removeEventListener('mouseup', handleCustomDragEnd);
    clearInterval(speedTimer);
    clearInterval(pingTimer);
    stopRotation();
    clearInterval(musicTimer);
    clearInterval(notifyTimer);
    stopProgressTimer();
    // 组件卸载时关闭频谱捕获，避免后端空跑
    invoke('set_spectrum_active', { active: false }).catch(() => {});
    if (speedCycleTimer) clearInterval(speedCycleTimer);
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

/* 外层包裹层：负责裁切多余的流�?*/
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

/* 隐藏在底层的巨大旋转渐变�?*/
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

    /* 10秒一圈刚刚好，柔和且不怎么�?GPU */
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

/* 顺时针匀速旋�?*/
@keyframes rainbow-rotate {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}

[data-tauri-drag-region] {
    -webkit-app-region: drag;
    cursor: grab;
}

[data-tauri-drag-region]:active {
    cursor: grabbing;
}

/* 修改网速盒子布局，强制靠左，并加入左侧内边距 */
.speed-box {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    height: 100%;
}

.speed-item {
    display: flex;
    align-items: center;
    gap: 6px;
    /* 稍微拉开箭头和数字的距离 */
    transform: translateY(-1px);
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
}

.label {
    font-size: 10px;
    /* 稍微调大箭头 */
    color: currentColor;
    opacity: 0.5;
    font-weight: 800;
    padding: 2px 5px;
    border-radius: 4px;
    transition: all 0.3s ease;
    background: rgba(150, 150, 150, 0.15);
    /* 默认给一个淡淡的底色，增加质�?*/
}

/* 高流量时�?label 样式 */
.label.high-traffic {
    color: currentColor;
    opacity: 1;
    background: rgba(255, 255, 255, 0.25);
}

:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .label.high-traffic {
    background: rgba(0, 0, 0, 0.15);
}

.value {
    font-size: 12px;
    transform: translateY(-0.5px);
    font-weight: 600;
    letter-spacing: 0.2px;
    font-variant-numeric: tabular-nums;
    min-width: 65px;
    text-align: left;
}

/* 网速轮换的淡入淡出动画 */
.speed-fade-enter-active,
.speed-fade-leave-active {
    transition: opacity 0.3s ease, transform 0.3s ease;
}

.speed-fade-enter-from {
    opacity: 0;
    transform: translateY(4px);
    /* 微微从下方滑�?*/
}

.speed-fade-leave-to {
    opacity: 0;
    transform: translateY(-4px);
    /* 微微向上滑出 */
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    transition: background-color 0.4s ease;
}

/* 去掉发光阴影，改为纯粹的扁平化圆点，干净利落 */
.good {
    background-color: #34C759;
}

.warning {
    background-color: #FFCC00;
}

.error {
    background-color: #FF3B30;
}

/* 让两个盒子脱离彼此的影响，在同一个包裹层内完美的“重叠”放�?*/
.music-ctl-box,
.speed-box {
    position: absolute;
    /* 改为绝对定位，实现无缝平�?*/
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    justify-content: center;
    gap: 4px;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
}

/* 硬件监控附属图标（拆分模式右侧圆钮，无背景填充，图标由 .hw-chip-svg 提供颜色） */
.hw-right-circle {
    cursor: pointer;
}

.hw-chip-svg {
    width: 22px;
    height: 22px;
    color: #3b82f6;
    transition: color 0.3s ease;
}

.hw-chip-svg.high {
    color: #ff4757;
}

/* 硬件监控展开态：CPU / 内存 详情 */
.hw-expanded-detail {
    display: flex;
    align-items: center;
    gap: 16px;
    position: relative;
    width: 100%;
    height: 100%;
    padding-left: 16px;
    padding-right: 34px;
}

.hw-detail-row {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1.15;
}

.hw-detail-label {
    font-size: 9px;
    font-weight: 700;
    opacity: 0.6;
    letter-spacing: 0.5px;
}

.hw-detail-val {
    font-size: 15px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.5px;
}

.hw-detail-val.high {
    color: #ff4757;
}

.hw-close-btn-x {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: #888;
    cursor: pointer;
    transition: all 0.2s ease;
}

.hw-close-btn-x:hover {
    color: #ff4757;
    background-color: rgba(255, 71, 87, 0.15);
}

.hw-close-btn-x svg {
    width: 14px;
    height: 14px;
}
.music-ctl-box {
    justify-content: flex-start;
}

/* 增加统一的内部绝对定位平替包裹层 */
.inner-wrapper {
    position: relative;
    flex-grow: 1;
    height: 100%;
    display: flex;
    align-items: center;
}

.album-cover {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    box-sizing: unset !important;
    border: 2px solid rgba(255, 255, 255, 0.5) !important;
    background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.250);
    transition: all 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    z-index: 2;
    transform: translateX(-8px);
}

/* 亮色模式下的外环颜色自动变暗 */
:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .album-cover {
    border-color: rgba(0, 0, 0, 0.15);
}

.album-cover.is-playing {
    transform: scale(1.08) translateX(-8px);
}

/* 封面内部绑定背景图的 div */
.cover-inner {
    width: 100%;
    height: 100%;
    background-position: center;
    background-repeat: no-repeat;
    background-size: cover;
    transition: background-image 0.3s ease;
    animation: rotate 8s linear infinite;
    animation-play-state: paused;
    /* 默认让动画处于暂停状�?*/
}

/* 正在播放时的旋转动画 */
.is-playing .cover-inner {
    animation-play-state: running;
    /* 当有播放状态时，让动画跑起�?*/
}

@keyframes rotate {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}

.music-controls {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    align-items: center;
    gap: 12px;
    z-index: 10;
}

.ctl-btn {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    border-radius: 50%;
    transition: background-color 0.2s ease, opacity 0.2s ease, transform 0.1s ease;
    outline: none;
    -webkit-app-region: no-drag;
}

/* 只有�?hover 的时候才出现背景�?*/
.ctl-btn:hover {
    background-color: rgba(255, 255, 255, 0.15);
}

:deep(.island-container[style*="background-color: rgba(255, 255, 255"]) .ctl-btn:hover {
    background-color: rgba(0, 0, 0, 0.1);
}

.ctl-btn:active {
    opacity: 0.6;
    transform: scale(0.92);
}

.ctl-btn svg {
    width: 16px;
    height: 16px;
    pointer-events: none;
}

/* 播放键稍微比切歌键大一点点，突出视觉中�?*/
.play-btn svg {
    width: 20px;
    height: 20px;
}

/* 控件显隐淡入淡出动画过渡 */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.25s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

/* 歌曲信息遮罩容器：挨着封面靠左，占据右侧剩余空�?*/
.music-info-mask-box {
    position: absolute;
    left: 30px;
    right: 18px;
    height: 100%;
    display: flex;
    align-items: center;
    overflow: hidden;
    padding-left: 0;
    -webkit-app-region: no-drag;
    transform: translateY(-1px) translateX(-0.5px);
    mask-image: linear-gradient(to right, #000000 75%, transparent 100%);
    -webkit-mask-image: linear-gradient(to right, #000000 75%, transparent 100%);
}

/* 歌曲文本基础样式 */
.music-info-text {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    /* 强制单行不换�?*/
    overflow: hidden;
    color: inherit;
    opacity: 0.9;
}

/* 灵动岛消息通知样式 */
.msg-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    padding: 0 45px 0 0px;
    box-sizing: border-box;
    z-index: 10;
    gap: 12px;
    -webkit-app-region: no-drag;
    transition: opacity 0.15s ease;
}

/* F11 通知可点击：hover 时给一点视觉反�?*/
.msg-box:hover {
    opacity: 0.8;
}

.msg-box:active {
    opacity: 0.65;
}

/* 预制消息图标/头像样式 */
.msg-avatar {
    width: 35px;
    height: 35px;
    border-radius: 50%;
    background: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.msg-avatar-img {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    object-fit: cover;
}

/* 文本靠左对齐包裹�?*/
.msg-text-wrapper {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    overflow: hidden;
    flex-grow: 1;
}

/* 消息弹窗容器 */
.msg-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 700;
    line-height: 1.4;
    width: 100%;
    overflow: hidden;
}

/* 发送者昵称（允许超长省略号） */
.sender-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* 尾部的程序名 */
.app-name {
    font-size: 10.5px;
    font-weight: 600;
    flex-shrink: 0;
    padding: 2px 6px;
    border-radius: 6px;
    background-color: rgba(150, 150, 150, 0.25);
    color: inherit;
    opacity: 0.9;
    letter-spacing: 0.2px;
    transform: translateY(-0.5px);
}

/* 调大后的内容样式 */
.msg-body {
    font-size: 12.5px;
    line-height: 1.4;
    opacity: 0.75;
    text-align: left;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.value.high-usage {
    color: #f06861 !important;
}


/* 音乐律动频谱样式 */
.audio-spectrum {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 12px;
    padding-right: 2px;
}

/* 暂停状态下的竖线（统一高度�?*/
.audio-spectrum .bar {
    width: 2px;
    height: 18px;
    background-color: #b6e0ee;
    border-radius: 3px;
    transform-origin: center;
    /* 改用极速的 ease-out 过渡，让前端完美衔接后端的帧�?*/
    transition: transform 0.08s ease-out;
    will-change: transform;
}

.music-ctl-box {
    transition: opacity 0.2s ease !important;
}

.music-ctl-box.expanded {
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-start;
    padding: 0 !important;
}

/* 顶部容器：取�?all 过渡，让它跟着 Rust 窗口的拉伸严丝合缝地重排 */
.music-top-row {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    position: relative;
    transition: none !important;
    /* 核心防抖魔法，取�?CSS 的挣�?*/
}

.music-ctl-box.expanded .music-top-row {
    height: 40px;
    margin-top: 14px !important;
    margin-left: 5px !important;
    border: none;
}

/* 封面：覆盖掉上面�?transition: all，只保留变形和圆角的过渡 */
.album-cover {
    transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.2), border-radius 0.3s ease !important;
}

.music-ctl-box.expanded .album-cover {
    width: 40px !important;
    height: 40px !important;
    border-radius: 6px !important;
    animation: none !important;
    border: none;
    transform: translateX(0px) rotate(0deg) !important;
}

.music-ctl-box.expanded .album-cover .cover-inner {
    animation: none !important;
    transform: rotate(0deg) !important;
    border: none;
}

.music-ctl-box.expanded .album-cover.is-playing {
    border: none;
    transform: scale(1.05) translateX(0px) rotate(0deg) !important;
}

/* 歌曲文本遮罩：取消过渡，随窗口大小瞬间变�?*/
.music-ctl-box.expanded .music-info-mask-box {
    left: 60px !important;
    right: 55px !important;
    display: flex !important;
    align-items: center !important;
    justify-content: flex-start !important;
    transition: none !important;
}

/* 你的两套文字过渡逻辑非常完美，全部保留原样（因为 opacity 不影响排版） */
.music-info-text {
    position: absolute;
    left: 0 !important;
    top: 50%;
    width: 100%;
    transform: translateY(-50%);
    transition: opacity 0.3s ease, transform 0.3s ease;
    text-align: left !important;
    display: flex !important;
    flex-direction: column !important;
    align-items: flex-start !important;
}

.double-line {
    opacity: 0;
    pointer-events: none;
    transform: translateY(-30%);
}

.single-line {
    opacity: 1;
    align-items: center;
    text-align: center;
}

.single-line.fade-out {
    opacity: 0;
    pointer-events: none;
    transform: translateY(20%);
}

.double-line.fade-in {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(-50%) !important;
}

.song-title {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.2;
    width: 100%;
    text-align: left !important;
}

.song-artist {
    font-size: 12.5px;
    opacity: 0.65;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.2;
    width: 100%;
    text-align: left !important;
}

/* 媒体控件与频�?*/
.music-ctl-box.expanded .music-controls {
    position: absolute;
    left: 50%;
    transform: translateX(-50%) translateY(5px);
    width: 100%;
    display: flex;
    justify-content: center;
    gap: 20px;
}

.music-ctl-box.expanded .ctl-btn svg {
    width: 22px;
    height: 22px;
}

.music-ctl-box.expanded .play-btn svg {
    width: 28px;
    height: 28px;
}

.audio-spectrum.expanded {
    position: absolute;
    right: 18px !important;
    top: 27px !important;
    transform: scale(1.3);
    /* �?all 换成具体的属性，防止抖动 */
    transition: opacity 0.3s ease, transform 0.3s ease !important;
}

/* F6 音乐进度�?*/
.music-progress {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 2;
}

.progress-time-row {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    opacity: 0.85;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    letter-spacing: 0.3px;
}

.progress-bar {
    position: relative;
    height: 4px;
    background: rgba(128, 128, 128, 0.25);
    border-radius: 2px;
    cursor: pointer;
    touch-action: none;
    user-select: none;
    transition: height 0.15s ease;
}

.progress-bar:hover {
    height: 6px;
}

.progress-bar.disabled {
    cursor: not-allowed;
    opacity: 0.55;
}

.progress-bar.disabled:hover {
    height: 4px;
}

.progress-bar.disabled .progress-thumb {
    display: none;
}

.progress-filled {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    background: currentColor;
    border-radius: 2px;
    transition: width 0.1s linear;
}

.progress-filled.dragging {
    transition: none;
}

.progress-thumb {
    position: absolute;
    top: 50%;
    width: 10px;
    height: 10px;
    background: currentColor;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.35);
    transition: width 0.15s ease, height 0.15s ease;
}

.progress-bar:hover .progress-thumb {
    width: 12px;
    height: 12px;
}

.progress-remaining {
    font-size: 9.5px;
    opacity: 0.55;
    text-align: center;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.progress-placeholder {
    min-height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10.5px;
    opacity: 0.55;
    letter-spacing: 0.2px;
}

/* 强制靠左对齐，干掉原本的 align-items: center。否则长文本会向两边溢出，导致开头被�?*/
.music-info-text.single-line {
    overflow: visible !important;
    align-items: flex-start !important;
    text-align: left !important;
}

/* 滚动的内部容�?*/
.scroll-inner {
    display: inline-block;
    white-space: nowrap;
    width: max-content;
    flex-shrink: 0;
    backface-visibility: hidden;
    transform: translateZ(0);
    -webkit-font-smoothing: antialiased;
    transform-style: preserve-3d;
}

/* 挂载动画 */
.scroll-inner.is-scrolling {
    animation: scroll-ping-pong var(--scroll-duration) linear infinite alternate;
}

/* 滚动动画帧：利用 0-20% �?80-100% 的区间实现两端停�?*/
@keyframes scroll-ping-pong {

    0%,
    20% {
        transform: translateX(0);
    }

    80%,
    100% {
        /* JS 里已经拼好了 px 单位，这里直�?-1 乘过去即�?*/
        transform: translateX(calc(-1 * var(--scroll-dist)));
    }
}

/* 系统操作通知样式 */
.system-toast-box {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    padding-left: 0;
    z-index: 10;
    -webkit-app-region: no-drag;
}

.toast-icon {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transform: translateX(-8px);
}

/* 灵动岛通知 */
.toast-icon.app-icon {
    color: currentColor;
}

/* 系统通知使用跟随字体的原生对比色 (黑白) */
.toast-icon.sys-icon {
    color: currentColor;
    opacity: 0.85;
}

.toast-icon svg {
    width: 22px;
    height: 22px;
    display: block;
}

.toast-icon.battery-charge-icon {
    color: #34C759;
}

.toast-icon.battery-low-icon {
    color: #FF3B30;
}

.toast-text {
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0.95;
    transform: translateX(-2px) translateY(-1px);
}

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

/* 展开状态下调整手柄的圆�?*/
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

/* 右侧独立实时小球 */
.right-circle {
    position: absolute;
    right: 0;
    width: 38px;
    height: 38px;
    border-radius: 50% !important;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.05);
}

.pop-enter-active,
.pop-leave-active {
    transition: all 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.pop-enter-from,
.pop-leave-to {
    opacity: 0;
    transform: scale(0.4) translateX(-30px);
}

/* 番茄钟文本排版 */
.pomodoro-text-box {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    transform: translateX(-5px) !important;
}

.pomodoro-info {
    display: flex;
    flex-direction: row;
    align-items: center;
}

.pomodoro-time {
    font-size: 18px;
    font-weight: bold;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.5px;
    transform: translateY(-0.5px);
    transition: color 0.3s ease;
}

.pomodoro-time.phase-focus {
    color: #ff4757;
}

.pomodoro-time.phase-break {
    color: #2196f3;
}

.pomodoro-svg {
    width: 24px;
    height: 24px;
    transition: color 0.3s ease;
}

.pomodoro-svg.phase-focus {
    color: #ff4757;
}

.pomodoro-svg.phase-break {
    color: #2196f3;
}

/* ===== 倒计时样式 ===== */
.countdown-text-box {
    display: flex;
    align-items: center;
    gap: 8px;
}

.countdown-svg {
    width: 24px;
    height: 24px;
    color: #ff9800;
    transition: color 0.3s ease;
}

.countdown-info {
    display: flex;
    align-items: center;
    gap: 4px;
}

.countdown-time {
    font-size: 18px;
    font-weight: 800;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: #ff9800;
    letter-spacing: 1px;
}

.countdown-finished-text {
    font-size: 13px;
    font-weight: 700;
    color: #ff9800;
    animation: cd-blink 1s ease-in-out infinite;
}

@keyframes cd-blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
}

/* 倒计时右侧圆钮（仅橙色图标，无背景填充，与番茄钟风格一致） */
.cd-right-circle {
    /* 无背景填充，图标颜色由 .countdown-svg 的 #ff9800 提供 */
}

/* 倒计时展开态控制按钮组 */
.cd-expanded-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    transform: translateX(8px) !important;
}

.cd-pause-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: rgba(255, 152, 0, 0.15);
    color: #ff9800;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid rgba(255, 152, 0, 0.3);
}

.cd-pause-btn:hover {
    background: rgba(255, 152, 0, 0.25);
}

.cd-close-btn {
    color: #ff9800 !important;
}

.cd-close-btn:hover {
    background-color: rgba(255, 152, 0, 0.15) !important;
    color: #ff9800 !important;
}

/* 番茄钟剩余轮数徽章 */
.pomodoro-cycle-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 18px;
    padding: 0 5px;
    margin-left: 6px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 800;
    background: rgba(128, 128, 128, 0.2);
    color: var(--item-title-color, #888);
    font-variant-numeric: tabular-nums;
    transform: translateY(-0.5px);
}

/* 实时活动全岛关闭按钮 */
.island-close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    color: #888;
    cursor: pointer;
    transition: all 0.2s ease;
    border-radius: 50%;
    z-index: 10;
    transform: translateX(8px) !important;
}

.island-close-btn:hover {
    color: #ff4757;
    background-color: rgba(255, 71, 87, 0.15);
    transform: scale(1.15);
}

.island-close-btn svg {
    width: 20px;
    height: 20px;
}
</style>
