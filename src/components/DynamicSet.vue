<template>
    <div class="dynamicset-grid">
        <div class="set-item-top"
            style="grid-column: span 2; flex-direction: column; align-items: flex-start; justify-content: center; gap: 8px;">
            <div class="set-item-meta"
                style="flex-direction: row; justify-content: space-between; width: 100%; align-items: center;">
                <span class="set-item-title-top">音乐控制平台</span>
                <span class="set-item-desc" style="font-size: 11px;">选择灵动岛显示的音乐平台</span>
            </div>
            <div class="capsule-switch player-grid">
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'netease' }"
                    @click="setTargetPlayer('netease')">
                    <img src="../assets/musci163.svg" class="platform-icon" alt="icon">
                    网易云
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'spotify' }"
                    @click="setTargetPlayer('spotify')">
                    <img src="../assets/Spotify.svg" class="platform-icon" alt="icon">
                    Spotify
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'apple' }"
                    @click="setTargetPlayer('apple')">
                    <img src="../assets/applemusic.svg" class="platform-icon" alt="icon">
                    Apple
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'qqmusic' }"
                    @click="setTargetPlayer('qqmusic')">
                    <img src="../assets/qqmusic.svg" class="platform-icon" alt="icon">
                    QQ音乐
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'kugou' }"
                    @click="setTargetPlayer('kugou')">
                    <img src="../assets/kugou.svg" class="platform-icon" alt="icon">
                    酷狗
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'echo' }"
                    @click="setTargetPlayer('echo')">
                    <img src="../assets/echomusic.ico" class="platform-icon" alt="icon">
                    EchoMusic
                </div>
                <div class="capsule-btn" :class="{ 'is-active': targetPlayer === 'lx-music' }"
                    @click="setTargetPlayer('lx-music')">
                    <img src="../assets/lxmusic.png" class="platform-icon" alt="icon">
                    洛雪音乐
                </div>
                <div class="capsule-btn smtc-btn" :class="{ 'is-active': targetPlayer === 'smtc' }"
                    @click="setTargetPlayer('smtc')">
                    <svg class="platform-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z" fill="currentColor"/>
                    </svg>
                    SMTC
                </div>
            </div>
        </div>
        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">灵动岛颜色</span>
                <span class="set-item-desc">切换灵动岛的默认背景色调</span>
            </div>
            <div class="capsule-switch">
                <div class="capsule-btn" :class="{ 'is-active': islandTheme === 'black' }"
                    @click="islandTheme = 'black'">暗色</div>
                <div class="capsule-btn" :class="{ 'is-active': islandTheme === 'white' }"
                    @click="islandTheme = 'white'">亮色</div>
            </div>
        </div>

        <div class="set-item" :class="{ 'disabled-set-item': enableRotation }">
            <div class="set-item-meta">
                <span class="set-item-title">音乐控制器 <p class="set-item-pro-tag">PRO</p></span>
                <span class="set-item-desc">{{ enableRotation ? '轮换开启中，已禁用' : '支持网易云音乐控制及歌曲信息显示' }}</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="enableMusicCtrl" :disabled="enableRotation">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item spectrum-color-item">
            <div class="set-item-meta">
                <span class="set-item-title">频谱颜色</span>
                <span class="set-item-desc">频谱条取色方式</span>
            </div>
            <div class="spectrum-color-controls">
                <div class="capsule-switch">
                    <div class="capsule-btn" :class="{ 'is-active': spectrumColorMode === 'album' }"
                        @click="spectrumColorMode = 'album'">跟随专辑</div>
                    <div class="capsule-btn" :class="{ 'is-active': spectrumColorMode === 'theme' }"
                        @click="spectrumColorMode = 'theme'">跟随主题</div>
                    <div class="capsule-btn" :class="{ 'is-active': spectrumColorMode === 'custom' }"
                        @click="spectrumColorMode = 'custom'">自定义</div>
                </div>
                <input
                    v-if="spectrumColorMode === 'custom'"
                    type="color"
                    class="spectrum-color-picker"
                    v-model="spectrumCustomColor"
                    title="自定义频谱颜色"
                />
            </div>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">消息通知接收 <p class="set-item-pro-tag">PRO</p></span>
                <span class="set-item-desc">启用系统控制中心消息弹窗提醒</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="enableMsgNotify" @change="toggleMsgNotify">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item" :class="{ 'disabled-set-item': enableRotation }">
            <div class="set-item-meta">
                <span class="set-item-title">静默消息模式</span>
                <span class="set-item-desc">{{ enableRotation ? '轮换开启中，已禁用' : '平时自动隐藏，收到消息后才弹出' }}</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="msgModeEnabled" @change="toggleMsgMode"
                    :disabled="enableRotation">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">自动隐藏</span>
                <span class="set-item-desc">鼠标离开灵动岛后自动收起</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="autoHideEnabled" @change="toggleAutoHide">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item" v-if="autoHideEnabled">
            <div class="set-item-meta">
                <span class="set-item-title">隐藏延迟时间</span>
                <span class="set-item-desc">鼠标离开后延迟收起的时间 ({{ autoHideDelay / 1000 }}秒)</span>
            </div>
            <div class="delay-input-container">
                <button class="delay-btn" @click="autoHideDelay = Math.max(100, autoHideDelay - 500); updateAutoHideDelay()">-</button>
                <input type="number" v-model.number="autoHideDelay" @change="updateAutoHideDelay"
                    class="delay-input" min="100" max="10000" step="500">
                <button class="delay-btn" @click="autoHideDelay = Math.min(10000, autoHideDelay + 500); updateAutoHideDelay()">+</button>
            </div>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">自动折叠</span>
                <span class="set-item-desc">{{ autoCollapseEnabled ? '展开后鼠标离开自动折叠回小岛状态' : '关闭后需手动折叠展开的灵动岛' }}</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="autoCollapseEnabled" @change="toggleAutoCollapse">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item" v-if="autoCollapseEnabled">
            <div class="set-item-meta">
                <span class="set-item-title">折叠延迟时间</span>
                <span class="set-item-desc">鼠标离开后延迟折叠的时间 ({{ autoCollapseDelay / 1000 }}秒)</span>
            </div>
            <div class="delay-input-container">
                <button class="delay-btn" @click="autoCollapseDelay = Math.max(100, autoCollapseDelay - 500); updateAutoCollapseDelay()">-</button>
                <input type="number" v-model.number="autoCollapseDelay" @change="updateAutoCollapseDelay"
                    class="delay-input" min="100" max="10000" step="500">
                <button class="delay-btn" @click="autoCollapseDelay = Math.min(10000, autoCollapseDelay + 500); updateAutoCollapseDelay()">+</button>
            </div>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">全屏自动隐藏</span>
                <span class="set-item-desc">游戏/视频全屏时自动隐藏灵动岛</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="autoHideFullscreen" @change="toggleAutoHideFS">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">灵动岛位置</span>
                <span class="set-item-desc">{{ positionLocked ? '已锁定，右键灵动岛可解锁' : (pinToTaskbar ? '已解锁，任务栏模式下仅可横向拖动' : '已解锁，可自由拖动调整位置') }}</span>
            </div>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">灵动岛轮换</span>
                <span class="set-item-desc">在网速岛、音乐岛间轮换</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="enableRotation" @change="toggleRotation">
                <span class="slider"></span>
            </label>
        </div>

        <div class="set-item">
            <div class="set-item-meta">
                <span class="set-item-title">省内存模式</span>
                <span class="set-item-desc">{{ destroyOnClose ? '关闭控制台时彻底释放内存，重新打开会稍慢' : '关闭控制台时仅隐藏到后台' }}</span>
            </div>
            <label class="switch">
                <input type="checkbox" v-model="destroyOnClose" @change="toggleDestroyOnClose">
                <span class="slider"></span>
            </label>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import {
    NSD_ISLAND_THEME,
    NSD_MUSIC_CTRL, NSD_MSG_NOTIFY,
    NSD_MSG_MODE, NSD_ROTATION_MODE,
    NSD_DESTROY_ON_CLOSE,
    NSD_AUTO_HIDE_ENABLED, NSD_AUTO_HIDE_DELAY,
    NSD_AUTO_COLLAPSE_ENABLED, NSD_AUTO_COLLAPSE_DELAY,
    NSD_TARGET_PLAYER, NSD_AUTO_HIDE_FS,
    NSD_SPECTRUM_COLOR_MODE, NSD_SPECTRUM_CUSTOM_COLOR,
} from '../constants/storageKeys';

defineProps<{
    pinToTaskbar: boolean;
    positionLocked: boolean;
}>();

// --- 音乐控制平台切换功能 ---
const targetPlayer = ref(localStorage.getItem(NSD_TARGET_PLAYER) || 'netease');

const setTargetPlayer = async (player: string) => {
    targetPlayer.value = player;
    localStorage.setItem(NSD_TARGET_PLAYER, player); // 本地记忆化
    try {
        await invoke('set_target_player', { player }); // 秒发给 Rust 立即生效
    } catch (e) {
        console.error('切换平台失败', e);
    }
};

// 灵动岛设置相关的 UI 状态绑定
const islandTheme = ref(localStorage.getItem(NSD_ISLAND_THEME) || 'black');
const enableMusicCtrl = ref(localStorage.getItem(NSD_MUSIC_CTRL) === 'true');
const spectrumColorMode = ref(localStorage.getItem(NSD_SPECTRUM_COLOR_MODE) || 'album');
const spectrumCustomColor = ref(localStorage.getItem(NSD_SPECTRUM_CUSTOM_COLOR) || '#b6e0ee');
const enableMsgNotify = ref(localStorage.getItem(NSD_MSG_NOTIFY) === 'true');
const msgModeEnabled = ref(localStorage.getItem(NSD_MSG_MODE) === 'true');
const enableRotation = ref(localStorage.getItem(NSD_ROTATION_MODE) === 'true');

// 省内存模式：关闭控制台时彻底销毁主窗口 WebView，释放 50-120MB 内存（B1 方案）
const destroyOnClose = ref(localStorage.getItem(NSD_DESTROY_ON_CLOSE) === 'true');

// 自动隐藏相关变量
const autoHideEnabled = ref(localStorage.getItem(NSD_AUTO_HIDE_ENABLED) === 'true');
const autoHideDelay = ref(Number(localStorage.getItem(NSD_AUTO_HIDE_DELAY) || '2000')); // 默认2秒

// 自动折叠相关变量（灵动岛展开后，鼠标离开自动折叠回小岛状态）
const autoCollapseEnabled = ref(localStorage.getItem(NSD_AUTO_COLLAPSE_ENABLED) === 'true');
const autoCollapseDelay = ref(Number(localStorage.getItem(NSD_AUTO_COLLAPSE_DELAY) || '2000')); // 默认2秒

// 切换消息模式
const toggleMsgMode = async () => {
    // 如果开启静默模式，则强制开启消息通知并同步本地存储
    if (msgModeEnabled.value) { enableMsgNotify.value = true; toggleMsgNotify(); }

    localStorage.setItem(NSD_MSG_MODE, String(msgModeEnabled.value));
    await emit('control-msg-mode', { enabled: msgModeEnabled.value });
};

// 新增切换保存方法
const toggleMsgNotify = () => {
    localStorage.setItem(NSD_MSG_NOTIFY, String(enableMsgNotify.value));
};

// 切换自动隐藏设置
const toggleAutoHide = async () => {
    localStorage.setItem(NSD_AUTO_HIDE_ENABLED, String(autoHideEnabled.value));
    localStorage.setItem(NSD_AUTO_HIDE_DELAY, String(autoHideDelay.value));
    await emit('control-auto-hide', { enabled: autoHideEnabled.value, delay: autoHideDelay.value });
};

// 更新自动隐藏延迟时间
const updateAutoHideDelay = async () => {
    // 确保延迟时间在合理范围内（100ms到10秒）
    autoHideDelay.value = Math.max(100, Math.min(10000, autoHideDelay.value));
    localStorage.setItem(NSD_AUTO_HIDE_DELAY, String(autoHideDelay.value));
    await emit('control-auto-hide', { enabled: autoHideEnabled.value, delay: autoHideDelay.value });
};

// 切换自动折叠设置
const toggleAutoCollapse = async () => {
    localStorage.setItem(NSD_AUTO_COLLAPSE_ENABLED, String(autoCollapseEnabled.value));
    localStorage.setItem(NSD_AUTO_COLLAPSE_DELAY, String(autoCollapseDelay.value));
    await emit('control-auto-collapse', { enabled: autoCollapseEnabled.value, delay: autoCollapseDelay.value });
};

// 更新自动折叠延迟时间
const updateAutoCollapseDelay = async () => {
    // 确保延迟时间在合理范围内（100ms到10秒）
    autoCollapseDelay.value = Math.max(100, Math.min(10000, autoCollapseDelay.value));
    localStorage.setItem(NSD_AUTO_COLLAPSE_DELAY, String(autoCollapseDelay.value));
    await emit('control-auto-collapse', { enabled: autoCollapseEnabled.value, delay: autoCollapseDelay.value });
};

// 全屏自动隐藏开关
const autoHideFullscreen = ref(localStorage.getItem(NSD_AUTO_HIDE_FS) === 'true');
const toggleAutoHideFS = async () => {
    localStorage.setItem(NSD_AUTO_HIDE_FS, String(autoHideFullscreen.value));
    await emit('control-autohide-fs', { enabled: autoHideFullscreen.value });
};

// 切换省内存模式
const toggleDestroyOnClose = async () => {
    localStorage.setItem(NSD_DESTROY_ON_CLOSE, String(destroyOnClose.value));
    try {
        await invoke('set_destroy_on_close', { enabled: destroyOnClose.value });
    } catch (e) {
        console.error('同步省内存模式失败', e);
    }
};

// 切换灵动岛轮换模式
const toggleRotation = async () => {
    // 1. 保存并发送轮换功能的开关状态
    localStorage.setItem(NSD_ROTATION_MODE, String(enableRotation.value));
    await emit('control-rotation-mode', { enabled: enableRotation.value });

    // ✨ 新增限制逻辑：如果用户【开启】了轮换功能
    if (enableRotation.value) {
        // 强行将静默消息模式设为关闭（false）
        msgModeEnabled.value = false;
        // 同步更新本地电脑的记忆状态
        localStorage.setItem(NSD_MSG_MODE, 'false');
        // 发送信号通知灵动岛浮窗：静默模式已关闭，请立刻现身
        await emit('control-msg-mode', { enabled: false });
    }
};

watch(islandTheme, async (newVal) => {
    localStorage.setItem(NSD_ISLAND_THEME, newVal);
    await emit('control-island-theme', { theme: newVal });
    console.log('灵动岛颜色切换为:', newVal);
});

// 添加监听器，将状态同步给灵动岛
watch(enableMusicCtrl, async (newVal) => {
    localStorage.setItem(NSD_MUSIC_CTRL, newVal.toString());
    await emit('control-music-ctl', { enabled: newVal });
    console.log('音乐控制器状态切换为:', newVal);
});

// 频谱颜色模式 / 自定义色 → 写存储并同步到灵动岛
const syncSpectrumColor = async () => {
    localStorage.setItem(NSD_SPECTRUM_COLOR_MODE, spectrumColorMode.value);
    localStorage.setItem(NSD_SPECTRUM_CUSTOM_COLOR, spectrumCustomColor.value);
    await emit('control-spectrum-color', {
        mode: spectrumColorMode.value,
        color: spectrumCustomColor.value,
    });
};
watch(spectrumColorMode, syncSpectrumColor);
watch(spectrumCustomColor, syncSpectrumColor);
</script>

<style scoped>
/* 双列网格结构，自动填充 */
.dynamicset-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 20px;
    padding: 0;
    box-shadow: 0 4px 20px -2px var(--card-shadow);
    max-height: calc(100vh - 180px);
    overflow-y: auto;
    align-content: start;
}

/* 👇 新增：自定义迷你滑块 (Webkit 标准) */
.dynamicset-grid::-webkit-scrollbar {
    width: 5px;
}

.dynamicset-grid::-webkit-scrollbar-track {
    background: transparent;
    margin: 12px 0;
}

.dynamicset-grid::-webkit-scrollbar-thumb {
    background-color: var(--slider-bg);
    border-radius: 10px;
}

.dynamicset-grid::-webkit-scrollbar-thumb:hover {
    background-color: var(--slider-checked-bg);
}

/* 设置项：去掉独立背景和边框，融入容器 */
.set-item {
    background: transparent;
    border: none;
    margin: 0;
    border-radius: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 92px;
    padding: 0 24px;
    box-sizing: border-box;
}

.set-item-top {
    background: transparent;
    border: none;
    margin: 0;
    border-radius: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 75px;
    padding: 0 24px;
    box-sizing: border-box;
    transform: translateY(5px);
}

.disabled-set-item {
    opacity: 0.6;
}

.set-item-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.set-item-title {
    font-size: 14px;
    font-weight: 600;
    display: flex;
    align-items: center;
    max-height: 24px;
    color: var(--item-title-color);
}

.set-item-title-top {
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    max-height: 24px;
    color: var(--item-title-color);
}

.set-item-pro-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--btn-pri-color);
    background: var(--btn-pri-bg);
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 6px;
    max-height: 24px;
}

.set-item-new-tag {
    font-size: 10px;
    font-weight: 600;
    color: #dd1f1f;
    background: #b9101020;
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 6px;
    max-height: 24px;
    transform: translateY(1px);
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

.set-item-desc {
    font-size: 12px;
    color: var(--item-desc-color);
}

/* 胶囊形滑块开关 (保持不变，仅确保层级) */
.capsule-switch {
    display: flex;
    background: var(--slider-bg);
    border-radius: 6px;
    padding: 2px;
    position: relative;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.05);
    flex-shrink: 0;
}

.capsule-btn {
    padding: 4px 14px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-body);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 1;
    opacity: 0.6;
}

.capsule-btn.is-active {
    background: var(--card-bg);
    color: var(--item-title-color);
    box-shadow: 0 1px 4px var(--card-shadow-hover);
    opacity: 1;
}

.spectrum-color-item {
    align-items: flex-start;
}

.spectrum-color-controls {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
}

.spectrum-color-picker {
    width: 36px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
}

/* 开关样式（scoped 下父组件样式不穿透，需在此保留） */
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

input:disabled+.slider {
    cursor: not-allowed;
    opacity: 0.5;
}

/* 音乐平台六宫格样式 */
.player-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    width: 100%;
    padding: 4px;
    box-sizing: border-box;
}

/* SMTC按钮特殊样式 */
.smtc-btn {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(16, 185, 129, 0.1));
    border: 1px dashed rgba(59, 130, 246, 0.3);
}

.smtc-btn.is-active {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.2), rgba(16, 185, 129, 0.2));
    border: 1px solid rgba(59, 130, 246, 0.5);
}

.smtc-btn:hover {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.15), rgba(16, 185, 129, 0.15));
}

/* 改用 flex 布局，让图片和文字完美对齐 */
.player-grid .capsule-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    /* 图标和文字之间的间距 */
    padding: 4px 0;
    font-size: 11px;
    white-space: nowrap;
    flex: 0 0 calc((100% - 24px) / 7);
    min-width: 60px;
}

/* 限制图标的尺寸，避免图片太大把盒子撑爆 */
.platform-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
    /* 保证图标不变形 */
    opacity: 0.8;
    /* 给一点点透明度，显得不那么刺眼 */
    transition: opacity 0.2s ease;
    transform: translateX(-3px) translateY(1px);
    border-radius: 3px;
}

/* 当选中该平台时，让图标变亮完全不透明 */
.capsule-btn.is-active .platform-icon {
    opacity: 1;
}

/* 自动隐藏延迟输入框样式 */
.delay-input-container {
    display: flex;
    align-items: center;
    gap: 8px;
}

.delay-btn {
    width: 32px;
    height: 32px;
    border: 1px solid var(--control-border);
    background: var(--control-bg);
    color: var(--text-body);
    border-radius: 8px;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
}

.delay-btn:hover {
    background: var(--btn-sec-bg);
    border-color: var(--slider-checked-bg);
}

.delay-input {
    width: 80px;
    height: 32px;
    border: 1px solid var(--control-border);
    background: var(--control-bg);
    color: var(--text-body);
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    text-align: center;
    outline: none;
    transition: all 0.2s ease;
}

.delay-input:focus {
    border-color: var(--slider-checked-bg);
}

/* 隐藏数字输入框的上下箭头 */
.delay-input::-webkit-outer-spin-button,
.delay-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
}

.delay-input[type=number] {
    -moz-appearance: textfield;
}
</style>
