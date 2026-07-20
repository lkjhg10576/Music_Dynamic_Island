<template>
    <div class="gallery-layout-wrapper">
        <div class="gallery-header">
            <div class="title-wrapper">
                <h1 class="art-title">Live<span class="art-accent">Active</span><span class="art-dot">.</span></h1>
                <div class="title-sub-row">
                    <span class="title-badge">实时活动枢纽</span>
                    <span class="subtitle-divider">/</span>
                    <span class="subtitle-text">选择模块 · 激活体验</span>
                </div>
            </div>
        </div>

        <transition name="fade">
            <button v-show="canScrollLeft" class="scroll-btn scroll-btn-left" @click="scrollToLeft">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round">
                    <polyline points="15 18 9 12 15 6"></polyline>
                </svg>
            </button>
        </transition>

        <transition name="fade">
            <button v-show="canScrollRight" class="scroll-btn scroll-btn-right" @click="scrollToRight">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round">
                    <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
            </button>
        </transition>

        <div class="gallery-track" ref="trackRef" @scroll="checkScroll"
            :class="{ 'mask-left': canScrollLeft, 'mask-right': canScrollRight }">

            <div v-for="item in activities" :key="item.id" class="gallery-card"
                :class="{ 'is-active': activeId === item.id }" :style="{ '--accent-color': item.accent }"
                @click="handleCardClick(item.id)">

                <div class="card-hero">
                    <div class="hero-top-row">
                        <div class="pro-icon" v-html="item.icon"></div>
                        <div v-if="item.id === 'pomodoro'" class="pomodoro-controls" @click.stop>
                            <button class="pomo-btn reset-btn" :disabled="isPomoRunning"
                                @click="pomoStep = 0; pomoFocusM = 25; pomoFocusS = 0; pomoBreakM = 5; pomoBreakS = 0; pomoCycles = 4">重置</button>
                            <button v-if="!isPomoRunning" class="pomo-btn toggle-btn"
                                :class="{ 'disabled': pomoStep < 3 && !pomoConfigDone }"
                                :disabled="pomoStep < 3 && !pomoConfigDone"
                                @click="handlePomoStart">
                                {{ pomoConfigDone ? '重新开始' : '启动' }}
                            </button>
                            <template v-else>
                                <button class="pomo-btn" @click="handlePomoPauseResume">
                                    {{ isPomoPaused ? '继续' : '暂停' }}
                                </button>
                                <button class="pomo-btn" @click="handlePomoStop">停止</button>
                            </template>
                        </div>
                        <label v-else-if="item.id !== 'countdown' && item.id !== 'hardware' && item.id !== 'health'" class="custom-switch" @click.stop>
                            <input type="checkbox" v-model="item.enabled" :disabled="item.disable">
                            <span class="slider"></span>
                        </label>
                    </div>
                    <div class="hero-text">
                        <div class="hero-text-main">
                            <h3>{{ item.title }}</h3>
                            <p>{{ item.desc }}</p>
                        </div>
                        <div v-if="!item.disable" class="hero-priority-chip" @click.stop
                            :title="'数字越小越优先显示（当前: ' + item.priority + '）'">
                            <span class="priority-label">序</span>
                            <input type="number" min="1" v-model.number="item.priority"
                                @focus="focusedPriority = item.id" @blur="onPriorityBlur(item)"
                                class="priority-input" />
                        </div>
                        <transition name="fade">
                            <span v-if="focusedPriority === item.id" class="priority-hint">数字越小越优先</span>
                        </transition>
                    </div>
                </div>

                <div class="card-body" v-if="activeId === item.id">
                    <transition name="fade-up" appear>
                        <div class="body-content">
                            <template v-if="item.id === 'pomodoro'">
                                <!-- 运行中状态显示 -->
                                <div v-if="isPomoRunning" class="pomo-running-state">
                                    <div class="pomo-phase-label" :class="pomoRunningPhase">
                                        {{ pomoRunningPhase === 'focus' ? '⏰ 专注中' : '☕ 休息中' }}
                                    </div>
                                    <div class="pomo-running-time" :class="pomoRunningPhase">
                                        {{ pomoFormattedRemaining }}
                                    </div>
                                    <div class="pomo-running-meta">
                                        <span>第 {{ pomoTotalCycles - pomoRemainingCycles + 1 }} / {{ pomoTotalCycles }} 轮</span>
                                        <span v-if="isPomoPaused" class="pomo-paused-tag">已暂停</span>
                                    </div>
                                </div>

                                <!-- 三步设置向导（仅在不运行时显示） -->
                                <div v-else class="pomo-setup-wizard">
                                    <!-- 步骤指示器 -->
                                    <div class="pomo-step-indicator">
                                        <span v-for="s in 3" :key="s" class="step-dot"
                                            :class="{ active: pomoStep === s - 1, done: pomoStep > s - 1 }">
                                            <template v-if="pomoStep > s - 1">✓</template>
                                            <template v-else>{{ s }}</template>
                                        </span>
                                        <span class="step-dot" :class="{ active: pomoConfigDone, done: pomoConfigDone }">
                                            {{ pomoConfigDone ? '✓' : '▶' }}
                                        </span>
                                    </div>

                                    <!-- Step 1: 专注时间 -->
                                    <div v-if="pomoStep === 0" class="pomo-step-content">
                                        <div class="step-label">⏰ 单次专注时间</div>
                                        <div class="step-input-row">
                                            <div class="step-input-group">
                                                <input type="number" v-model.number="pomoFocusM" class="step-input"
                                                    min="0" max="999" />
                                                <span class="step-unit">分</span>
                                            </div>
                                            <div class="step-input-group">
                                                <input type="number" v-model.number="pomoFocusS" class="step-input"
                                                    min="0" max="59" />
                                                <span class="step-unit">秒</span>
                                            </div>
                                        </div>
                                        <div class="step-summary">专注 {{ pomoFocusM }} 分 {{ pomoFocusS }} 秒</div>
                                        <button class="step-next-btn" @click="pomoStep = 1"
                                            :disabled="pomoFocusM === 0 && pomoFocusS === 0">
                                            下一步 →
                                        </button>
                                    </div>

                                    <!-- Step 2: 休息时间 -->
                                    <div v-if="pomoStep === 1" class="pomo-step-content">
                                        <div class="step-label">☕ 单次休息时间</div>
                                        <div class="step-input-row">
                                            <div class="step-input-group">
                                                <input type="number" v-model.number="pomoBreakM" class="step-input"
                                                    min="0" max="999" />
                                                <span class="step-unit">分</span>
                                            </div>
                                            <div class="step-input-group">
                                                <input type="number" v-model.number="pomoBreakS" class="step-input"
                                                    min="0" max="59" />
                                                <span class="step-unit">秒</span>
                                            </div>
                                        </div>
                                        <div class="step-summary">休息 {{ pomoBreakM }} 分 {{ pomoBreakS }} 秒</div>
                                        <div class="step-nav-row">
                                            <button class="step-back-btn" @click="pomoStep = 0">← 上一步</button>
                                            <button class="step-next-btn" @click="pomoStep = 2"
                                                :disabled="pomoBreakM === 0 && pomoBreakS === 0">
                                                下一步 →
                                            </button>
                                        </div>
                                    </div>

                                    <!-- Step 3: 循环轮数 -->
                                    <div v-if="pomoStep === 2" class="pomo-step-content">
                                        <div class="step-label">🔄 循环轮数</div>
                                        <div class="step-input-row" style="justify-content: center;">
                                            <div class="step-input-group" style="width: 120px;">
                                                <input type="number" v-model.number="pomoCycles" class="step-input"
                                                    min="1" max="999" />
                                                <span class="step-unit">轮</span>
                                            </div>
                                        </div>
                                        <div class="step-summary">共 {{ pomoCycles }} 轮（专注 + 休息）</div>
                                        <div class="step-nav-row">
                                            <button class="step-back-btn" @click="pomoStep = 1">← 上一步</button>
                                            <button class="step-start-btn" @click="handlePomoStart"
                                                :disabled="pomoCycles < 1">
                                                🚀 开始番茄钟
                                            </button>
                                        </div>
                                    </div>

                                    <!-- 配置已完成的状态 -->
                                    <div v-if="pomoConfigDone && !isPomoRunning" class="pomo-ready-state">
                                        <div class="pomo-ready-summary">
                                            专注 {{ pomoFocusM }}:{{ String(pomoFocusS).padStart(2, '0') }} |
                                            休息 {{ pomoBreakM }}:{{ String(pomoBreakS).padStart(2, '0') }} |
                                            {{ pomoCycles }} 轮
                                        </div>
                                        <div class="pomo-ready-actions">
                                            <button class="step-back-btn" @click="pomoStep = 0">修改参数</button>
                                        </div>
                                    </div>
                                </div>

                                <div style="border-bottom: 1px dashed var(--control-border); margin: 6px 0;"></div>

                                <div class="pro-setting-item" style="padding-top: 6px;">
                                    <div class="pro-meta">
                                        <span class="pro-title">免打扰模式</span>
                                    </div>
                                    <label class="custom-switch mini"><input type="checkbox"><span
                                            class="slider"></span></label>
                                </div>
                            </template>

                            <template v-else-if="item.id === 'countdown'">
                                <!-- 倒计时结束状态 -->
                                <div v-if="cdFinished" class="cd-finish-state">
                                    <div class="cd-finish-text">⏰ 倒计时结束</div>
                                    <button class="cd-reset-btn" @click="handleCdStop">关闭</button>
                                </div>

                                <!-- 运行中状态 -->
                                <div v-else-if="cdRunning" class="cd-running-state">
                                    <div class="cd-running-time">{{ cdFormattedRemaining }}</div>
                                    <div class="cd-running-meta">
                                        <span v-if="cdPaused" class="cd-paused-tag">已暂停</span>
                                    </div>
                                    <div class="cd-controls">
                                        <button class="cd-btn" @click="handleCdPauseResume">
                                            {{ cdPaused ? '▶ 继续' : '⏸ 暂停' }}
                                        </button>
                                        <button class="cd-btn cd-stop-btn" @click="handleCdStop">⏹ 停止</button>
                                    </div>
                                </div>

                                <!-- 设置状态 -->
                                <div v-else class="cd-setup">
                                    <div class="cd-setup-label">⏱ 设定时间</div>
                                    <div class="cd-input-row">
                                        <div class="cd-input-group">
                                            <input type="number" v-model.number="cdMinutes" class="cd-input" min="0" max="999" />
                                            <span class="cd-unit">分</span>
                                        </div>
                                        <div class="cd-input-group">
                                            <input type="number" v-model.number="cdSeconds" class="cd-input" min="0" max="59" />
                                            <span class="cd-unit">秒</span>
                                        </div>
                                    </div>
                                    <button class="cd-start-btn" @click="handleCdStart"
                                        :disabled="cdMinutes === 0 && cdSeconds === 0">
                                        ▶ 开始倒计时
                                    </button>
                                </div>
                            </template>

                            <template v-else-if="item.id === 'hardware'">
                                <div class="hw-config-panel">
                                    <!-- 硬件监控总开关 -->
                                    <div class="hw-toggle-row">
                                        <div class="hw-toggle-label">
                                            <span class="hw-toggle-title">启用硬件监控</span>
                                            <span class="hw-toggle-desc">后端实时推送 CPU / 内存数据</span>
                                        </div>
                                        <label class="custom-switch mini">
                                            <input type="checkbox" v-model="hwEnabled" @change="toggleHwEnabled">
                                            <span class="slider"></span>
                                        </label>
                                    </div>

                                    <!-- 显示模式选择 -->
                                    <div class="hw-mode-section" v-if="hwEnabled">
                                        <div class="hw-section-label">显示模式</div>
                                        <div class="hw-mode-grid">
                                            <button class="hw-mode-btn" :class="{ active: hwMode === 'single' }"
                                                @click="setHwMode('single')">
                                                <span class="mode-icon">◎</span>
                                                <span class="mode-name">单圆环</span>
                                                <span class="mode-desc">固定显示选中指标</span>
                                            </button>
                                            <button class="hw-mode-btn" :class="{ active: hwMode === 'dual' }"
                                                @click="setHwMode('dual')">
                                                <span class="mode-icon">◉</span>
                                                <span class="mode-name">双圆环</span>
                                                <span class="mode-desc">CPU 外圈 + 内存内圈</span>
                                            </button>
                                            <button class="hw-mode-btn" :class="{ active: hwMode === 'rotation' }"
                                                @click="setHwMode('rotation')">
                                                <span class="mode-icon">↻</span>
                                                <span class="mode-name">轮换</span>
                                                <span class="mode-desc">每 5 秒切换 CPU / 内存</span>
                                            </button>
                                        </div>
                                    </div>

                                    <!-- 默认指标选择（仅单圆环模式显示） -->
                                    <div class="hw-metric-section" v-if="hwEnabled && hwMode === 'single'">
                                        <div class="hw-section-label">默认指标</div>
                                        <div class="hw-metric-row">
                                            <label class="hw-radio-label">
                                                <input type="radio" value="cpu" v-model="hwDefaultMetric" @change="saveHwConfig(); syncHwToWidget()">
                                                <span class="radio-text">CPU</span>
                                            </label>
                                            <label class="hw-radio-label">
                                                <input type="radio" value="mem" v-model="hwDefaultMetric" @change="saveHwConfig(); syncHwToWidget()">
                                                <span class="radio-text">内存</span>
                                            </label>
                                        </div>
                                    </div>

                                    <!-- 实时预览 -->
                                    <div class="hw-live-preview" v-if="hwEnabled">
                                        <div class="hw-section-label">实时预览</div>
                                        <div class="hw-ring-preview">
                                            <svg viewBox="0 0 60 60" class="hw-preview-svg">
                                                <circle cx="30" cy="30" r="24" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="5" />
                                                <circle cx="30" cy="30" r="24" fill="none" :stroke="hwCpuPct >= 80 ? '#a855f7' : '#ffffff'" stroke-width="5"
                                                    :stroke-dasharray="`${(hwCpuPct / 100) * 150.8} 150.8`"
                                                    stroke-linecap="round" transform="rotate(-90 30 30)"
                                                    style="transition: stroke-dasharray 0.5s ease;" />
                                            </svg>
                                            <span class="hw-preview-label">CPU {{ Math.round(hwCpuPct) }}%</span>
                                        </div>
                                        <div class="hw-ring-preview">
                                            <svg viewBox="0 0 60 60" class="hw-preview-svg">
                                                <circle cx="30" cy="30" r="24" fill="none" stroke="rgba(255,255,255,0.1)" stroke-width="5" />
                                                <circle cx="30" cy="30" r="24" fill="none" :stroke="hwMemPct >= 80 ? '#ff4757' : '#3b82f6'" stroke-width="5"
                                                    :stroke-dasharray="`${(hwMemPct / 100) * 150.8} 150.8`"
                                                    stroke-linecap="round" transform="rotate(-90 30 30)"
                                                    style="transition: stroke-dasharray 0.5s ease;" />
                                            </svg>
                                            <span class="hw-preview-label">RAM {{ Math.round(hwMemPct) }}%</span>
                                        </div>
                                    </div>
                                </div>
                            </template>

                            <template v-else-if="item.id === 'health'">
                                <div class="health-config-panel">
                                    <!-- 久坐提醒 -->
                                    <div class="health-reminder-row">
                                        <div class="health-reminder-header">
                                            <span class="health-reminder-icon">🪑</span>
                                            <div class="health-reminder-info">
                                                <span class="health-reminder-title">久坐提醒</span>
                                                <span class="health-reminder-desc">定时提醒起身活动</span>
                                            </div>
                                        </div>
                                        <div class="health-reminder-controls">
                                            <div class="health-time-input-group">
                                                <input type="number" v-model.number="srMinutes" class="health-time-input" min="1" max="999" @change="onSrTimeChange" />
                                                <span class="health-time-unit">分钟</span>
                                            </div>
                                            <label class="custom-switch mini">
                                                <input type="checkbox" v-model="srEnabled" @change="toggleSrEnabled">
                                                <span class="slider"></span>
                                            </label>
                                        </div>
                                    </div>
                                    <div class="health-status-row" v-if="srEnabled">
                                        <span class="health-status-text" :class="{ alerting: srAlerting }">
                                            {{ srAlerting ? '⏰ 提醒中' : '✓ 已开启' }}
                                        </span>
                                        <template v-if="!srAlerting && srRemainingSeconds > 0">
                                            <span class="health-countdown-text">距离下次提醒 {{ srFormattedRemaining }}</span>
                                            <button class="health-skip-btn" :class="{ 'is-disabled': !srCanSkip }" :disabled="!srCanSkip" @click.stop="skipSittingReminder">
                                                跳过提醒
                                            </button>
                                        </template>
                                    </div>

                                    <div class="health-divider"></div>

                                    <!-- 喝水提醒 -->
                                    <div class="health-reminder-row">
                                        <div class="health-reminder-header">
                                            <span class="health-reminder-icon">💧</span>
                                            <div class="health-reminder-info">
                                                <span class="health-reminder-title">喝水提醒</span>
                                                <span class="health-reminder-desc">定时提醒补充水分</span>
                                            </div>
                                        </div>
                                        <div class="health-reminder-controls">
                                            <div class="health-time-input-group">
                                                <input type="number" v-model.number="wrMinutes" class="health-time-input" min="1" max="999" @change="onWrTimeChange" />
                                                <span class="health-time-unit">分钟</span>
                                            </div>
                                            <label class="custom-switch mini">
                                                <input type="checkbox" v-model="wrEnabled" @change="toggleWrEnabled">
                                                <span class="slider"></span>
                                            </label>
                                        </div>
                                    </div>
                                    <div class="health-status-row" v-if="wrEnabled">
                                        <span class="health-status-text" :class="{ alerting: wrAlerting }">
                                            {{ wrAlerting ? '⏰ 提醒中' : '✓ 已开启' }}
                                        </span>
                                        <template v-if="!wrAlerting && wrRemainingSeconds > 0">
                                            <span class="health-countdown-text">距离下次提醒 {{ wrFormattedRemaining }}</span>
                                            <button class="health-skip-btn" :class="{ 'is-disabled': !wrCanSkip }" :disabled="!wrCanSkip" @click.stop="skipWaterReminder">
                                                跳过提醒
                                            </button>
                                        </template>
                                    </div>
                                </div>
                            </template>

                            <template v-else>
                                <div class="pro-coming-soon">
                                    <div class="loader-line"></div>
                                    <p>模块部署中 SYSTEM_PENDING</p>
                                </div>
                            </template>
                        </div>
                    </transition>
                </div>
            </div>

            <div class="spacer"></div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from 'vue';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
    NSD_POMODORO_FOCUS_SECS,
    NSD_POMODORO_BREAK_SECS,
    NSD_POMODORO_CYCLES,
    NSD_COUNTDOWN_SECS,
    NSD_HW_ENABLED,
    NSD_HW_MODE,
    NSD_HW_DEFAULT_METRIC,
    NSD_HW_ROTATION,
    NSD_HW_DUAL_RING,
    NSD_SITTING_REMINDER_ENABLED,
    NSD_SITTING_REMINDER_SECS,
    NSD_WATER_REMINDER_ENABLED,
    NSD_WATER_REMINDER_SECS,
    NSD_ACTIVITY_PRIORITY,
} from '../constants/storageKeys';

// ===== 三步设置状态 =====
const pomoStep = ref(0); // 0=专注时间, 1=休息时间, 2=循环轮数
const pomoFocusM = ref(Number(localStorage.getItem(NSD_POMODORO_FOCUS_SECS + '_m')) || 25);
const pomoFocusS = ref(Number(localStorage.getItem(NSD_POMODORO_FOCUS_SECS + '_s')) || 0);
const pomoBreakM = ref(Number(localStorage.getItem(NSD_POMODORO_BREAK_SECS + '_m')) || 5);
const pomoBreakS = ref(Number(localStorage.getItem(NSD_POMODORO_BREAK_SECS + '_s')) || 0);
const pomoCycles = ref(Number(localStorage.getItem(NSD_POMODORO_CYCLES)) || 4);
const pomoConfigDone = ref(false);

// ===== 运行中状态（由后端事件驱动） =====
const isPomoRunning = ref(false);
const isPomoPaused = ref(false);
const pomoRunningPhase = ref<'focus' | 'break'>('focus');
const pomoRemainingSecs = ref(0);
const pomoRemainingCycles = ref(4);
const pomoTotalCycles = ref(4);

const pomoFormattedRemaining = computed(() => {
    const t = pomoRemainingSecs.value;
    const m = Math.floor(t / 60).toString().padStart(2, '0');
    const s = (t % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
});

// ===== 倒计时状态 =====
const cdRunning = ref(false);
const cdPaused = ref(false);
const cdRemainingSecs = ref(0);
const cdTotalSecs = ref(0);
const cdFinished = ref(false);
const cdMinutes = ref(Number(localStorage.getItem(NSD_COUNTDOWN_SECS) || '5'));
const cdSeconds = ref(0);

const cdFormattedRemaining = computed(() => {
    const t = cdRemainingSecs.value;
    const m = Math.floor(t / 60).toString().padStart(2, '0');
    const s = (t % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
});

// ===== 硬件监控配置 =====
const hwEnabled = ref(localStorage.getItem(NSD_HW_ENABLED) === 'true');
const hwMode = ref(localStorage.getItem(NSD_HW_MODE) || 'single');
const hwDefaultMetric = ref(localStorage.getItem(NSD_HW_DEFAULT_METRIC) || 'cpu');
const hwCpuPct = ref(0);
const hwMemPct = ref(0);

// ===== 健康提醒配置 =====
const srEnabled = ref(localStorage.getItem(NSD_SITTING_REMINDER_ENABLED) === 'true');
const srMinutes = ref(Number(localStorage.getItem(NSD_SITTING_REMINDER_SECS) || '60'));
const srActive = ref(false);
const srAlerting = ref(false);
const srRemainingSeconds = ref(0);
const srCanSkip = ref(true);
const wrEnabled = ref(localStorage.getItem(NSD_WATER_REMINDER_ENABLED) === 'true');
const wrMinutes = ref(Number(localStorage.getItem(NSD_WATER_REMINDER_SECS) || '120'));
const wrActive = ref(false);
const wrAlerting = ref(false);
const wrRemainingSeconds = ref(0);
const wrCanSkip = ref(true);

const srFormattedRemaining = computed(() => formatRemaining(srRemainingSeconds.value));
const wrFormattedRemaining = computed(() => formatRemaining(wrRemainingSeconds.value));

function formatRemaining(secs: number): string {
    if (secs <= 0) return '';
    if (secs >= 60) {
        const m = Math.floor(secs / 60);
        const s = secs % 60;
        return `${m}分${s}秒`;
    }
    return `${secs}秒`;
}

async function skipSittingReminder() {
    await invoke('skip_sitting_reminder').catch(() => {});
    srCanSkip.value = false;
}

async function skipWaterReminder() {
    await invoke('skip_water_reminder').catch(() => {});
    wrCanSkip.value = false;
}

function saveHealthConfig() {
    localStorage.setItem(NSD_SITTING_REMINDER_ENABLED, String(srEnabled.value));
    localStorage.setItem(NSD_SITTING_REMINDER_SECS, srMinutes.value.toString());
    localStorage.setItem(NSD_WATER_REMINDER_ENABLED, String(wrEnabled.value));
    localStorage.setItem(NSD_WATER_REMINDER_SECS, wrMinutes.value.toString());
}

async function toggleSrEnabled() {
    saveHealthConfig();
    if (srEnabled.value) {
        try {
            await invoke('start_sitting_reminder', { intervalSecs: srMinutes.value * 60 });
            srActive.value = true;
            srAlerting.value = false;
        } catch (_e) {}
    } else {
        try {
            await invoke('stop_sitting_reminder');
            srActive.value = false;
            srAlerting.value = false;
        } catch (_e) {}
    }
}

async function toggleWrEnabled() {
    saveHealthConfig();
    if (wrEnabled.value) {
        try {
            await invoke('start_water_reminder', { intervalSecs: wrMinutes.value * 60 });
            wrActive.value = true;
            wrAlerting.value = false;
        } catch (_e) {}
    } else {
        try {
            await invoke('stop_water_reminder');
            wrActive.value = false;
            wrAlerting.value = false;
        } catch (_e) {}
    }
}

function saveHwConfig() {
    localStorage.setItem(NSD_HW_ENABLED, String(hwEnabled.value));
    localStorage.setItem(NSD_HW_MODE, hwMode.value);
    localStorage.setItem(NSD_HW_DEFAULT_METRIC, hwDefaultMetric.value);
    localStorage.setItem(NSD_HW_ROTATION, String(hwMode.value === 'rotation'));
    localStorage.setItem(NSD_HW_DUAL_RING, String(hwMode.value === 'dual'));
}

async function toggleHwEnabled() {
    saveHwConfig();
    try {
        await invoke('set_hardware_emit', { enabled: hwEnabled.value });
    } catch (_e) {
        // command 可能不存在，忽略
    }
    // 跨窗口通知灵动岛：同步硬件监控附属图标的显示状态
    syncHwToWidget();
}

function setHwMode(mode: string) {
    hwMode.value = mode;
    saveHwConfig();
    // 通知灵动岛更新模式和指标
    syncHwToWidget();
}

function syncHwToWidget() {
    try {
        // 直接把完整配置随事件携带，避免灵动岛窗口再回头读 localStorage 的时序竞态
        emit('control-hardware-mon', {
            enabled: hwEnabled.value,
            mode: hwMode.value,
            defaultMetric: hwDefaultMetric.value,
        });
    } catch (_e) {
        // 忽略
    }
}

function saveCdConfig() {
    localStorage.setItem(NSD_COUNTDOWN_SECS, (cdMinutes.value * 60 + cdSeconds.value).toString());
}

async function handleCdStart() {
    const totalSecs = cdMinutes.value * 60 + cdSeconds.value;
    if (totalSecs < 1) return;
    saveCdConfig();
    await invoke('start_countdown', { totalSecs });
    cdTotalSecs.value = totalSecs;
    cdRemainingSecs.value = totalSecs;
    cdRunning.value = true;
    cdPaused.value = false;
    cdFinished.value = false;
}

async function handleCdPauseResume() {
    if (cdPaused.value) {
        await invoke('resume_countdown');
        cdPaused.value = false;
    } else {
        await invoke('pause_countdown');
        cdPaused.value = true;
    }
}

async function handleCdStop() {
    await invoke('stop_countdown');
    cdRunning.value = false;
    cdPaused.value = false;
    cdFinished.value = false;
}

// ===== 持久化三步配置 =====
function savePomoConfig() {
    localStorage.setItem(NSD_POMODORO_FOCUS_SECS + '_m', pomoFocusM.value.toString());
    localStorage.setItem(NSD_POMODORO_FOCUS_SECS + '_s', pomoFocusS.value.toString());
    localStorage.setItem(NSD_POMODORO_BREAK_SECS + '_m', pomoBreakM.value.toString());
    localStorage.setItem(NSD_POMODORO_BREAK_SECS + '_s', pomoBreakS.value.toString());
    localStorage.setItem(NSD_POMODORO_CYCLES, pomoCycles.value.toString());
}

function getFocusSecs(): number {
    return pomoFocusM.value * 60 + pomoFocusS.value;
}

function getBreakSecs(): number {
    return pomoBreakM.value * 60 + pomoBreakS.value;
}

// ===== 操作处理 =====
async function handlePomoStart() {
    const focusSecs = getFocusSecs();
    const breakSecs = getBreakSecs();
    if (focusSecs < 1 || breakSecs < 1 || pomoCycles.value < 1) return;
    savePomoConfig();
    pomoConfigDone.value = true;
    await invoke('start_pomodoro', {
        focusSecs,
        breakSecs,
        cycles: pomoCycles.value,
    });
    // 立即更新本地状态，等待后端 tick 来确认
    isPomoRunning.value = true;
    isPomoPaused.value = false;
    pomoRunningPhase.value = 'focus';
    pomoRemainingSecs.value = focusSecs;
    pomoRemainingCycles.value = pomoCycles.value;
    pomoTotalCycles.value = pomoCycles.value;
}

async function handlePomoPauseResume() {
    if (isPomoPaused.value) {
        await invoke('resume_pomodoro');
        isPomoPaused.value = false;
    } else {
        await invoke('pause_pomodoro');
        isPomoPaused.value = true;
    }
}

async function handlePomoStop() {
    await invoke('stop_pomodoro');
    isPomoRunning.value = false;
    isPomoPaused.value = false;
}

function onSrTimeChange() {
    if (srMinutes.value < 1) srMinutes.value = 1;
    saveHealthConfig();
    if (srEnabled.value) {
        invoke('start_sitting_reminder', { intervalSecs: srMinutes.value * 60 }).catch(() => {});
    }
}

function onWrTimeChange() {
    if (wrMinutes.value < 1) wrMinutes.value = 1;
    saveHealthConfig();
    if (wrEnabled.value) {
        invoke('start_water_reminder', { intervalSecs: wrMinutes.value * 60 }).catch(() => {});
    }
}

const activities = ref([
    {
        id: 'pomodoro',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>',
        title: '专注番茄钟',
        desc: '沉浸工作时间管理',
        accent: '#ff4757',
        enabled: false,
        disable: false,
        priority: 1,
    },
    {
        id: 'countdown',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>',
        title: '快捷倒计时',
        desc: '自定义时长倒计时',
        accent: '#ff9800',
        enabled: false,
        disable: false,
        priority: 2,
    },
    {
        id: 'hardware',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2" /><rect x="9" y="9" width="6" height="6" /><line x1="9" y1="1" x2="9" y2="4" /><line x1="15" y1="1" x2="15" y2="4" /><line x1="9" y1="20" x2="9" y2="23" /><line x1="15" y1="20" x2="15" y2="23" /><line x1="20" y1="9" x2="23" y2="9" /><line x1="20" y1="14" x2="23" y2="14" /><line x1="1" y1="9" x2="4" y2="9" /><line x1="1" y1="14" x2="4" y2="14" /></svg>',
        title: '硬件监控',
        desc: '实时监测处理器与内存',
        accent: '#3b82f6',
        enabled: false,
        disable: false,
        priority: 3,
    },
    {
        id: 'health',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"></path></svg>',
        title: '健康提醒',
        desc: '久坐与喝水提醒',
        accent: '#10b981',
        enabled: false,
        disable: false,
        priority: 4,
    },
    {
        id: 'sysmsg',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>',
        title: '系统动态感知',
        desc: '实时捕捉软硬件生态变化',
        accent: '#ff4757',
        enabled: false,
        disable: true,
        priority: 99,
    },
    {
        id: 'printer',
        icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>',
        title: '打印机队列',
        desc: '批量打印进度状态',
        accent: '#ff4757',
        enabled: false,
        disable: true,
        priority: 99,
    },
]);

// ===== 实时活动优先级同步 =====
// RT_IDS: 参与"多活动并行轮换"的四种实时活动 id（顺序固定，作为 priority 平局时的稳定排序键）
const RT_IDS = ['pomodoro', 'countdown', 'hardware', 'health'] as const;

// 当前正在编辑优先级的活动 id（用于显示提示文字）
const focusedPriority = ref<string | null>(null);

// 读取 NSD_ACTIVITY_PRIORITY 持久化的优先级 map
function loadPriorityMap(): Record<string, number> {
    try {
        const raw = localStorage.getItem(NSD_ACTIVITY_PRIORITY);
        if (raw) {
            const parsed = JSON.parse(raw);
            if (parsed && typeof parsed === 'object') {
                return parsed as Record<string, number>;
            }
        }
    } catch (_e) {}
    return {};
}

// 将 activities 中 RT_IDS 的 priority 持久化回 NSD_ACTIVITY_PRIORITY
function persistPriorityMap() {
    const map: Record<string, number> = {};
    for (const id of RT_IDS) {
        const item = activities.value.find(a => a.id === id);
        if (item) map[id] = item.priority;
    }
    localStorage.setItem(NSD_ACTIVITY_PRIORITY, JSON.stringify(map));
}

// 动态构建 { id: { enabled, priority } } 配置 map
// enabled 取自各活动的运行/启用状态（而非 activities[].enabled，因为 pomodoro/countdown/health 没有通用开关）
function buildActivityConfig(): Record<string, { enabled: boolean; priority: number }> {
    const map: Record<string, { enabled: boolean; priority: number }> = {};
    const enabledMap: Record<string, boolean> = {
        pomodoro: isPomoRunning.value,
        countdown: cdRunning.value,
        hardware: hwEnabled.value,
        health: srEnabled.value || wrEnabled.value,
    };
    for (const id of RT_IDS) {
        const item = activities.value.find(a => a.id === id);
        if (item) {
            map[id] = { enabled: enabledMap[id], priority: item.priority };
        }
    }
    return map;
}

// 推送配置到灵动岛窗口
function syncActivityConfig() {
    try {
        emit('control-activity-config', buildActivityConfig());
    } catch (_e) {
        // 忽略
    }
}

// 优先级输入框失焦：持久化 + 推送
function onPriorityBlur(_item: { id: string }) {
    focusedPriority.value = null;
    persistPriorityMap();
    syncActivityConfig();
}


const activeId = ref('pomodoro');
const trackRef = ref<HTMLElement | null>(null);

const canScrollLeft = ref(false);
const canScrollRight = ref(true);

const checkScroll = () => {
    if (!trackRef.value) return;
    const { scrollLeft, clientWidth } = trackRef.value;

    canScrollLeft.value = scrollLeft > 1;

    const cards = trackRef.value.querySelectorAll('.gallery-card');
    if (cards.length > 0) {
        const lastCard = cards[cards.length - 1] as HTMLElement;
        const lastCardRightEdge = lastCard.offsetLeft + lastCard.offsetWidth + 24;
        canScrollRight.value = Math.ceil(scrollLeft + clientWidth) < lastCardRightEdge;
    } else {
        canScrollRight.value = false;
    }
};

const scrollToLeft = () => {
    const idx = activities.value.findIndex(item => item.id === activeId.value);
    if (idx > 0) {
        activateAndCenter(activities.value[idx - 1].id);
    }
};

const scrollToRight = () => {
    const idx = activities.value.findIndex(item => item.id === activeId.value);
    const last = activities.value.length - 1;
    if (idx >= 0 && idx < last) {
        activateAndCenter(activities.value[idx + 1].id);
    }
};

const activateAndCenter = (id: string) => {
    activeId.value = id;

    const container = trackRef.value;
    if (!container) return;

    const index = activities.value.findIndex(item => item.id === id);
    if (index === -1) return;

    const finalOffsetLeft = 4 + (index * 220);
    const finalActiveWidth = 320;

    let targetScrollLeft = finalOffsetLeft - (container.clientWidth / 2) + (finalActiveWidth / 2);
    targetScrollLeft = Math.max(0, targetScrollLeft);

    container.scrollTo({ left: targetScrollLeft, behavior: 'smooth' });
};

const handleCardClick = (id: string) => {
    activateAndCenter(id);
};

onMounted(async () => {
    // 启动恢复：从 NSD_ACTIVITY_PRIORITY 回填各活动 priority
    const savedPriority = loadPriorityMap();
    for (const id of RT_IDS) {
        const item = activities.value.find(a => a.id === id);
        if (item && typeof savedPriority[id] === 'number') {
            item.priority = savedPriority[id];
        }
    }
    // 监听后端番茄钟 tick 事件
    await listen<any>('pomodoro-tick', (event) => {
        const payload = event.payload;
        if (payload.active === false) {
            isPomoRunning.value = false;
            isPomoPaused.value = false;
            return;
        }
        isPomoRunning.value = true;
        isPomoPaused.value = payload.paused || false;
        pomoRunningPhase.value = payload.phase;
        pomoRemainingSecs.value = payload.remaining_secs;
        pomoRemainingCycles.value = payload.remaining_cycles;
        pomoTotalCycles.value = payload.total_cycles || pomoTotalCycles.value;
    });

    // 监听番茄钟完成事件
    await listen<any>('pomodoro-complete', () => {
        isPomoRunning.value = false;
        isPomoPaused.value = false;
    });

    // 尝试恢复运行中状态
    try {
        const state: any = await invoke('get_pomodoro_state');
        if (state.active) {
            isPomoRunning.value = true;
            isPomoPaused.value = state.paused || false;
            pomoRunningPhase.value = state.phase;
            pomoRemainingSecs.value = state.remaining_secs;
            pomoRemainingCycles.value = state.remaining_cycles;
            pomoTotalCycles.value = state.total_cycles;
            pomoConfigDone.value = true;
        }
    } catch (_e) {
        // 后端 pomodoro 模块不存在或未初始化，忽略
    }

    // 监听倒计时 tick 事件
    await listen<any>('countdown-tick', (event) => {
        const p = event.payload;
        if (p.active === false) {
            cdRunning.value = false;
            cdPaused.value = false;
            cdFinished.value = false;
            return;
        }
        cdRunning.value = true;
        cdPaused.value = p.paused || false;
        cdRemainingSecs.value = p.remaining_secs;
        cdTotalSecs.value = p.total_secs || cdTotalSecs.value;
        cdFinished.value = p.phase === 'finished';
    });

    // 监听倒计时完成事件
    await listen<any>('countdown-complete', () => {
        cdRunning.value = false;
        cdFinished.value = true;
    });

    // 尝试恢复倒计时运行状态
    try {
        const state: any = await invoke('get_countdown_state');
        if (state.active) {
            cdRunning.value = true;
            cdPaused.value = state.paused || false;
            cdRemainingSecs.value = state.remaining_secs;
            cdTotalSecs.value = state.total_secs;
            cdFinished.value = state.phase === 'finished';
        }
    } catch (_e) {}

    // 监听后端硬件监控推送事件
    await listen<any>('monitor-stats', (event) => {
        const p = event.payload;
        if (typeof p.cpu_pct === 'number') hwCpuPct.value = p.cpu_pct;
        if (typeof p.mem_pct === 'number') hwMemPct.value = p.mem_pct;
    });

    // 监听健康提醒 tick 事件
    await listen<any>('health-reminder-tick', (event) => {
        const p = event.payload;
        if (p.sitting) {
            srActive.value = p.sitting.enabled;
            srAlerting.value = p.sitting.alerting;
            srRemainingSeconds.value = p.sitting.remaining_secs;
            srCanSkip.value = p.sitting.can_skip;
        }
        if (p.water) {
            wrActive.value = p.water.enabled;
            wrAlerting.value = p.water.alerting;
            wrRemainingSeconds.value = p.water.remaining_secs;
            wrCanSkip.value = p.water.can_skip;
        }
    });

    // 尝试恢复健康提醒运行状态
    try {
        const state: any = await invoke('get_health_reminder_state');
        if (state.sitting.enabled) {
            srActive.value = true;
            srAlerting.value = state.sitting.alerting;
            srRemainingSeconds.value = state.sitting.remaining_secs;
            srCanSkip.value = state.sitting.can_skip;
        }
        if (state.water.enabled) {
            wrActive.value = true;
            wrAlerting.value = state.water.alerting;
            wrRemainingSeconds.value = state.water.remaining_secs;
            wrCanSkip.value = state.water.can_skip;
        }
    } catch (_e) {}

    // 启动时自动开启已启用的健康提醒
    if (srEnabled.value && !srActive.value) {
        invoke('start_sitting_reminder', { intervalSecs: srMinutes.value * 60 }).catch(() => {});
        srActive.value = true;
    }
    if (wrEnabled.value && !wrActive.value) {
        invoke('start_water_reminder', { intervalSecs: wrMinutes.value * 60 }).catch(() => {});
        wrActive.value = true;
    }

    // 硬件监控后台始终推送 monitor-stats，前端不再控制 emit 开关

    // 启动后向灵动岛推送一次活动配置（双保险：与 WidgetIsland 自身 onMounted 读 localStorage 互补）
    syncActivityConfig();

    // 监听各活动 enabled 状态变化，自动同步配置到灵动岛
    watch([isPomoRunning, cdRunning, hwEnabled, srEnabled, wrEnabled], () => {
        syncActivityConfig();
    });

    nextTick(() => { checkScroll(); });
    window.addEventListener('resize', checkScroll);
});

onUnmounted(() => {
    window.removeEventListener('resize', checkScroll);
});
</script>

<style scoped>
/* 框架布局*/
.gallery-layout-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
}

.gallery-header {
    flex-shrink: 0;
    margin-bottom: 24px;
}

/* 标题区域 — 艺术字排版 */
.title-wrapper {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-left: 4px;
}

.art-title {
    font-size: 60px;
    font-weight: 900;
    letter-spacing: -3px;
    line-height: 1;
    margin: 0;
    color: var(--h1-color, #1a1a1a);
    display: flex;
    align-items: baseline;
    user-select: none;
}

.art-accent {
    -webkit-text-fill-color: transparent;
    -webkit-text-stroke: 1.5px var(--h1-color, #1a1a1a);
    background: none;
}

.art-dot {
    color: #ef4444;
    font-size: 64px;
    line-height: 0.8;
    margin-left: 1px;
}

.title-sub-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 2px;
    flex-wrap: wrap;
}

.title-badge {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 1.5px;
    color: var(--h1-color, #333);
    background: var(--control-bg, rgba(0, 0, 0, 0.04));
    padding: 4px 14px;
    border-radius: 100px;
    border: 1px solid var(--control-border, rgba(0, 0, 0, 0.06));
}

.subtitle-divider {
    font-size: 11px;
    color: var(--control-border, #ddd);
    font-weight: 300;
}

.subtitle-text {
    font-size: 12px;
    font-weight: 600;
    color: var(--subtitle-color, #888);
    letter-spacing: 0.3px;
}

/* 横向控制按钮 */
.scroll-btn {
    position: absolute;
    top: 280px;
    transform: translateY(-50%);
    z-index: 10;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--card-bg, #ffffff);
    border: 1px solid var(--control-border);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--item-title-color);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.scroll-btn svg {
    width: 20px;
    height: 20px;
}

.scroll-btn:hover {
    transform: translateY(-50%) scale(1.1);
    border-color: var(--accent-color, #ccc);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
}

.scroll-btn-left {
    left: 4px;
}

.scroll-btn-right {
    right: 4px;
}

/* 横向容器 */
.gallery-track {
    display: flex;
    gap: 20px;
    overflow-x: auto;
    padding: 10px 50vw 30px 4px;
    align-items: stretch;
    flex-grow: 1;
    min-height: 0;
    scroll-behavior: smooth;
    scrollbar-width: none;
    -ms-overflow-style: none;
}

.gallery-track::-webkit-scrollbar {
    display: none;
}

@property --mask-left-size {
    syntax: '<length>';
    initial-value: 0px;
    inherits: false;
}

@property --mask-right-size {
    syntax: '<length>';
    initial-value: 0px;
    inherits: false;
}

.gallery-track {
    --mask-left-size: 0px;
    --mask-right-size: 0px;

    -webkit-mask-image: linear-gradient(to right,
            transparent 0%,
            #000 var(--mask-left-size),
            #000 calc(100% - var(--mask-right-size)),
            transparent 100%);
    mask-image: linear-gradient(to right,
            transparent 0%,
            #000 var(--mask-left-size),
            #000 calc(100% - var(--mask-right-size)),
            transparent 100%);

    transition: --mask-left-size 0.4s cubic-bezier(0.16, 1, 0.3, 1),
        --mask-right-size 0.4s cubic-bezier(0.16, 1, 0.3, 1);
}

.gallery-track.mask-left {
    --mask-left-size: 60px;
}

.gallery-track.mask-right {
    --mask-right-size: 60px;
}

.spacer {
    width: 50vw;
    flex-shrink: 0;
}

/* 卡片 */
.gallery-card {
    flex-shrink: 0;
    width: 200px;
    background: var(--card-bg);
    border: 1px solid var(--control-border);
    border-radius: 20px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    transform: scale(0.96);
    opacity: 0.45;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.02);
    transition: width 0.5s cubic-bezier(0.16, 1, 0.3, 1),
        transform 0.5s cubic-bezier(0.16, 1, 0.3, 1),
        opacity 0.3s ease,
        box-shadow 0.5s cubic-bezier(0.16, 1, 0.3, 1),
        border-color 0.5s cubic-bezier(0.16, 1, 0.3, 1);
}

.gallery-card:hover {
    opacity: 0.85;
}

.gallery-card.is-active {
    width: 320px;
    transform: scale(1);
    opacity: 1;
    cursor: default;
    border-color: var(--accent-color);
    box-shadow: 0 20px 40px -10px rgba(0, 0, 0, 0.1),
        0 0 0 1px var(--accent-color) inset;
}

:global(.dark-theme) .gallery-card.is-active {
    box-shadow: 0 20px 40px -10px rgba(0, 0, 0, 0.5),
        0 0 20px -5px var(--accent-color);
    background: linear-gradient(180deg, var(--control-bg) 0%, var(--card-bg) 100%);
}

.card-hero {
    padding: 24px;
    display: flex;
    flex-direction: column;
    height: 140px;
    flex-shrink: 0;
    position: relative;
}

.hero-top-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: auto;
}

.pro-icon {
    width: 36px;
    height: 36px;
    color: var(--item-title-color);
    transition: all 0.4s ease;
}

.is-active .pro-icon {
    color: var(--accent-color);
    transform: scale(1.1);
}

.hero-text {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    position: relative;
}

.hero-text-main {
    flex: 1;
    min-width: 0;
}

.hero-text h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 800;
    color: var(--h1-color);
    letter-spacing: -0.5px;
    transition: color 0.3s;
}

.hero-text p {
    margin: 6px 0 0 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--subtitle-color);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* 优先级输入 chip */
.hero-priority-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 6px;
    border-radius: 100px;
    background: var(--control-bg, rgba(0, 0, 0, 0.04));
    border: 1px solid var(--control-border, rgba(0, 0, 0, 0.06));
    flex-shrink: 0;
    transition: all 0.2s ease;
}

.hero-priority-chip:hover {
    border-color: var(--accent-color, #ccc);
}

.priority-label {
    font-size: 10px;
    font-weight: 700;
    color: var(--item-desc-color, #888);
    letter-spacing: 0.5px;
}

.priority-input {
    width: 28px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--h1-color, #1a1a1a);
    font-size: 13px;
    font-weight: 800;
    text-align: center;
    outline: none;
    -moz-appearance: textfield;
    appearance: textfield;
}

.priority-input::-webkit-outer-spin-button,
.priority-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
}

.priority-hint {
    position: absolute;
    bottom: 100%;
    right: 0;
    margin-bottom: 4px;
    padding: 0;
    background: transparent;
    color: var(--h1-color);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    pointer-events: none;
    z-index: 10;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
}

.card-body {
    padding: 0 24px 24px 24px;
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
}

.body-content {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
}

.fade-up-enter-active {
    transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1);
    transition-delay: 0.1s;
}

.fade-up-enter-from {
    opacity: 0;
    transform: translateY(15px);
}

/* Switch */
.custom-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
}

.custom-switch.mini {
    width: 36px;
    height: 20px;
}

.custom-switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.custom-switch .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--control-border, #e2e8f0);
    border-radius: 24px;
    transition: background-color 0.3s ease;
}

.custom-switch .slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: #ffffff;
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.custom-switch.mini .slider:before {
    height: 14px;
    width: 14px;
    left: 3px;
    bottom: 3px;
}

.custom-switch input:checked+.slider {
    background-color: var(--accent-color, #10b981);
}

.custom-switch input:checked+.slider:before {
    transform: translateX(20px);
}

.custom-switch.mini input:checked+.slider:before {
    transform: translateX(16px);
}

.custom-switch input:disabled+.slider {
    cursor: not-allowed;
    opacity: 0.6;
}

.custom-switch input:disabled {
    cursor: not-allowed;
}

/* 排版辅助 */
.pro-setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    padding: 14px 0;
    border-bottom: 1px dashed var(--control-border);
}

.pro-setting-item:last-child {
    border-bottom: none;
}

.pro-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex-shrink: 0;
    max-width: 100%;
}

.pro-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--item-title-color);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.mt-10 {
    margin-top: 10px;
}

.pro-coming-soon {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    height: 100%;
    padding: 20px 0;
}

.loader-line {
    width: 40px;
    height: 3px;
    background: var(--control-border);
    position: relative;
    overflow: hidden;
    margin-bottom: 12px;
}

.loader-line::after {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    width: 15px;
    background: var(--item-title-color);
    animation: loader-slide 1.5s infinite ease-in-out;
}

@keyframes loader-slide {
    0% { transform: translateX(-15px); }
    100% { transform: translateX(40px); }
}

.pro-coming-soon p {
    font-size: 11px;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: var(--item-desc-color);
    margin: 0;
    letter-spacing: 0.5px;
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

/* 番茄钟按钮 */
.pomodoro-controls {
    display: flex;
    gap: 6px;
    align-items: center;
}

.pomo-btn {
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid var(--control-border);
    background: var(--card-bg);
    color: var(--item-title-color);
    transition: all 0.2s ease;
    outline: none;
}

.pomo-btn:hover {
    background: var(--control-border);
}

.pomo-btn:disabled {
    cursor: not-allowed;
    opacity: 0.4;
}

.toggle-btn:not(.disabled) {
    background: var(--accent-color, #ff4757);
    border-color: var(--accent-color, #ff4757);
    color: #fff;
}

.toggle-btn:not(.disabled):hover {
    opacity: 0.9;
}

.toggle-btn.disabled {
    background: var(--card-bg);
    border-color: var(--control-border);
    color: var(--item-desc-color);
}

/* ===== 三步设置向导样式 ===== */
.pomo-setup-wizard {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-top: 10px;
}

.pomo-step-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
}

.step-dot {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 800;
    background: var(--control-border);
    color: var(--item-desc-color);
    transition: all 0.3s ease;
}

.step-dot.active {
    background: var(--accent-color, #ff4757);
    color: #fff;
}

.step-dot.done {
    background: #10b981;
    color: #fff;
}

.step-label {
    font-size: 14px;
    font-weight: 700;
    color: var(--h1-color);
    text-align: center;
}

.step-input-row {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
}

.step-input-group {
    display: flex;
    align-items: center;
    gap: 6px;
}

.step-input {
    width: 60px;
    background: rgba(0, 0, 0, 0.03);
    border: 1px solid var(--control-border);
    border-radius: 8px;
    padding: 8px 6px;
    text-align: center;
    font-size: 18px;
    font-weight: 700;
    color: var(--h1-color);
    outline: none;
    transition: all 0.2s ease;
}

.step-input:focus {
    border-color: var(--accent-color, #ff4757);
    background: transparent;
}

:global(.dark-theme) .step-input {
    background: rgba(255, 255, 255, 0.05);
}

.step-input::-webkit-outer-spin-button,
.step-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
}

.step-input[type=number] {
    -moz-appearance: textfield;
    appearance: textfield;
}

.step-unit {
    font-size: 13px;
    font-weight: 600;
    color: var(--item-desc-color);
}

.step-summary {
    text-align: center;
    font-size: 13px;
    font-weight: 600;
    color: var(--subtitle-color);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.step-nav-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
}

.step-next-btn,
.step-start-btn {
    padding: 8px 20px;
    font-size: 13px;
    font-weight: 700;
    border-radius: 8px;
    cursor: pointer;
    border: none;
    background: var(--accent-color, #ff4757);
    color: #fff;
    transition: all 0.2s ease;
    outline: none;
    align-self: center;
    width: fit-content;
}

.step-next-btn:hover,
.step-start-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
}

.step-next-btn:disabled,
.step-start-btn:disabled {
    cursor: not-allowed;
    opacity: 0.4;
    transform: none;
}

.step-back-btn {
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 8px;
    cursor: pointer;
    border: 1px solid var(--control-border);
    background: transparent;
    color: var(--item-title-color);
    transition: all 0.2s ease;
    outline: none;
}

.step-back-btn:hover {
    background: var(--control-border);
}

.pomo-step-content {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

/* ===== 运行中状态样式 ===== */
.pomo-running-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px 0;
}

.pomo-phase-label {
    font-size: 14px;
    font-weight: 700;
    padding: 4px 14px;
    border-radius: 100px;
    border: 1px solid;
}

.pomo-phase-label.focus {
    color: #ff4757;
    border-color: rgba(255, 71, 87, 0.3);
    background: rgba(255, 71, 87, 0.08);
}

.pomo-phase-label.break {
    color: #2196f3;
    border-color: rgba(33, 150, 243, 0.3);
    background: rgba(33, 150, 243, 0.08);
}

.pomo-running-time {
    font-size: 32px;
    font-weight: 900;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    letter-spacing: 3px;
}

.pomo-running-time.focus {
    color: #ff4757;
}

.pomo-running-time.break {
    color: #2196f3;
}

.pomo-running-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--subtitle-color);
}

.pomo-paused-tag {
    background: rgba(255, 193, 7, 0.15);
    color: #f59e0b;
    padding: 2px 10px;
    border-radius: 100px;
    font-size: 11px;
    font-weight: 700;
}

.pomo-ready-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 12px 0;
}

.pomo-ready-summary {
    font-size: 12px;
    font-weight: 700;
    color: var(--subtitle-color);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    text-align: center;
}

.pomo-ready-actions {
    display: flex;
    gap: 8px;
}

/* ===== 倒计时样式 ===== */
.cd-setup {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 10px;
}

.cd-setup-label {
    font-size: 14px;
    font-weight: 700;
    color: var(--h1-color);
    text-align: center;
}

.cd-input-row {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
}

.cd-input-group {
    display: flex;
    align-items: center;
    gap: 6px;
}

.cd-input {
    width: 60px;
    background: rgba(0, 0, 0, 0.03);
    border: 1px solid var(--control-border);
    border-radius: 8px;
    padding: 8px 6px;
    text-align: center;
    font-size: 18px;
    font-weight: 700;
    color: var(--h1-color);
    outline: none;
    transition: all 0.2s ease;
}

.cd-input:focus {
    border-color: #ff9800;
    background: transparent;
}

.cd-input::-webkit-outer-spin-button,
.cd-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
}

.cd-input[type=number] {
    -moz-appearance: textfield;
    appearance: textfield;
}

.cd-unit {
    font-size: 13px;
    font-weight: 600;
    color: var(--item-desc-color);
}

.cd-start-btn {
    padding: 8px 20px;
    font-size: 13px;
    font-weight: 700;
    border-radius: 8px;
    cursor: pointer;
    border: none;
    background: #ff9800;
    color: #fff;
    transition: all 0.2s ease;
    outline: none;
    align-self: center;
    width: fit-content;
}

.cd-start-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
}

.cd-start-btn:disabled {
    cursor: not-allowed;
    opacity: 0.4;
    transform: none;
}

.cd-running-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px 0;
}

.cd-running-time {
    font-size: 32px;
    font-weight: 900;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    letter-spacing: 3px;
    color: #ff9800;
}

.cd-running-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--subtitle-color);
}

.cd-paused-tag {
    background: rgba(255, 193, 7, 0.15);
    color: #f59e0b;
    padding: 2px 10px;
    border-radius: 100px;
    font-size: 11px;
    font-weight: 700;
}

.cd-controls {
    display: flex;
    gap: 8px;
    margin-top: 4px;
}

.cd-btn {
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid var(--control-border);
    background: var(--card-bg);
    color: var(--item-title-color);
    transition: all 0.2s ease;
    outline: none;
}

.cd-btn:hover {
    background: var(--control-border);
}

.cd-stop-btn {
    border-color: #ef4444;
    color: #ef4444;
}

.cd-stop-btn:hover {
    background: rgba(239, 68, 68, 0.1);
}

.cd-finish-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px 0;
}

.cd-finish-text {
    font-size: 18px;
    font-weight: 800;
    color: #ff9800;
    animation: cd-pulse 1s ease-in-out infinite;
}

@keyframes cd-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}

.cd-reset-btn {
    padding: 6px 16px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid var(--control-border);
    background: var(--card-bg);
    color: var(--item-title-color);
    transition: all 0.2s ease;
    outline: none;
}

.cd-reset-btn:hover {
    background: var(--control-border);
}

/* ===== 硬件监控卡片样式 ===== */
.hw-config-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px 0;
}

.hw-toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
}

.hw-toggle-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.hw-toggle-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--item-title-color);
}

.hw-toggle-desc {
    font-size: 11px;
    color: var(--item-desc-color);
}

.hw-section-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--item-desc-color);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
}

.hw-mode-section,
.hw-metric-section {
    display: flex;
    flex-direction: column;
}

.hw-mode-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
}

.hw-mode-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 6px;
    border-radius: 10px;
    border: 1px solid var(--control-border);
    background: var(--card-bg);
    color: var(--item-title-color);
    cursor: pointer;
    transition: all 0.2s ease;
    outline: none;
}

.hw-mode-btn:hover {
    border-color: var(--accent-color, #3b82f6);
}

.hw-mode-btn.active {
    border-color: var(--accent-color, #3b82f6);
    background: rgba(59, 130, 246, 0.08);
}

.mode-icon {
    font-size: 18px;
    line-height: 1;
}

.mode-name {
    font-size: 12px;
    font-weight: 700;
}

.mode-desc {
    font-size: 10px;
    color: var(--item-desc-color);
    text-align: center;
    line-height: 1.3;
}

.hw-metric-row {
    display: flex;
    gap: 16px;
}

.hw-radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--item-title-color);
}

.hw-radio-label input[type="radio"] {
    accent-color: var(--accent-color, #3b82f6);
}

.hw-live-preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.hw-live-preview .hw-section-label {
    margin-bottom: 4px;
}

.hw-live-preview {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 16px;
}

.hw-ring-preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
}

.hw-preview-svg {
    width: 56px;
    height: 56px;
}

.hw-preview-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--item-desc-color);
}

/* ===== 健康提醒 ===== */
.health-config-panel {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 0;
}

.health-reminder-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 4px;
    border-radius: 8px;
    background: var(--card-bg, rgba(255,255,255,0.06));
}

.health-reminder-header {
    display: flex;
    align-items: center;
    gap: 8px;
}

.health-reminder-icon {
    font-size: 18px;
    line-height: 1;
}

.health-reminder-info {
    display: flex;
    flex-direction: column;
}

.health-reminder-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--item-title-color, #fff);
}

.health-reminder-desc {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.health-reminder-controls {
    display: flex;
    align-items: center;
    gap: 8px;
}

.health-time-input-group {
    display: flex;
    align-items: center;
    gap: 4px;
}

.health-time-input {
    width: 48px;
    padding: 2px 4px;
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    border-radius: 4px;
    background: var(--input-bg, rgba(255,255,255,0.08));
    color: var(--text-color, #fff);
    font-size: 12px;
    text-align: center;
    outline: none;
}

.health-time-input:focus {
    border-color: var(--accent-color, #10b981);
}

.health-time-unit {
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.health-divider {
    height: 1px;
    background: var(--control-border, rgba(255,255,255,0.08));
    margin: 2px 0;
}

.health-status-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 4px 4px;
}

.health-status-text {
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.6));
}

.health-status-text.alerting {
    color: #fbbf24;
    font-weight: 600;
}

.health-countdown-text {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.health-skip-btn {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 8px;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    background: transparent;
    color: var(--item-title-color, #fff);
    transition: all 0.2s ease;
    outline: none;
    line-height: 1.6;
}

.health-skip-btn:hover:not(.is-disabled) {
    background: var(--control-border, rgba(255,255,255,0.15));
    border-color: var(--accent-color, #10b981);
    color: var(--accent-color, #10b981);
}

.health-skip-btn.is-disabled {
    opacity: 0.35;
    cursor: not-allowed;
}

</style>