<template>
    <div class="panel-container">
        <div class="custom-titlebar">
            <div data-tauri-drag-region class="titlebar-drag-area"></div>

            <div class="titlebar-controls">
                <button class="titlebar-btn" @click="minimizeWindow">
                    <svg viewBox="0 0 12 12" fill="currentColor">
                        <rect x="1" y="5" width="10" height="1.5" rx="0.5" />
                    </svg>
                </button>
                <button class="titlebar-btn close-btn" @click="closeWindow">
                    <svg viewBox="0 0 12 12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
                        <path d="M2.5 2.5L9.5 9.5M9.5 2.5L2.5 9.5" />
                    </svg>
                </button>
            </div>
        </div>

        <header class="panel-header">
            <div class="brand">
                <img src="../assets/logo.png" class="logo-icon">
                <div>
                    <h1>Music Dynamic Island</h1>
                    <p class="subtitle">MDI桌面灵动岛组件 v{{ appVersion }}</p>
                </div>
            </div>

            <div class="header-controls">
                <button class="dynamicset-btn" :class="{ 'is-active': isDynamicSet }" @click="toggleDynamicSet">
                    {{ currentView === 'main' ? '灵动岛设置' : (currentView === 'island' ? 'LiveActive' : (currentView === 'live' ? '个性化' : '返回设置')) }}
                </button>
                <span class="control-separator"></span>

                <span class="status-badge" :class="{ 'is-active': isWidgetVisible }">
                    {{ isWidgetVisible ? '已开启' : '已关闭' }}
                </span>
                <label class="switch header-switch">
                    <input type="checkbox" :checked="isWidgetVisible" @change="toggleWidget">
                    <span class="slider"></span>
                </label>
            </div>
        </header>

        <hr class="divider" />

        <div class="main-content" :class="{ 'dynamicset-layout': currentView !== 'main' }">
            <template v-if="currentView === 'main'">
                <div class="card status-card" @click="metricDropdownOpen = false">
                    <div class="card-header-row">
                        <div class="metric-dropdown" :class="{ open: metricDropdownOpen }">
                            <button class="metric-trigger" @click.stop="metricDropdownOpen = !metricDropdownOpen">
                                <span>{{ metricLabel }}</span>
                                <svg class="metric-chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                                    <polyline points="6 9 12 15 18 9" stroke-width="2.5" stroke-linecap="round"
                                        stroke-linejoin="round" />
                                </svg>
                            </button>
                            <transition name="metric-menu">
                                <div class="metric-menu" v-show="metricDropdownOpen" @click.stop>
                                    <div class="metric-option" :class="{ active: chartMetric === 'speed' }"
                                        @click="switchMetric('speed')">网速状态</div>
                                    <div class="metric-option" :class="{ active: chartMetric === 'cpu' }"
                                        @click="switchMetric('cpu')">CPU状态</div>
                                    <div class="metric-option" :class="{ active: chartMetric === 'ram' }"
                                        @click="switchMetric('ram')">内存状态</div>
                                </div>
                            </transition>
                        </div>
                        <button class="stats-toggle-btn" @click="toggleRightPanel">
                            {{ rightPanel === 'settings' ? '数据统计' : '退出' }}
                        </button>
                    </div>
                    <div class="speed-monitor">
                        <template v-if="chartMetric === 'speed'">
                            <div class="speed-item">
                                <span class="arrow up">
                                    <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
                                        <path
                                            d="M16 4C16.8 4 17.5 4.3 18.1 4.9L28.1 14.9C29.3 16.1 29.3 18 28.1 19.1C26.9 20.3 25 20.3 23.9 19.1L18 13.2V26C18 27.7 16.7 29 15 29C13.3 29 12 27.7 12 26V13.2L6.1 19.1C4.9 20.3 3 20.3 1.9 19.1C0.7 18 0.7 16.1 1.9 14.9L11.9 4.9C12.5 4.3 13.2 4 14 4H16Z"
                                            fill="currentColor" />
                                    </svg>
                                </span>
                                <div class="speed-info">
                                    <span class="label">上传速度</span>
                                    <span class="value">{{ uploadSpeed }}</span>
                                </div>
                            </div>
                            <div class="speed-item">
                                <span class="arrow down">
                                    <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
                                        <path
                                            d="M16 28C15.2 28 14.5 27.7 13.9 27.1L3.9 17.1C2.7 15.9 2.7 14 3.9 12.9C5.1 11.7 7 11.7 8.1 12.9L14 18.8V6C14 4.3 15.3 3 17 3C18.7 3 20 4.3 20 6V18.8L25.9 12.9C27.1 11.7 29 11.7 30.1 12.9C31.3 14 31.3 15.9 30.1 17.1L20.1 27.1C19.5 27.7 18.8 28 18 28H16Z"
                                            fill="currentColor" />
                                    </svg>
                                </span>
                                <div class="speed-info">
                                    <span class="label">下载速度</span>
                                    <span class="value">{{ downloadSpeed }}</span>
                                </div>
                            </div>
                        </template>
                        <template v-else-if="chartMetric === 'cpu'">
                            <div class="speed-item">
                                <span class="arrow" :style="{ background: 'rgba(249,115,22,0.12)', color: '#f97316' }">
                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round">
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
                                </span>
                                <div class="speed-info">
                                    <span class="label">CPU 占用率</span>
                                    <span class="value" :class="{ 'high-usage': cpuUsageVal >= 90 }">{{ cpuUsageVal }}%</span>
                                </div>
                            </div>
                        </template>
                        <template v-else>
                            <div class="speed-item">
                                <span class="arrow" :style="{ background: 'rgba(16,185,129,0.12)', color: '#10b981' }">
                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round">
                                        <line x1="5" y1="8" x2="19" y2="8" />
                                        <line x1="5" y1="12" x2="19" y2="12" />
                                        <line x1="5" y1="16" x2="19" y2="16" />
                                        <rect x="3" y="4" width="18" height="16" rx="2" />
                                    </svg>
                                </span>
                                <div class="speed-info">
                                    <span class="label">内存占用 ({{ formatMem(memUsedVal) }} / {{ formatMem(memTotalVal) }})</span>
                                    <span class="value" :class="{ 'high-usage': memUsageVal >= 90 }">{{ memUsageVal }}%</span>
                                </div>
                            </div>
                        </template>
                    </div>
                    <div class="mini-chart">
                        <SpeedChart ref="speedChartRef" :data="chartDataQueue" :max-value="chartMaxValue"
                            :color="chartColor" />
                    </div>
                </div>

                <div class="card settings-card" v-if="rightPanel === 'settings'">
                    <h3>常规设置</h3>

                    <div class="setting-item flex-row-item">
                        <div class="item-meta">
                            <span class="item-title">主题颜色</span>
                            <span class="item-desc">切换控制台主题色</span>
                        </div>
                        <select v-model="themeMode" class="theme-select" @change="handleThemeChange">
                            <option value="light">浅色模式</option>
                            <option value="dark">深色模式</option>
                            <option value="system">跟随系统</option>
                        </select>
                    </div>

                    <div class="setting-item">
                        <div class="item-meta">
                            <span class="item-title">开机自启动</span>
                            <span class="item-desc">跟随系统启动 NSD</span>
                        </div>
                        <label class="switch">
                            <input type="checkbox" v-model="autoStart" @change="toggleAutoStart">
                            <span class="slider"></span>
                        </label>
                    </div>

                    <div class="setting-item slider-item">
                        <div class="item-meta" style="width: 100%;">
                            <div class="combo-title-row">
                                <span class="item-title">灵动岛不透明度</span>

                                <span class="title-separator">|</span>

                                <span class="item-title-sec">
                                    置于任务栏
                                    <span class="tooltip-wrapper" data-tooltip="若要在全屏游戏中使用灵动岛建议关闭此项">
                                        <p class="set-item-tips-tag">🙋</p>
                                    </span>
                                </span>

                                <label class="switch mini-switch" style="opacity: 0.8;">
                                    <input type="checkbox" v-model="pinToTaskbar" @change="togglePinTaskbar">
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <span class="item-desc">调节灵动岛的背景透明度 ({{ opacity }}%)</span>
                        </div>

                        <input type="range" min="0" max="100" v-model="opacity" class="range-input" />
                    </div>
                </div>

                <template v-else>
                    <div class="card stats-card">
                        <div class="card-header-row">
                            <h3>数据统计</h3>
                            <select v-model="statChartType" class="theme-select">
                                <option value="bar">柱状图</option>
                                <option value="line">折线图</option>
                            </select>
                        </div>

                        <div class="stats-overview">
                            <div class="stat-box">
                                <span class="stat-label">总上传</span>
                                <span class="stat-val">{{ formatBytesValue(totalUpload) }}</span>
                                <span class="stat-unit">{{ formatBytesUnit(totalUpload) }}</span>
                            </div>
                            <div class="stat-box">
                                <span class="stat-label">总下载</span>
                                <span class="stat-val">{{ formatBytesValue(totalDownload) }}</span>
                                <span class="stat-unit">{{ formatBytesUnit(totalDownload) }}</span>
                            </div>
                            <div class="stat-box">
                                <span class="stat-label">本月流量</span>
                                <span class="stat-val">{{ formatBytesValue(monthTraffic) }}</span>
                                <span class="stat-unit">{{ formatBytesUnit(monthTraffic) }}</span>
                            </div>
                        </div>

                        <div class="stats-chart-container">
                            <StatsChart
                                ref="statsChartRef"
                                :days="statsDays"
                                :up-data="statsUpData"
                                :down-data="statsDownData"
                                :chart-type="statChartType"
                            />
                        </div>
                    </div>
                </template>
            </template>

            <template v-else-if="currentView === 'island'">
                <DynamicSet :pin-to-taskbar="pinToTaskbar" :position-locked="positionLocked" />
            </template>

            <template v-else-if="currentView === 'live'">
                <LiveActive />
            </template>

            <template v-else-if="currentView === 'personalize'">
                <PersonalizeCenter />
            </template>
        </div>

        <footer class="panel-footer">
            <div class="ft_left">
                <span>&copy; 2026 lkjhg10576 All rights
                    reserved.</span>
                <span>MDI v{{ appVersion }}</span>
            </div>
            <div class="ft_right">
                <span class="action-link" @click="openNSDweb">官方网站</span>
                <span class="action-link" @click="openNSDdata">开源数据</span>
                <span class="action-link" @click="openMywebsite">作者主页</span>
                <div class="update-check-group">
                    <span class="action-link"
                        :style="{ opacity: isChecking ? 0.5 : 1, pointerEvents: isChecking ? 'none' : 'auto', position: 'relative' }"
                        @click="checkUpdate">
                        <span v-if="hasNewVersion" class="update-dot"></span>
                        {{ isChecking ? '检查中...' : (hasNewVersion ? '检测到新版本' : '检查更新') }}
                    </span>
                    <label class="beta-check-switch" @click.stop>
                        <span class="beta-check-label">检测Beta版本</span>
                        <span class="switch mini-switch">
                            <input type="checkbox" :checked="checkBeta" @change="onCheckBetaChange">
                            <span class="slider"></span>
                        </span>
                    </label>
                </div>
            </div>
        </footer>

        <Transition name="fade">
            <div v-if="dialog.visible" class="modal-overlay" @click.self="closeDialog">
                <div class="modal-card">
                    <div class="modal-header">
                        <h4>{{ dialog.title }}</h4>
                    </div>
                    <div class="modal-body">
                        <p>{{ dialog.message }}</p>
                    </div>
                    <div class="modal-footer">
                        <button v-if="dialog.isConfirm" class="btn btn-secondary" @click="closeDialog">取消</button>
                        <button class="btn btn-primary" @click="handleDialogConfirm">确定</button>
                    </div>
                </div>
            </div>
        </Transition>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { getVersion } from '@tauri-apps/api/app';
import SpeedChart from '../components/SpeedChart.vue';
import StatsChart from '../components/StatsChart.vue';
import DynamicSet from '../components/DynamicSet.vue';
import LiveActive from './LiveActive.vue';
import PersonalizeCenter from '../components/PersonalizeCenter.vue';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { openUrl } from '@tauri-apps/plugin-opener';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { formatSpeed } from '../utils/format';
import {
    NSD_ISLAND_OPACITY,
    NSD_PIN_TASKBAR,
    NSD_POSITION_LOCKED, NSD_DESTROY_ON_CLOSE,
    NSD_THEME_MODE, NSD_TARGET_PLAYER, NSD_TRAFFIC_STATS,
    NSD_CHART_METRIC,
    NSD_CHECK_BETA,
} from '../constants/storageKeys';

const isWidgetVisible = ref(false);
const autoStart = ref(false);
const opacity = ref(Number(localStorage.getItem(NSD_ISLAND_OPACITY) || '100'));

const savedTheme = localStorage.getItem(NSD_THEME_MODE) || 'light';
const themeMode = ref(['light', 'dark', 'system'].includes(savedTheme) ? savedTheme : 'light');

const uploadSpeed = ref('0 B/s');
const downloadSpeed = ref('0 B/s');

const appVersion = ref('1.0.0');

const isDynamicSet = computed(() => currentView.value !== 'main');

// 四视图状态：'main' | 'island' | 'live' | 'personalize'
const currentView = ref<'main' | 'island' | 'live' | 'personalize'>('main');

// 切换视图：主设置 → 岛设置 → LiveActive → 个性化 → 主设置
const toggleDynamicSet = () => {
    if (currentView.value === 'main') currentView.value = 'island';
    else if (currentView.value === 'island') currentView.value = 'live';
    else if (currentView.value === 'live') currentView.value = 'personalize';
    else currentView.value = 'main';
};

const isChecking = ref(false);
const hasNewVersion = ref(false);
const checkBeta = ref(localStorage.getItem(NSD_CHECK_BETA) === 'true');

// 置于任务栏状态，默认从本地存储读取
const pinToTaskbar = ref(localStorage.getItem(NSD_PIN_TASKBAR) === 'true');
// 切换开关时保存本地并发送信号给灵动岛
const togglePinTaskbar = async () => {
    localStorage.setItem(NSD_PIN_TASKBAR, String(pinToTaskbar.value));
    await emit('control-pin-taskbar', { enabled: pinToTaskbar.value });
};

// 灵动岛位置锁定状态，默认从本地存储读取
const positionLocked = ref(localStorage.getItem(NSD_POSITION_LOCKED) === 'true');
// 注意：位置解锁功能现在通过右键菜单实现，不再通过控制台按钮

// 控制窗口功能
const minimizeWindow = async () => {
    await getCurrentWindow().minimize();
};
const closeWindow = async () => {
    // 直接调用后端命令：由后端在命令上下文里决定 hide（省内存关）还是 destroy（省内存开）
    await invoke('close_main_window');
};

// 切换灵动岛设置时，重绘图表
watch(currentView, async (newVal) => {
    if (newVal === 'main') {
        await nextTick();
        speedChartRef.value?.resize();
        if (rightPanel.value === 'stats') {
            statsChartRef.value?.resize();
        }
    }
});

const rightPanel = ref<'settings' | 'stats'>('settings');
const statChartType = ref<'bar' | 'line'>('bar');
const statsChartRef = ref<InstanceType<typeof StatsChart> | null>(null);

const trafficData = ref<Record<string, { up: number; down: number }>>({});
let saveThrottleCounter = 0;

// 格式化字节数为人类可读格式
const formatBytesValue = (bytes: number) => {
    if (bytes === 0) return '0';
    const k = 1024;
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)).toString();
};

const formatBytesUnit = (bytes: number) => {
    if (bytes === 0) return 'B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return sizes[i];
};

const totalUpload = computed(() => Object.values(trafficData.value).reduce((acc, curr) => acc + curr.up, 0));
const totalDownload = computed(() => Object.values(trafficData.value).reduce((acc, curr) => acc + curr.down, 0));
const monthTraffic = computed(() => {
    const currentMonth = getLocalYYYYMMDD(new Date()).slice(0, 7);
    return Object.entries(trafficData.value)
        .filter(([date]) => date.startsWith(currentMonth))
        .reduce((acc, [, data]) => acc + data.up + data.down, 0);
});

// 统计图表数据
const statsDays = computed(() => {
    const days: string[] = [];
    for (let i = 6; i >= 0; i--) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        days.push(getLocalYYYYMMDD(d).slice(5));
    }
    return days;
});

const statsUpData = computed(() => {
    const data: number[] = [];
    for (let i = 6; i >= 0; i--) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        const dateStr = getLocalYYYYMMDD(d);
        const dayData = trafficData.value[dateStr] || { up: 0, down: 0 };
        data.push(Number((dayData.up / (1024 * 1024)).toFixed(2)));
    }
    return data;
});

const statsDownData = computed(() => {
    const data: number[] = [];
    for (let i = 6; i >= 0; i--) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        const dateStr = getLocalYYYYMMDD(d);
        const dayData = trafficData.value[dateStr] || { up: 0, down: 0 };
        data.push(Number((dayData.down / (1024 * 1024)).toFixed(2)));
    }
    return data;
});

// 获取本地日期格式为 YYYY-MM-DD
const getLocalYYYYMMDD = (date: Date) => {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
};

// 加载网络流量统计
const loadTrafficData = () => {
    try {
        const stored = localStorage.getItem(NSD_TRAFFIC_STATS);
        if (stored) trafficData.value = JSON.parse(stored);
    } catch (e) {
        console.error("加载统计数据失败", e);
    }
};
loadTrafficData();

// 切换右侧面板
const toggleRightPanel = async () => {
    rightPanel.value = rightPanel.value === 'settings' ? 'stats' : 'settings';
    localStorage.setItem(NSD_TRAFFIC_STATS, JSON.stringify(trafficData.value));
    saveThrottleCounter = 0;

    // 侧边栏布局变化会挤压左侧卡片，强制让实时走势图重新计算高宽
    await nextTick();
    speedChartRef.value?.resize();
};

const toggleAutoStart = async () => {
    try {
        if (autoStart.value) {
            await enable();
        } else {
            await disable();
        }
    } catch (error) {
        autoStart.value = !autoStart.value;
        showDialog('设置失败', '无法修改开机自启动状态，请检查系统权限。');
    }
};

const dialog = ref({
    visible: false,
    title: 'Music Dynamic Island',
    message: '',
    isConfirm: false,
    callback: null as (() => void) | null
});

const showDialog = (title: string, message: string, isConfirm = false, onConfirm: (() => void) | null = null) => {
    dialog.value = { visible: true, title, message, isConfirm, callback: onConfirm };
};

const closeDialog = () => {
    dialog.value.visible = false;
};

const handleDialogConfirm = () => {
    if (dialog.value.callback) dialog.value.callback();
    closeDialog();
};

const parseVersion = (v: string) => {
    // 使用正则匹配出类似于 X.Y.Z 的纯数字版本号部分
    // 兼容 v1.2.3、v1.3.0-beta.1 等标签，按基础版本参与比较
    const match = v.match(/\d+\.\d+\.\d+/);
    if (match) {
        return match[0].split('.').map(Number);
    }
    // 如果实在没匹配到，返回 [0, 0, 0] 防止代码崩溃
    return [0, 0, 0];
};

/** 比较两个版本数组：remote > local 返回 1，相等 0，小于 -1 */
const compareVersions = (remote: number[], local: number[]): number => {
    for (let i = 0; i < 3; i++) {
        const rNum = remote[i] || 0;
        const lNum = local[i] || 0;
        if (rNum > lNum) return 1;
        if (rNum < lNum) return -1;
    }
    return 0;
};

/** 判断 remote 版本字符串是否比 local 更新 */
const isNewerVersion = (remoteVersionStr: string, localVersionStr: string): boolean => {
    return compareVersions(parseVersion(remoteVersionStr), parseVersion(localVersionStr)) > 0;
};

type GithubRelease = {
    tag_name?: string;
    html_url?: string;
    draft?: boolean;
    prerelease?: boolean;
};

const GITHUB_RELEASES_API = 'https://api.github.com/repos/lkjhg10576/Music_Dynamic_Island/releases';
const GITHUB_API_HEADERS = {
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'Tauri-App-NetSpeed-Dynamic',
} as const;

/**
 * 按 checkBeta 选择目标发行版：
 * - false: 请求 releases/latest（仅正式 Latest）
 * - true:  请求 releases 列表，排除 draft，允许 prerelease，选出版本最高且比本地新的
 */
const fetchTargetRelease = async (
    _localVersionStr: string,
    includeBeta: boolean,
    signal?: AbortSignal,
): Promise<{ release: GithubRelease | null; notFound: boolean }> => {
    if (!includeBeta) {
        const response = await fetch(`${GITHUB_RELEASES_API}/latest`, {
            method: 'GET',
            headers: { ...GITHUB_API_HEADERS },
            signal,
        });
        if (response.status === 404) {
            return { release: null, notFound: true };
        }
        if (!response.ok) {
            throw new Error(`GitHub API error: ${response.status}`);
        }
        const data = await response.json() as GithubRelease;
        if (!data?.tag_name) {
            return { release: null, notFound: true };
        }
        return { release: data, notFound: false };
    }

    // Beta 模式：获取发行版列表（含 Pre-release），排除 draft
    const response = await fetch(`${GITHUB_RELEASES_API}?per_page=30`, {
        method: 'GET',
        headers: { ...GITHUB_API_HEADERS },
        signal,
    });
    if (response.status === 404) {
        return { release: null, notFound: true };
    }
    if (!response.ok) {
        throw new Error(`GitHub API error: ${response.status}`);
    }

    const list = await response.json() as GithubRelease[];
    if (!Array.isArray(list) || list.length === 0) {
        return { release: null, notFound: true };
    }

    const candidates = list.filter((item) => {
        if (!item || item.draft) return false;
        if (!item.tag_name || !/\d+\.\d+\.\d+/.test(item.tag_name)) return false;
        return true;
    });

    if (candidates.length === 0) {
        return { release: null, notFound: true };
    }

    // 不依赖 API 返回顺序：按版本号选出最高的发行版
    let best = candidates[0];
    for (let i = 1; i < candidates.length; i++) {
        const cur = candidates[i];
        if (compareVersions(parseVersion(cur.tag_name!), parseVersion(best.tag_name!)) > 0) {
            best = cur;
        }
    }

    // 仅当比本地新时才视为目标更新版本；否则返回最高版供“已是最新”判断
    return { release: best, notFound: false };
};

const onCheckBetaChange = (e: Event) => {
    const enabled = (e.target as HTMLInputElement).checked;
    const wasEnabled = checkBeta.value;
    checkBeta.value = enabled;
    localStorage.setItem(NSD_CHECK_BETA, String(enabled));
    // 仅从关闭切换为开启时立即检测一次
    if (enabled && !wasEnabled) {
        checkUpdate();
    }
};

let lastRx = 0;
let lastTx = 0;
let systemThemeMedia: MediaQueryList;
let unlistenMonitorStats: (() => void) | null = null;

const speedChartRef = ref<InstanceType<typeof SpeedChart> | null>(null);
const chartDataQueue = ref<number[]>(Array(15).fill(0));

// --- F7 实时状态下拉：网速 / CPU / 内存 ---
type ChartMetric = 'speed' | 'cpu' | 'ram';
const savedMetric = localStorage.getItem(NSD_CHART_METRIC) || '';
const chartMetric = ref<ChartMetric>(['speed', 'cpu', 'ram'].includes(savedMetric) ? (savedMetric as ChartMetric) : 'speed');
const metricDropdownOpen = ref(false);

// CPU / 内存实时数值（用于卡片数字展示）
const cpuUsageVal = ref(0);
const memUsageVal = ref(0);
const memUsedVal = ref(0);
const memTotalVal = ref(0);

const metricLabel = computed(() => {
    if (chartMetric.value === 'cpu') return 'CPU状态';
    if (chartMetric.value === 'ram') return '内存状态';
    return '网速状态';
});

// 折线图固定上限：CPU/内存为 100，网速为 0（动态）
const chartMaxValue = computed(() => (chartMetric.value === 'cpu' || chartMetric.value === 'ram') ? 100 : 0);

// 折线图主色：CPU 橙色、内存 绿色、网速 默认蓝
const chartColor = computed(() => {
    if (chartMetric.value === 'cpu') return '#f97316';
    if (chartMetric.value === 'ram') return '#10b981';
    return ''; // 空串走 SpeedChart 默认蓝
});

// 切换指标：清空折线图、立即拉取一次新数据
const switchMetric = async (m: ChartMetric) => {
    if (chartMetric.value === m) {
        metricDropdownOpen.value = false;
        return;
    }
    chartMetric.value = m;
    localStorage.setItem(NSD_CHART_METRIC, m);
    // 立即清空，给用户"已切换"的视觉反馈
    chartDataQueue.value = Array(15).fill(0);
    metricDropdownOpen.value = false;
};

// 格式化内存字节数为人类可读
const formatMem = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
};

// 折线图数据压入辅助：保持滚动窗口长度 15
const pushChartData = (val: number) => {
    const newData = [...chartDataQueue.value, val];
    if (newData.length > 15) newData.shift();
    chartDataQueue.value = newData;
};

const openMywebsite = () => {
    openUrl('https://github.com/lkjhg10576');
}

// 静默检查更新（后台偷偷查，不弹窗，报错了也不干扰用户）
const silentCheckUpdate = async () => {
    try {
        const localVersionStr = await getVersion();
        const { release } = await fetchTargetRelease(localVersionStr, checkBeta.value);
        if (!release?.tag_name) return;

        if (isNewerVersion(release.tag_name, localVersionStr)) {
            hasNewVersion.value = true; // 发现新版本，把红点亮起来
        }
    } catch (error) {
        // 静默模式失败就当无事发生
    }
};

const openNSDweb = async () => {
    openUrl('https://nsd.georgewu.top/');
}

const openNSDdata = async () => {
    openUrl('https://nsd.georgewu.top/#stats');
}

const checkUpdate = async () => {
    if (isChecking.value) return; // 防止连点
    isChecking.value = true;

    try {
        const localVersionStr = await getVersion();

        // 加一个 10 秒超时控制器
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 10000);

        const { release, notFound } = await fetchTargetRelease(
            localVersionStr,
            checkBeta.value,
            controller.signal,
        );

        clearTimeout(timeoutId);

        if (notFound || !release?.tag_name) {
            showDialog('检查更新', '未找到可用版本');
            return;
        }

        const remoteVersionStr = release.tag_name;
        const findNew = isNewerVersion(remoteVersionStr, localVersionStr);

        if (findNew) {
            hasNewVersion.value = true;
            showDialog(
                '发现新版本',
                `发现新版本 ${remoteVersionStr}！当前版本为 v${localVersionStr}。是否前往 GitHub 下载更新？`,
                true,
                () => {
                    if (release.html_url) {
                        openUrl(release.html_url);
                    }
                    hasNewVersion.value = false; // 用户点击去更新后，消掉红点并恢复文字
                }
            );
        } else {
            hasNewVersion.value = false;
            showDialog('提示', '当前已是最新版本！');
        }
    } catch (error: any) {
        console.error('检查更新时出错:', error);
        // 精准识别是不是超时导致的
        if (error.name === 'AbortError') {
            showDialog('网络超时', '连接 GitHub 超时，请检查网络或稍后再试');
        } else if (typeof error?.message === 'string' && error.message.startsWith('GitHub API error:')) {
            showDialog('检查更新', '检查更新失败，请稍后再试');
        } else {
            showDialog('网络错误', '请求失败，请检查您的网络连接');
        }
    } finally {
        isChecking.value = false; // 无论成功失败，最后都恢复状态
    }
};

const applyTheme = () => {
    const root = document.documentElement;
    if (themeMode.value === 'dark') {
        root.classList.add('dark-theme');
    } else if (themeMode.value === 'light') {
        root.classList.remove('dark-theme');
    } else if (themeMode.value === 'system') {
        const media = window.matchMedia('(prefers-color-scheme: dark)');
        if (media.matches) {
            root.classList.add('dark-theme');
        } else {
            root.classList.remove('dark-theme');
        }
    }
};

const handleThemeChange = () => {
    localStorage.setItem(NSD_THEME_MODE, themeMode.value);
    applyTheme();
};

const handleSystemThemeUpdate = () => {
    if (themeMode.value === 'system') {
        applyTheme();
    }
};

watch(opacity, async (newVal) => {
    localStorage.setItem(NSD_ISLAND_OPACITY, newVal.toString());
    await emit('control-island-opacity', { opacity: newVal });
});

onMounted(async () => {
    // 告诉 Rust 上次绑定的目标是谁（直接从存储读取，避免持有整套灵动岛 UI 状态）
    const savedTargetPlayer = localStorage.getItem(NSD_TARGET_PLAYER) || 'netease';
    await invoke('set_target_player', { player: savedTargetPlayer }).catch(() => { });

    // 同步省内存模式设置到后端
    const savedDestroyOnClose = localStorage.getItem(NSD_DESTROY_ON_CLOSE) === 'true';
    await invoke('set_destroy_on_close', { enabled: savedDestroyOnClose }).catch(() => { });

    silentCheckUpdate();

    window.addEventListener('contextmenu', (e) => {
        e.preventDefault();
    }, { capture: true });

    applyTheme();
    systemThemeMedia = window.matchMedia('(prefers-color-scheme: dark)');
    systemThemeMedia.addEventListener('change', handleSystemThemeUpdate);

    // 监听后端推送的 monitor-stats 事件（硬件 + 网速统一）
    unlistenMonitorStats = await listen<any>('monitor-stats', (event) => {
        const p = event.payload;
        // 网速数据
        if (typeof p.download_speed === 'number') {
            downloadSpeed.value = formatSpeed(p.download_speed);
        }
        if (typeof p.upload_speed === 'number') {
            uploadSpeed.value = formatSpeed(p.upload_speed);
        }
        // 流量统计累计
        if (typeof p.download_bytes === 'number' && typeof p.upload_bytes === 'number') {
            const todayStr = getLocalYYYYMMDD(new Date());
            if (!trafficData.value[todayStr]) {
                trafficData.value[todayStr] = { up: 0, down: 0 };
            }
            // 用本次累计值 - 上次累计值 = 差值
            if (lastRx > 0) {
                const rxDiff = Math.max(0, p.download_bytes - lastRx);
                const txDiff = Math.max(0, p.upload_bytes - lastTx);
                trafficData.value[todayStr].down += rxDiff;
                trafficData.value[todayStr].up += txDiff;
                saveThrottleCounter++;
                if (saveThrottleCounter >= 5) {
                    localStorage.setItem(NSD_TRAFFIC_STATS, JSON.stringify(trafficData.value));
                    saveThrottleCounter = 0;
                }
            }
            lastRx = p.download_bytes;
            lastTx = p.upload_bytes;
        }
        // 网速模式：折线图填充下载速度
        if (chartMetric.value === 'speed' && typeof p.download_speed === 'number') {
            const speedMB = p.download_speed / (1024 * 1024);
            pushChartData(speedMB);
        }
        // CPU / 内存数据
        if (typeof p.cpu_pct === 'number') {
            cpuUsageVal.value = Math.round(p.cpu_pct);
        }
        if (typeof p.mem_pct === 'number') {
            memUsageVal.value = Math.round(p.mem_pct);
        }
        if (typeof p.used_mem === 'number') {
            memUsedVal.value = p.used_mem;
        }
        if (typeof p.total_mem === 'number') {
            memTotalVal.value = p.total_mem;
        }
        // CPU / 内存模式：折线图填充
        if (chartMetric.value === 'cpu' && typeof p.cpu_pct === 'number') {
            pushChartData(cpuUsageVal.value);
        }
        if (chartMetric.value === 'ram' && typeof p.mem_pct === 'number') {
            pushChartData(memUsageVal.value);
        }
    });

    window.addEventListener('resize', () => {
        speedChartRef.value?.resize();
        statsChartRef.value?.resize();
    });

    try {
        autoStart.value = await isEnabled();
    } catch (e) {
        console.error("获取自启动状态失败:", e);
    }

    try {
        appVersion.value = await getVersion();
    } catch (e) {
        console.error("获取应用版本号失败:", e);
    }

    // 监听来自灵动岛右键菜单的“打开设置”信号
    await listen('open-settings-panel', async () => {
        // 1. 如果当前在主设置页，就切到灵动岛设置页
        if (currentView.value === 'main') {
            currentView.value = 'island';
        }

        // 2. 唤醒并聚焦主窗口
        const appWindow = getCurrentWindow();
        await appWindow.show();        // 确保窗口显示
        await appWindow.unminimize();  // 如果最小化了，就恢复
        await appWindow.setFocus();    // 强制抢占焦点弹到最前面
    });

    // 监听来自灵动岛右键菜单的位置锁定同步信号
    await listen<{ locked: boolean }>('position-lock-sync', (event) => {
        positionLocked.value = event.payload.locked;
    });

    await listen<{ visible: boolean }>('island-status-sync', (event) => {
        isWidgetVisible.value = event.payload.visible;
    });

    for (let i = 0; i < 6; i++) {
        try {
            const visible = await invoke<boolean>('is_widget_visible');
            if (visible) {
                isWidgetVisible.value = true;
                return;
            }
        } catch { /* 忽略 */ }
        await new Promise(r => setTimeout(r, 200));
    }
    isWidgetVisible.value = false;
});

onUnmounted(() => {
    systemThemeMedia?.removeEventListener('change', handleSystemThemeUpdate);
    if (unlistenMonitorStats) unlistenMonitorStats();
    localStorage.setItem(NSD_TRAFFIC_STATS, JSON.stringify(trafficData.value));
});

const toggleWidget = async () => {
    const nextState = !isWidgetVisible.value;
    await emit('control-island-visibility', { show: nextState });
    isWidgetVisible.value = nextState;
};
</script>

<style scoped>
/*提取出的颜色变量层*/
:global(:root) {
    --bg-body: #f8fafc;
    --text-body: #1e293b;
    --h1-color: #0f172a;
    --subtitle-color: #798089;
    --control-bg: #ffffff;
    --control-border: #e2e8f0;
    --status-badge-inactive: #94a3b8;
    --status-badge-active: #2b2b2b;
    --divider-border: #e2e8f0;
    --card-bg: #ffffff;
    --card-border: #e2e8f0;
    --card-shadow: rgba(0, 0, 0, 0.03);
    --card-shadow-hover: rgba(0, 0, 0, 0.06);
    --card-h3-color: #334155;
    --arrow-up-bg: #eff6ff;
    --arrow-up-color: #3b82f6;
    --arrow-down-bg: #ecfdf5;
    --arrow-down-color: #10b981;
    --speed-label: #64748b;
    --speed-value: #0f172a;
    --chart-border: #f1f5f9;
    --item-title-color: #1e293b;
    --tag-dev-bg: #f1f5f9;
    --tag-dev-color: #64748b;
    --item-desc-color: #898f99df;
    --slider-bg: #d7dce2;
    --slider-checked-bg: #b9b9b9;
    --slider-disabled-bg: #e2e8f0;
    --range-bg: #e2e8f0;
    --range-thumb-bg: #ffffff;
    --range-thumb-border: #2b2b2b;
    --range-thumb-shadow: rgba(0, 0, 0, 0.3);
    --footer-text: #2b2b2b89;
    --overlay-bg: rgba(15, 23, 42, 0.3);
    --modal-bg: #ffffff;
    --modal-border: #e2e8f0;
    --modal-h4: #0f172a;
    --modal-p: #64748b;
    --btn-sec-bg: #ebebeb;
    --btn-sec-color: #64748b;
    --btn-sec-border: #e2e8f0;
    --btn-sec-hover-bg: #e2e8f0;
    --btn-sec-hover-color: #334155;
    --btn-pri-bg: #2b2b2b;
    --btn-pri-color: #ffffff;
    --btn-pri-border: #2b2b2b;
    --btn-pri-hover-bg: #1a1a1a;
    --btn-pri-shadow-hover: rgba(0, 0, 0, 0.15);
    --select-bg: #ffffff;
    --select-border: #e2e8f0;
    --select-text: #1e293b;
    --data-tag-bg: #ececec;
    --data-tag-color: #2b2b2b;
}

/*暗色模式变量覆盖*/
:global(.dark-theme) {
    --bg-body: #1e2021;
    --text-body: #cbd5e1;
    --h1-color: #f8fafc;
    --subtitle-color: #a5aeba;
    --control-bg: #292b2e;
    --control-border: #383c41;
    --status-badge-inactive: #64748b;
    --status-badge-active: #f8fafc;
    --divider-border: #334155;
    --card-bg: #292b2e;
    --card-border: #383c41;
    --card-shadow: rgba(0, 0, 0, 0.2);
    --card-shadow-hover: rgba(0, 0, 0, 0.3);
    --card-h3-color: #e2e8f0;
    --arrow-up-bg: rgba(59, 130, 246, 0.15);
    --arrow-up-color: #60a5fa;
    --arrow-down-bg: rgba(16, 185, 129, 0.15);
    --arrow-down-color: #34d399;
    --speed-label: #94a3b8;
    --speed-value: #f8fafc;
    --chart-border: #474c53;
    --item-title-color: #f8fafc;
    --tag-dev-bg: #334155;
    --tag-dev-color: #94a3b8;
    --item-desc-color: #898f99df;
    --slider-bg: #3e4247;
    --slider-checked-bg: #5d646d;
    --slider-disabled-bg: #334155;
    --range-bg: #42474e;
    --range-thumb-bg: #1e293b;
    --range-thumb-border: #60a5fa;
    --range-thumb-shadow: rgba(0, 0, 0, 0.5);
    --footer-text: #8b8f96aa;
    --overlay-bg: rgba(0, 0, 0, 0.6);
    --modal-bg: #292b2e;
    --modal-border: #383c41;
    --modal-h4: #f8fafc;
    --modal-p: #94a3b8;
    --btn-sec-bg: #1a1a1a;
    --btn-sec-color: #cbd5e1;
    --btn-sec-border: #475569;
    --btn-sec-hover-bg: #475569;
    --btn-sec-hover-color: #f8fafc;
    --btn-pri-bg: #1a1a1a;
    --btn-pri-color: #ffffff;
    --btn-pri-border: #2b2b2b;
    --btn-pri-hover-bg: #161616;
    --btn-pri-shadow-hover: rgba(0, 0, 0, 0.15);
    --select-bg: #292b2e;
    --select-border: #383c41;
    --select-text: #f8fafc;
    --data-tag-bg: #202020;
    --data-tag-color: #f8fafc;
}

/*原有布局及节点样式 */
:global(html) {
    color: var(--text-body);
    transition: background-color 0.3s ease, color 0.3s ease;
}

:global(body) {
    background-color: transparent !important;
    color: inherit;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Roboto, sans-serif;
    user-select: none;
    -webkit-font-smoothing: antialiased;
}

.panel-container {
    background-color: var(--bg-body);
    padding: 36px 32px 28px 32px;
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    min-height: calc(100vh - 56px);
    position: relative;
}

.panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
}

.brand {
    display: flex;
    align-items: center;
    gap: 16px;
}

.logo-icon {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
}

.brand h1 {
    font-size: 20px;
    margin: 0;
    font-weight: 700;
    letter-spacing: 0.2px;
    color: var(--h1-color);
}

.subtitle {
    font-size: 13px;
    color: var(--subtitle-color);
    margin: 4px 0 0 0;
}

.header-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--control-bg);
    padding: 8px 16px;
    border-radius: 24px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
    border: 1px solid var(--control-border);
}

.status-badge {
    font-size: 13px;
    font-weight: 600;
    color: var(--status-badge-inactive);
    transition: all 0.3s;
}

.status-badge.is-active {
    color: var(--status-badge-active);
}

.divider {
    border: none;
    border-top: 1px solid var(--divider-border);
    margin-bottom: 24px;
}

.main-content {
    display: grid;
    grid-template-columns: 1fr 1.3fr;
    gap: 18px;
    flex-grow: 1;
    transition: all 0.3s ease;
}

/* 游戏模式自适应列宽 */
.main-content.game-mode-layout {
    grid-template-columns: 1fr;
}

.card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 20px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 4px 20px -2px var(--card-shadow);
    transition: transform 0.2s, box-shadow 0.2s;
}

.card:hover {
    box-shadow: 0 8px 24px -4px var(--card-shadow-hover);
}

.card h3 {
    font-size: 15px;
    color: var(--card-h3-color);
    margin: 0 0 20px 0;
    font-weight: 600;
}

.speed-monitor {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-bottom: 24px;
}

.speed-item {
    display: flex;
    align-items: center;
    gap: 16px;
}

.arrow {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 16px;
}

.arrow svg {
    width: 20px;
    height: 20px;
}

.arrow.up {
    background: var(--arrow-up-bg);
    color: var(--arrow-up-color);
}

.arrow.down {
    background: var(--arrow-down-bg);
    color: var(--arrow-down-color);
}

.speed-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.speed-info .label {
    font-size: 12px;
    color: var(--speed-label);
    font-weight: 500;
}

.speed-info .value {
    font-size: 22px;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: var(--speed-value);
    letter-spacing: -0.5px;
}

.mini-chart {
    width: 100%;
    height: 80px;
    margin-top: auto;
    padding-top: 16px;
    border-top: 1px solid var(--chart-border);
}

.setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0;
    border-bottom: 1px solid var(--chart-border);
}

.setting-item:last-child {
    border-bottom: none;
    padding-bottom: 0;
}

.slider-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 16px;
}

.flex-row-item {
    flex-direction: row;
    align-items: center;
}

.item-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.item-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--item-title-color);
    display: flex;
    align-items: center;
    gap: 8px;
}

.item-title-sec {
    height: 22px;
    font-size: 14px;
    font-weight: 600;
    color: var(--item-title-color);
    opacity: 0.8;
    display: flex;
    align-items: center;
}

.item-desc {
    font-size: 13px;
    color: var(--item-desc-color);
}

.switch {
    position: relative;
    display: inline-block;
    width: 48px;
    height: 28px;
}

.switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--slider-bg);
    transition: 0.4s cubic-bezier(0.4, 0.0, 0.2, 1);
    border-radius: 28px;
}

.slider:before {
    position: absolute;
    content: "";
    height: 22px;
    width: 22px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transition: 0.4s cubic-bezier(0.4, 0.0, 0.2, 1);
    border-radius: 50%;
}

input:checked+.slider {
    background-color: var(--slider-checked-bg);
}

input:checked+.slider:before {
    transform: translateX(20px);
}

.range-input {
    width: 100%;
    -webkit-appearance: none;
    appearance: none;
    background: var(--range-bg);
    height: 6px;
    border-radius: 3px;
    outline: none;
}

.range-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--range-thumb-bg);
    border: 2px solid var(--range-thumb-border);
    cursor: pointer;
    box-shadow: 0 2px 6px var(--range-thumb-shadow);
    transition: transform 0.1s;
}

.range-input::-webkit-slider-thumb:hover {
    transform: scale(1.1);
}

.panel-footer {
    margin-top: 25px;
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    font-size: 12px;
    color: var(--footer-text);
    font-weight: 500;
}

.panel-footer span {
    display: flex;
}

.ft_left {
    display: flex;
    flex-direction: row;
    justify-content: left;
    align-items: center;
    gap: 10px;
}

.openmywebsite {
    background: none;
    border: none;
    cursor: pointer;
    outline: none;
    font-size: 12px;
    color: var(--footer-text);
    font-weight: bold;
}

.openmywebsite:hover {
    text-decoration: underline;
}

.ft_right {
    display: flex;
    flex-direction: row;
    justify-content: right;
    align-items: center;
    gap: 13px;
}

.action-link {
    color: var(--footer-text);
    cursor: pointer;
    transition: color 0.2s;
}

.action-link:hover {
    color: var(--footer-text);
    text-decoration: underline;
}

.update-check-group {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
}

.beta-check-switch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
}

.beta-check-label {
    font-size: 11px;
    color: var(--footer-text);
    opacity: 0.85;
    white-space: nowrap;
}

.beta-check-switch .mini-switch {
    width: 32px;
    height: 18px;
    flex-shrink: 0;
}

.beta-check-switch .mini-switch .slider:before {
    height: 12px;
    width: 12px;
    left: 3px;
    bottom: 3px;
}

.beta-check-switch .mini-switch input:checked + .slider:before {
    transform: translateX(14px);
}

.update-dot {
    position: absolute;
    top: 2px;
    right: -8px;
    width: 5px;
    height: 5px;
    background-color: #ff3b30;
    border-radius: 50%;
    box-shadow: 0 0 4px rgba(255, 59, 48, 0.4);
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: var(--overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
}

.modal-card {
    background: var(--modal-bg);
    border: 1px solid var(--modal-border);
    border-radius: 20px;
    width: 360px;
    padding: 24px;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

.modal-header h4 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 700;
    color: var(--modal-h4);
}

.modal-body p {
    margin: 0 0 24px 0;
    font-size: 14px;
    color: var(--modal-p);
    line-height: 1.5;
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
}

.btn {
    padding: 8px 18px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    outline: none;
}

.btn-secondary {
    background: var(--btn-sec-bg);
    color: var(--btn-sec-color);
    border: 1px solid var(--btn-sec-border);
}

.btn-secondary:hover {
    background: var(--btn-sec-hover-bg);
    color: var(--btn-sec-hover-color);
}

.btn-primary {
    background: var(--btn-pri-bg);
    color: var(--btn-pri-color);
    border: 1px solid var(--btn-pri-border);
}

.btn-primary:hover {
    background: var(--btn-pri-hover-bg);
    box-shadow: 0 4px 12px var(--btn-pri-shadow-hover);
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.25s ease, transform 0.25s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

.fade-enter-from .modal-card {
    transform: scale(0.95);
}

.fade-leave-to .modal-card {
    transform: scale(0.95);
}

.theme-select {
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 8px;
    background-color: var(--select-bg);
    border: 1px solid var(--select-border);
    color: var(--select-text);
    outline: none;
    cursor: pointer;
    transition: all 0.2s ease;
}

.theme-select:hover {
    border-color: var(--slider-checked-bg);
}



/* 灵动岛设置按钮样式 */
.dynamicset-btn {
    background: transparent;
    border: 1px solid var(--control-border);
    color: var(--text-body);
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 16px;
    cursor: pointer;
    transition: all 0.2s ease;
}

.dynamicset-btn:hover {
    background: var(--btn-sec-bg);
    border-color: var(--slider-checked-bg);
}

.dynamicset-btn.is-active {
    background: var(--btn-pri-bg);
    color: var(--btn-pri-color);
    border-color: var(--btn-pri-border);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.control-separator {
    width: 1px;
    height: 16px;
    background: var(--control-border);
}



/* =========================================
   灵动岛设置面板 - 扁平化全宽布局
   ========================================= */

/* 核心修复：当处于灵动岛设置模式时，强制单列全宽，解决只有半宽的问题 */
.main-content.dynamicset-layout {
    grid-template-columns: 1fr !important;
}


.set-item-tips-tag {
    font-size: 10px;
    font-weight: bold;
    color: var(--btn-pri-color);
    background: transparent;
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 6px;
    max-height: 24px;
    transform: translateY(1px);
    opacity: 0.8;
}

/* Tooltip 容器 */
.tooltip-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    cursor: help;
    /* 鼠标悬停时显示帮助光标 */
}

/* Tooltip 气泡本体 */
.tooltip-wrapper::after {
    content: attr(data-tooltip);
    position: absolute;
    bottom: calc(100% + 2px);
    /* 位于元素上方 8px */
    left: 50%;
    transform: translateX(-50%) translateY(4px);
    /* 初始位置稍微偏下，用于动画 */

    /* 样式：复用现有主题变量 */
    background: var(--modal-bg);
    color: var(--text-body);
    border: 1px solid var(--card-border);
    box-shadow: 0 4px 12px var(--card-shadow-hover);

    padding: 8px 12px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    line-height: 1.4;
    white-space: nowrap;
    /* 保持单行，若文本过长可改为 normal 并设置 max-width */

    /* 交互与动画 */
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 999;
}

/* Hover 时显示 */
.tooltip-wrapper:hover::after {
    opacity: 1;
    visibility: visible;
    transform: translateX(-50%) translateY(0);
}

.tooltip-wrapper:hover::before {
    opacity: 1;
    transform: translateX(-50%) scale(1);
}

/* 数据统计模块样式 */
.card-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.card-header-row h3 {
    margin-bottom: 0;
}

.stats-toggle-btn {
    background: transparent;
    color: var(--item-title-color);
    border: 1px solid var(--chart-border);
    padding: 4px 10px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
}

.stats-toggle-btn:hover {
    background: var(--btn-sec-bg);
}

/* F7 实时状态下拉菜单 */
.metric-dropdown {
    position: relative;
}

.metric-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--card-h3-color);
    transition: color 0.2s ease;
}

.metric-trigger:hover {
    opacity: 0.75;
}

.metric-chevron {
    width: 16px;
    height: 16px;
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.metric-dropdown.open .metric-chevron {
    transform: rotate(180deg);
}

.metric-menu {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    min-width: 120px;
    background: var(--modal-bg);
    border: 1px solid var(--card-border);
    border-radius: 10px;
    box-shadow: 0 8px 24px var(--card-shadow-hover);
    padding: 4px;
    z-index: 100;
}

.metric-option {
    padding: 8px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-body);
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s ease;
}

.metric-option:hover {
    background: var(--btn-sec-bg);
}

.metric-option.active {
    color: var(--btn-pri-bg);
    font-weight: 700;
}

.metric-menu-enter-active,
.metric-menu-leave-active {
    transition: opacity 0.18s ease, transform 0.18s ease;
}

.metric-menu-enter-from,
.metric-menu-leave-to {
    opacity: 0;
    transform: translateY(-6px);
}

/* CPU/内存高占用警示 */
.value.high-usage {
    color: #ef4444;
}

.stats-card {
    display: flex;
    flex-direction: column;
}

.stats-overview {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
}

.stat-box {
    flex: 1;
    background: var(--control-bg);
    border: 1px solid var(--card-border);
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: start;
    gap: 4px;
    height: 70px;
    /* 固定高度，不会因内容变化而撑高 */
    box-sizing: border-box;
    position: relative;
}

.stat-label {
    font-size: 12px;
    color: var(--item-desc-color);
    font-weight: 500;
    flex-shrink: 0;
    transform: translateY(-5px);
}

.stat-val {
    font-size: 16px;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: var(--speed-value);
    white-space: nowrap;
    /* 数值不会换行 */
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    flex-shrink: 0;
    transform: translateY(-5px);
}

.stat-unit {
    font-size: 10px;
    font-weight: 600;
    color: var(--item-desc-color);
    opacity: 0.7;
    flex-shrink: 0;
    position: absolute;
    bottom: 3px;
    background: var(--data-tag-bg);
    color: var(--data-tag-color);
    padding: 1px 5px;
    border-radius: 4px;
}

.stats-chart-container {
    width: 100%;
    flex-grow: 1;
    min-height: 180px;
    border-top: 1px solid var(--chart-border);
    padding-top: 16px;
}


/* =========================================
   常规设置 - 标题与开关缝合样式
   ========================================= */

/* 标题行横向排列，垂直居中 */
.combo-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
}

/* 分割线样式 */
.title-separator {
    color: var(--control-border);
    font-size: 14px;
    opacity: 0.8;
}

/* 缩小的迷你开关，靠左对齐紧贴着标题 */
.mini-switch {
    transform: scale(0.65);
    transform-origin: left center;
    margin: 0;
}

input:disabled+.slider {
    cursor: not-allowed;
    opacity: 0.5;
}

/* 自定义标题栏 */
.custom-titlebar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 32px;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    z-index: 9999;
    border-top-left-radius: inherit;
    border-top-right-radius: inherit;
}

.titlebar-drag-area {
    flex-grow: 1;
    height: 100%;
    -webkit-app-region: drag;
}

.titlebar-controls {
    display: flex;
    height: 100%;
    -webkit-app-region: no-drag;
}

.titlebar-btn {
    background: transparent;
    border: none;
    color: var(--text-body);
    width: 45px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.2s ease, color 0.2s ease;
}

.titlebar-btn svg {
    width: 11px;
    height: 11px;
    opacity: 0.8;
}

.titlebar-btn:hover {
    background-color: var(--btn-sec-bg);
}

.titlebar-btn:hover svg {
    opacity: 1;
}

.close-btn:hover {
    background-color: #ff4757 !important;
    color: #ffffff !important;
}

/* 硬件监控模式选择 */
.hw-mode-select-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
}

.hw-mode-label {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    font-size: 12px;
    color: var(--item-title-color);
    user-select: none;
}

.hw-mode-label.mini {
    font-size: 11px;
}

.hw-mode-label input[type="radio"] {
    accent-color: #3b82f6;
    margin: 0;
}

.hw-default-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
}

.hw-default-label {
    font-size: 11px;
    color: var(--item-desc-color);
}

</style>