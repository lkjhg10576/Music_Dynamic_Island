<template>
    <div class="clipboard-manager" @click="onRootClick">
        <div class="cm-page">
            <!-- ===== 顶部：标题 + 总开关 ===== -->
            <div class="cm-toolbar">
                <span class="cm-page-title">剪贴板历史 <span class="cm-count" v-if="items.length">{{ items.length }}</span></span>
                <label class="switch" @click.stop>
                    <input type="checkbox" :checked="enabled" @change="toggleEnabled">
                    <span class="slider"></span>
                </label>
            </div>
            <p class="cm-subtitle">开启后自动记录复制的文本与图片，保留 3 天</p>

            <!-- ===== 关闭时的提示条：历史保留、可搜索、可复制 ===== -->
            <div v-if="!enabled" class="cm-paused-bar">
                ⏸ 记录已暂停，以下为历史内容，仍可复制使用
            </div>

            <!-- ===== 二级设置：灵动岛提示（总开关关时整个不渲染） ===== -->
            <div v-if="enabled" class="cm-setting-item">
                <div class="cm-setting-meta">
                    <span class="cm-setting-title">灵动岛提示</span>
                    <span class="cm-setting-desc">复制时在灵动岛弹出简短提示（默认关闭）</span>
                </div>
                <label class="switch" @click.stop>
                    <input type="checkbox" :checked="islandToast" @change="toggleIslandToast">
                    <span class="slider"></span>
                </label>
            </div>

            <!-- ===== 工具栏：搜索 / 清空全部 / 存储限制 ===== -->
            <div class="cm-toolbar-row">
                <div class="cm-search">
                    <svg class="cm-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                        stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                    </svg>
                    <input v-model="search" class="cm-search-input" type="text" placeholder="搜索剪贴板内容…">
                    <svg v-if="search" class="cm-search-clear" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                        stroke-width="2" stroke-linecap="round" stroke-linejoin="round" @click.stop="search = ''">
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                </div>
                <button class="cm-btn" :class="{ 'cm-btn-danger': clearArmed }" @click.stop="onClearClick">
                    {{ clearArmed ? '确认清空？' : '清空全部' }}
                </button>
                <div class="cm-quota-wrap">
                    <button class="cm-btn" @click.stop="quotaOpen = !quotaOpen">存储限制 ▾</button>
                    <div v-if="quotaOpen" class="cm-quota-pop">
                        <div class="cm-quota-line">图片 {{ imageCount }}/{{ MAX_IMAGES }}　{{ formatBytes(imageBytes) }} / {{ formatBytes(MAX_IMAGE_BYTES) }}</div>
                        <div class="cm-quota-line">总条目 {{ items.length }}/{{ MAX_ITEMS }}</div>
                    </div>
                </div>
            </div>

            <!-- ===== 条目卡片列表 ===== -->
            <div class="cm-list">
                <div v-if="filteredItems.length === 0" class="cm-empty">
                    {{ search ? '没有匹配的内容' : '暂无剪贴板历史。开启开关后，复制的内容会自动记录在这里' }}
                </div>
                <div v-for="item in filteredItems" :key="item.id" class="cm-card" :class="{ 'is-pinned': item.pinned }">
                    <!-- 头部：类型 + 常驻置顶星；时间 ↔ 悬停图标用绝对定位叠加，无布局抖动 -->
                    <div class="cm-card-head">
                        <span class="cm-kind">{{ item.kind === 'text' ? '文本' : '图片' }}</span>
                        <span v-if="item.pinned" class="cm-pin-badge">★</span>
                        <span class="cm-head-right">
                            <span class="cm-time">{{ formatRelativeTime(item.ts_ms) }}</span>
                            <span class="cm-actions">
                                <button class="cm-icon-btn" :title="item.pinned ? '取消置顶' : '置顶'"
                                    @click.stop="onPinToggle(item)">{{ item.pinned ? '★' : '☆' }}</button>
                                <button class="cm-icon-btn cm-icon-del" title="删除" @click.stop="onDelete(item)">✕</button>
                            </span>
                        </span>
                    </div>
                    <!-- 中间：点击复制；复制成功反馈半透明覆盖后淡出 -->
                    <div class="cm-card-body" @click="onCopy(item)">
                        <pre v-if="item.kind === 'text'" class="cm-text">{{ item.text }}</pre>
                        <img v-else-if="item.thumb_path" class="cm-thumb" :src="convertFileSrc(item.thumb_path)"
                            :alt="imgMeta(item)">
                        <div v-else class="cm-thumb-placeholder">🖼 暂无预览</div>
                        <div class="cm-copied-overlay" :class="{ 'is-visible': copiedId === item.id }">✓ 已复制</div>
                    </div>
                    <!-- 底栏：字符数 / 尺寸与体积 -->
                    <div class="cm-card-foot">
                        {{ item.kind === 'text' ? `${item.char_len} 个字符` : imgMeta(item) }}
                    </div>
                </div>
            </div>

            <p class="cm-hint">点击卡片中间区域即可复制；置顶条目在下次进入本页时排在最上方</p>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
    useClipboard, formatRelativeTime, formatBytes, type ClipItem,
} from '../composables/useClipboard';
import { NSD_CLIPBOARD_ENABLED, NSD_CLIPBOARD_ISLAND_TOAST } from '../constants/storageKeys';
import { getSettingRaw, setSettingRaw } from '../utils/settings';

// ===== 配额常量（与后端 Rust 常量一致，仅用于展示） =====
const MAX_ITEMS = 100;
const MAX_IMAGES = 20;
const MAX_IMAGE_BYTES = 100 * 1024 * 1024;

const {
    items, refresh, setEnabled, copyItem, togglePin, deleteItem, clearAll, bindEvents, unbindEvents,
} = useClipboard();

// ===== 开关状态（config.json 单一数据源） =====
const enabled = ref(getSettingRaw(NSD_CLIPBOARD_ENABLED) === 'true');
const islandToast = ref(getSettingRaw(NSD_CLIPBOARD_ISLAND_TOAST) === 'true');

async function toggleEnabled(e: Event) {
    const next = (e.target as HTMLInputElement).checked;
    enabled.value = next;
    setSettingRaw(NSD_CLIPBOARD_ENABLED, String(next));
    try {
        await setEnabled(next);
    } catch (err) {
        console.error('切换剪贴板监听失败:', err);
        enabled.value = !next;
        setSettingRaw(NSD_CLIPBOARD_ENABLED, String(!next));
    }
}

async function toggleIslandToast(e: Event) {
    const next = (e.target as HTMLInputElement).checked;
    islandToast.value = next;
    setSettingRaw(NSD_CLIPBOARD_ISLAND_TOAST, String(next));
}

// ===== 工具栏：搜索（前端本地过滤） =====
const search = ref('');

const filteredItems = computed(() => {
    const q = search.value.trim().toLowerCase();
    if (!q) return items.value;
    return items.value.filter((it) => {
        if (it.kind === 'text') {
            return (it.text ?? '').toLowerCase().includes(q);
        }
        // 图片：匹配尺寸（两种分隔写法）与相对时间
        const dims = it.img_w && it.img_h ? `${it.img_w}x${it.img_h} ${it.img_w}×${it.img_h}` : '';
        return `${dims} ${formatRelativeTime(it.ts_ms)}`.toLowerCase().includes(q);
    });
});

// ===== 存储限制下拉：配额统计（与后端常量同口径，置顶计入不豁免） =====
const imageCount = computed(() => items.value.filter((it) => it.kind === 'image').length);
const imageBytes = computed(() =>
    items.value.filter((it) => it.kind === 'image').reduce((acc, it) => acc + it.img_bytes, 0)
);

// ===== 清空全部：首次点击进入确认态（红色背景）；点外部还原 / 10 秒超时自动还原 =====
const clearArmed = ref(false);
let clearTimer: number | null = null;

function onClearClick() {
    if (!clearArmed.value) {
        clearArmed.value = true;
        clearTimer = window.setTimeout(disarmClear, 10_000);
        return;
    }
    disarmClear();
    clearAll()
        .then(refresh)
        .catch((err) => console.error('清空剪贴板历史失败:', err));
}

function disarmClear() {
    clearArmed.value = false;
    if (clearTimer !== null) {
        clearTimeout(clearTimer);
        clearTimer = null;
    }
}

// ===== 存储限制下拉：点击外部关闭（与清空确认态共用根容器 click） =====
const quotaOpen = ref(false);

function onRootClick() {
    if (clearArmed.value) disarmClear();
    if (quotaOpen.value) quotaOpen.value = false;
}

// ===== 条目操作 =====
// 复制成功反馈：中间区域半透明覆盖，约 800ms 淡出
const copiedId = ref<string | null>(null);
let copiedTimer: number | null = null;

async function onCopy(item: ClipItem) {
    try {
        await copyItem(item.id);
        copiedId.value = item.id;
        if (copiedTimer !== null) clearTimeout(copiedTimer);
        copiedTimer = window.setTimeout(() => {
            copiedId.value = null;
            copiedTimer = null;
        }, 800);
    } catch (err) {
        console.error('复制失败:', err);
    }
}

async function onPinToggle(item: ClipItem) {
    // 仅变更图标，顺序不变（下次进入页面才重排）
    try {
        await togglePin(item.id);
        item.pinned = !item.pinned;
        item.pin_ts_ms = item.pinned ? Date.now() : 0;
    } catch (err) {
        console.error('置顶失败:', err);
    }
}

async function onDelete(item: ClipItem) {
    try {
        await deleteItem(item.id);
        items.value = items.value.filter((it) => it.id !== item.id);
    } catch (err) {
        console.error('删除失败:', err);
    }
}

function imgMeta(item: ClipItem): string {
    const size = formatBytes(item.img_bytes);
    const dims = item.img_w && item.img_h ? `${item.img_w}×${item.img_h} · ` : '';
    return `${dims}${size}`;
}

onMounted(async () => {
    await refresh();
    await bindEvents();
});

onUnmounted(() => {
    unbindEvents();
    if (copiedTimer !== null) clearTimeout(copiedTimer);
    disarmClear();
});
</script>

<style scoped>
.clipboard-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.cm-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    min-height: 0;
}

.cm-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
}

.cm-page-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--card-h3-color);
}

.cm-count {
    color: var(--btn-pri-bg);
    font-size: 13px;
    margin-left: 4px;
}

.cm-subtitle {
    margin: -6px 0 0 0;
    font-size: 12px;
    color: var(--subtitle-color);
}

/* 关闭时的提示条 */
.cm-paused-bar {
    background: var(--btn-sec-bg);
    border: 1px solid var(--card-border);
    border-radius: 10px;
    padding: 9px 14px;
    font-size: 12.5px;
    color: var(--text-body);
}

/* 二级设置行（灵动岛提示） */
.cm-setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 12px;
}

.cm-setting-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.cm-setting-title {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--item-title-color);
}

.cm-setting-desc {
    font-size: 12px;
    color: var(--item-desc-color);
}

/* 工具栏：搜索 / 清空 / 存储限制 */
.cm-toolbar-row {
    display: flex;
    gap: 8px;
    align-items: center;
}

.cm-search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
}

.cm-search-icon {
    position: absolute;
    left: 10px;
    width: 14px;
    height: 14px;
    color: var(--subtitle-color);
    pointer-events: none;
}

.cm-search-input {
    width: 100%;
    background: var(--control-bg);
    border: 1px solid var(--control-border);
    border-radius: 10px;
    padding: 8px 30px 8px 30px;
    font-size: 13px;
    color: var(--text-body);
    outline: none;
    transition: border-color 0.15s ease;
}

.cm-search-input:focus {
    border-color: var(--btn-pri-bg);
}

.cm-search-clear {
    position: absolute;
    right: 9px;
    width: 13px;
    height: 13px;
    color: var(--subtitle-color);
    cursor: pointer;
    transition: color 0.15s ease;
}

.cm-search-clear:hover {
    color: var(--text-body);
}

.cm-btn {
    border: 1px solid var(--card-border);
    background: var(--control-bg);
    color: var(--text-body);
    border-radius: 10px;
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
}

.cm-btn:hover {
    opacity: 0.85;
}

/* 清空全部确认态：红色背景强调风险 */
.cm-btn.cm-btn-danger {
    background: #ef4444;
    border-color: #ef4444;
    color: #ffffff;
}

.cm-quota-wrap {
    position: relative;
}

.cm-quota-pop {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    min-width: 210px;
    background: var(--modal-bg);
    border: 1px solid var(--card-border);
    border-radius: 10px;
    box-shadow: 0 8px 24px var(--card-shadow-hover);
    padding: 10px 12px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.cm-quota-line {
    font-size: 12.5px;
    color: var(--text-body);
    white-space: nowrap;
}

/* 条目卡片列表 */
.cm-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: auto;
    flex: 1;
    min-height: 0;
    padding: 2px;
}

.cm-empty {
    padding: 28px 16px;
    text-align: center;
    color: var(--subtitle-color);
    font-size: 13px;
    line-height: 1.8;
}

.cm-card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 12px;
    box-shadow: 0 2px 8px var(--card-shadow);
    overflow: hidden;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.cm-card:hover {
    border-color: var(--slider-checked-bg);
    box-shadow: 0 4px 14px var(--card-shadow-hover);
}

/* 头部：类型 + 置顶星 + 时间/操作叠加区（右侧固定宽度，切换零抖动） */
.cm-card-head {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 9px 12px 0 12px;
}

.cm-kind {
    font-size: 11px;
    font-weight: 700;
    color: var(--subtitle-color);
    background: var(--btn-sec-bg);
    border-radius: 5px;
    padding: 1px 7px;
}

/* 已置顶条目：类型旁常驻实心 ★ */
.cm-pin-badge {
    font-size: 11px;
    color: #f59e0b;
    line-height: 1;
}

.cm-head-right {
    margin-left: auto;
    position: relative;
    width: 52px;
    height: 20px;
}

/* 时间与图标绝对定位叠加 + opacity 过渡，避免内容替换抖动 */
.cm-time {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11.5px;
    color: var(--item-desc-color);
    white-space: nowrap;
    opacity: 1;
    transition: opacity 0.18s ease;
}

.cm-actions {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    gap: 2px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease;
}

.cm-card:hover .cm-time {
    opacity: 0;
}

.cm-card:hover .cm-actions {
    opacity: 1;
    pointer-events: auto;
}

.cm-icon-btn {
    background: transparent;
    border: none;
    color: var(--item-title-color);
    width: 24px;
    height: 20px;
    padding: 0;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    border-radius: 5px;
    transition: background 0.15s ease;
}

.cm-icon-btn:hover {
    background: var(--btn-sec-bg);
}

.cm-icon-del:hover {
    color: #ef4444;
}

/* 中间区域：点击复制 */
.cm-card-body {
    position: relative;
    margin: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    min-height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
}

.cm-text {
    margin: 0;
    width: 100%;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-body);
    white-space: pre-wrap;
    word-break: break-all;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

.cm-thumb {
    max-width: 128px;
    max-height: 128px;
    object-fit: contain;
    border-radius: 6px;
    background: var(--btn-sec-bg);
}

.cm-thumb-placeholder {
    font-size: 12.5px;
    color: var(--subtitle-color);
    padding: 14px 0;
}

/* 复制成功反馈：中间区域半透明覆盖，约 800ms 淡出 */
.cm-copied-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--card-bg);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.25s ease;
    font-size: 13px;
    font-weight: 700;
    color: #10b981;
}

.cm-copied-overlay.is-visible {
    opacity: 0.92;
}

/* 底栏：字符数 / 尺寸与体积 */
.cm-card-foot {
    padding: 0 12px 9px 12px;
    font-size: 11.5px;
    color: var(--item-desc-color);
}

.cm-hint {
    margin: 0;
    font-size: 12px;
    color: var(--subtitle-color);
}

/* 开关本体（scoped 下父组件样式不穿透，需在此保留） */
.switch {
    position: relative;
    display: inline-block;
    width: 48px;
    height: 28px;
    flex-shrink: 0;
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
</style>
