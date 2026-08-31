<template>
    <div class="lyrics-manager">
        <!-- ===== 一级界面：缓存列表 ===== -->
        <div v-if="view === 'list'" class="lm-page">
            <div class="lm-toolbar">
                <span class="lm-page-title">歌词缓存 <span class="lm-count">{{ entries.length }}</span></span>
                <button class="lm-btn lm-btn-primary" @click="importCurrentTrack">导入当前歌曲</button>
            </div>
            <div class="lm-card">
                <div class="lm-row lm-head">
                    <span>歌名</span>
                    <span>歌手</span>
                    <span class="lm-col-action"></span>
                </div>
                <div v-if="entries.length === 0" class="lm-empty">
                    暂无已缓存歌词，播放过且有网络歌词的歌曲会自动缓存，也可点击「导入当前歌曲」手动绑定
                </div>
                <div v-for="e in entries" :key="e.key" class="lm-row lm-entry" @dblclick="openEntry(e)">
                    <span class="lm-song" :title="e.song">{{ e.song }}</span>
                    <span class="lm-artist" :title="e.artist">{{ e.artist }}</span>
                    <span class="lm-col-action">
                        <button class="lm-delete" @click.stop="removeEntry(e)">删除</button>
                    </span>
                </div>
            </div>
            <p class="lm-hint">双击以配置歌曲的对应歌词</p>
        </div>

        <!-- ===== 二级界面：单首配置 ===== -->
        <div v-else class="lm-page">
            <div class="lm-toolbar">
                <button class="lm-btn lm-btn-plain" @click="goBack">← 返回列表</button>
                <span class="lm-page-title lm-editor-title">
                    {{ editingKey ? '编辑歌词绑定' : '新建歌词绑定' }}
                </span>
            </div>
            <div class="lm-search-row">
                <input v-model="editSong" class="lm-input" type="text" placeholder="歌名" @keydown.enter="search">
                <input v-model="editArtist" class="lm-input" type="text" placeholder="歌手" @keydown.enter="search">
                <button class="lm-btn lm-btn-primary" :disabled="searching" @click="search">
                    {{ searching ? '搜索中…' : '搜索' }}
                </button>
            </div>
            <div class="lm-columns">
                <div class="lm-card lm-candidates">
                    <div class="lm-cand-row lm-head">
                        <span>歌名</span><span>歌手</span><span>专辑</span><span>时长</span><span>来源</span>
                    </div>
                    <div v-if="candidates.length === 0" class="lm-empty lm-empty-compact">
                        {{ searching ? '正在搜索...' : (searched ? '未找到匹配的歌词' : '输入歌名与歌手以搜索') }}
                    </div>
                    <div v-for="(c, i) in candidates" :key="i" class="lm-cand-row lm-entry"
                        @dblclick="previewCandidate(c)">
                        <span :title="c.song">{{ c.song }}</span>
                        <span :title="c.artist">{{ c.artist }}</span>
                        <span :title="c.album">{{ c.album }}</span>
                        <span>{{ formatTime(c.duration_ms) }}</span>
                        <span class="lm-source">{{ SOURCE_LABELS[c.source] || c.source }}</span>
                    </div>
                </div>
                <div class="lm-card lm-preview-card">
                    <pre class="lm-lyric-pre">{{ previewContent || '双击左侧结果以载入，或使用「导入」使用本地.lrc' }}</pre>
                    <div class="lm-preview-actions">
                        <button class="lm-btn" @click="importFile">导入</button>
                        <button class="lm-btn lm-btn-primary" :disabled="saving" @click="save">
                            {{ saving ? '保存中…' : '保存' }}
                        </button>
                    </div>
                </div>
            </div>
            <input ref="fileInputRef" type="file" accept=".lrc,.txt" class="lm-file-input" @change="onFilePicked">
        </div>

        <!-- ===== 歌名不一致确认弹窗 ===== -->
        <Transition name="fade">
            <div v-if="confirmDialog.visible" class="lm-modal-overlay" @click.self="confirmDialog.visible = false">
                <div class="lm-modal">
                    <h4>{{ confirmDialog.title }}</h4>
                    <p class="lm-modal-body">{{ confirmDialog.message }}</p>
                    <div class="lm-modal-actions">
                        <button class="lm-btn" @click="confirmDialog.visible = false">取消</button>
                        <button class="lm-btn lm-btn-primary" @click="confirmDialog.onConfirm">确认使用</button>
                    </div>
                </div>
            </div>
        </Transition>

        <!-- ===== 空歌词保存确认弹窗 ===== -->
        <Transition name="fade">
            <div v-if="showEmptyLyricDialog" class="lm-modal-overlay" @click.self="showEmptyLyricDialog = false">
                <div class="lm-modal">
                    <h4>无歌词</h4>
                    <p class="lm-modal-body">歌词为空。仍要保存吗？</p>
                    <div class="lm-modal-actions">
                        <button class="lm-btn" @click="showEmptyLyricDialog = false">返回</button>
                        <button class="lm-btn lm-btn-primary" @click="confirmSaveEmptyLyric">保存</button>
                    </div>
                </div>
            </div>
        </Transition>

        <!-- ===== 未保存退出三键弹窗 ===== -->
        <Transition name="fade">
            <div v-if="showLeaveDialog" class="lm-modal-overlay">
                <div class="lm-modal">
                    <h4>尚未保存当前歌词。仍要退出吗？</h4>
                    <div class="lm-modal-actions lm-modal-actions-column">
                        <button class="lm-btn lm-btn-primary" @click="leaveSaveAndExit">保存并退出</button>
                        <button class="lm-btn" @click="leaveDiscard">不保存</button>
                        <button class="lm-btn lm-btn-plain" @click="leaveStay">返回</button>
                    </div>
                </div>
            </div>
        </Transition>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface LyricEntry {
    key: string;
    file: string;
    source: string;
    song: string;
    artist: string;
    duration_ms: number;
    saved_at: number;
}

interface LyricCandidate {
    source: string;
    id: string;
    song: string;
    artist: string;
    album: string;
    duration_ms: number;
}

interface CurrentTrack {
    song: string;
    artist: string;
    duration_ms: number;
    app_id: string;
}

const SOURCE_LABELS: Record<string, string> = { qq: 'QQ', netease: '网易', auto: '自动', user: '手动' };

// ===== 一级列表 =====
const view = ref<'list' | 'editor'>('list');
const entries = ref<LyricEntry[]>([]);

async function refreshEntries() {
    try {
        entries.value = await invoke<LyricEntry[]>('list_lyrics_cache');
    } catch (e) {
        console.error('读取歌词缓存失败:', e);
        entries.value = [];
    }
}
refreshEntries();

async function removeEntry(e: LyricEntry) {
    try {
        await invoke('delete_lyrics_entry', { key: e.key });
        await refreshEntries();
    } catch (err) {
        console.error('删除失败:', err);
    }
}

// ===== 二级编辑 =====
const editSong = ref('');
const editArtist = ref('');
// 绑定时长：来自条目或导入的当前歌曲，编辑界面不可改
const bindingDurationMs = ref(0);
// 当前条目 key（编辑已有条目时存在）
const editingKey = ref<string | null>(null);
// dirty 基准：进入编辑时的已存歌词内容
const initialContent = ref('');

const previewContent = ref('');
const candidates = ref<LyricCandidate[]>([]);
const searched = ref(false);
const searching = ref(false);
const saving = ref(false);

const fileInputRef = ref<HTMLInputElement | null>(null);

const isDirty = () => previewContent.value !== initialContent.value;

function formatTime(ms: number) {
    if (!ms || ms <= 0) return '--:--';
    const totalSec = Math.floor(ms / 1000);
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
}

/** 显示层规范化（仅用于弹窗比对歌名，不参与 key 生成——key 只在 Rust 侧生成） */
function normalizeDisplay(s: string) {
    return s.trim().toLowerCase().replace(/\s+/g, '');
}

async function openEntry(e: LyricEntry) {
    editSong.value = e.song;
    editArtist.value = e.artist;
    bindingDurationMs.value = e.duration_ms;
    editingKey.value = e.key;
    let saved = '';
    try {
        saved = await invoke<string | null>('get_lyrics_by_key', { key: e.key }) ?? '';
    } catch { /* 读不到视为空 */ }
    previewContent.value = saved;
    initialContent.value = saved;
    candidates.value = [];
    searched.value = false;
    view.value = 'editor';
}

/** 「导入当前歌曲」：SMTC 聚合，未播放时提示 */
async function importCurrentTrack() {
    try {
        const track = await invoke<CurrentTrack | null>('import_current_track');
        if (!track) {
            alertInfo('获取播放信息失败', '未检测到播放，请播放一首歌曲并点击以导入。');
            return;
        }
        editSong.value = track.song;
        editArtist.value = track.artist;
        bindingDurationMs.value = track.duration_ms;
        editingKey.value = null;
        previewContent.value = '';
        initialContent.value = '';
        candidates.value = [];
        searched.value = false;
        view.value = 'editor';
    } catch (err) {
        console.error('获取当前歌曲失败:', err);
        alertInfo('获取歌词失败', '获取正在播放歌曲时出现异常，获取失败。');
    }
}

async function search() {
    const song = editSong.value.trim();
    if (!song) return;
    searching.value = true;
    try {
        candidates.value = await invoke<LyricCandidate[]>('search_lyrics_candidates', {
            songName: song,
            artistName: editArtist.value.trim(),
        });
        searched.value = true;
    } catch (err) {
        console.error('搜索失败:', err);
        candidates.value = [];
        searched.value = true;
    } finally {
        searching.value = false;
    }
}

/** 双击候选行：拉完整 LRC → 原文预览；候选歌名与条目歌名（规范化）不一致才弹确认 */
async function previewCandidate(c: LyricCandidate) {
    let content = '';
    try {
        content = await invoke<string>('get_lyrics_by_candidate', { source: c.source, id: c.id });
    } catch (err) {
        alertInfo('获取歌词失败', typeof err === 'string' ? err : '获取歌词失败，请更换源并重试。');
        return;
    }
    if (normalizeDisplay(c.song) !== normalizeDisplay(editSong.value)) {
        confirmDialog.value = {
            visible: true,
            title: '歌名不一致',
            message: `即将保存的「${c.song}」歌词，歌名与正在编辑的歌曲「${editSong.value}」不匹配。仍要保存吗？`,
            onConfirm: () => {
                previewContent.value = content;
                confirmDialog.value.visible = false;
            },
        };
        return;
    }
    previewContent.value = content;
}

// ===== 导入本地 .lrc：0 信任防线 =====
const MAX_IMPORT_SIZE = 1024 * 1024; // 1MB
// 至少命中若干条 [mm:ss.xx] 时间轴才算有效 LRC
const MIN_TIMELINE_LINES = 3;
// 时间轴行正则：[mm:ss.xx] / [mm:ss.xxx] / [mm:ss]
const TIMELINE_RE = /\[\d{1,2}:\d{2}(?:\.\d{1,3})?\]/;

// 简体/繁体特征字（命中计分；GBK 天然覆盖 GB2312/GB18030）
const COMMON_SIMPLIFIED = '的一是了我不人在他有这上们来到时大地为子中你说生国年着就那和要她出也得里后自以会家可下而过天去能对小多然于心学么之都好看起发当没成只如事把还用第样道想作种开美总从无情己面最女但现前些所同日手又行意动方期它头经长儿回位分爱老因很给名法间斯知世什两次使身者被高已亲其进此话常与活正感';
const COMMON_TRADITIONAL = '的一是了我不人在他有這上們來到時大地為子中你說生國年著就那和要她出也得裡後自以會家可下而過天去能對小多然於心學麼之都好看起發當沒成只如事把還用第樣道想作種開美總從無情己面最女但現前些所同日手又行意動方期它頭經長兒回位分愛老因很給名法間斯知世什兩次使身者被高已親其進此話常與活正感';

interface DecodeAttempt {
    encoding: string;
    text: string;
    score: number;
    mojibakeRate: number;
}

/** 过滤控制字符：保留 \t \n \r，挡 NUL / ESC 转义序列 */
function filterControlChars(text: string): string {
    // eslint-disable-next-line no-control-regex
    return text.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g, '');
}

function scoreDecode(bytes: Uint8Array, encoding: string): DecodeAttempt {
    let text: string;
    try {
        text = new TextDecoder(encoding, { fatal: false }).decode(bytes);
    } catch {
        return { encoding, text: '', score: -1, mojibakeRate: 1 };
    }
    let replacement = 0;
    let score = 0;
    for (const ch of text) {
        const code = ch.codePointAt(0) ?? 0;
        if (code === 0xFFFD) replacement++;
    }
    if (encoding === 'gbk') {
        for (const ch of COMMON_SIMPLIFIED) if (text.includes(ch)) score++;
    } else if (encoding === 'big5') {
        for (const ch of COMMON_TRADITIONAL) if (text.includes(ch)) score++;
    } else if (encoding === 'shift-jis') {
        for (const ch of text) {
            const c = ch.codePointAt(0) ?? 0;
            if (c >= 0x3040 && c <= 0x30FF) score++;
        }
    } else if (encoding === 'euc-kr') {
        for (const ch of text) {
            const c = ch.codePointAt(0) ?? 0;
            if (c >= 0xAC00 && c <= 0xD7AF) score++;
        }
    }
    const mojibakeRate = text.length > 0 ? replacement / text.length : 1;
    return { encoding, text, score, mojibakeRate };
}

/**
 * 编码识别（冻结方案）：
 * 1. UTF-8 严格模式（利用多字节结构特征，ASCII/UTF-8 在此结束）
 * 2. 失败进 GBK / BIG5 / Shift-JIS / EUC-KR 四赛道并行解码 + 特征打分
 * 3. 置信门槛：胜者乱码率 <1% 且得分显著领先第二名，否则拒收
 */
function decodeImportedLrc(bytes: Uint8Array): string {
    // 1. UTF-8 严格模式
    try {
        return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch { /* 非 UTF-8，进四赛道 */ }

    const attempts = ['gbk', 'big5', 'shift-jis', 'euc-kr'].map((enc) => scoreDecode(bytes, enc));
    attempts.sort((a, b) => b.score - a.score);
    const best = attempts[0];
    const second = attempts[1];
    const clearWinner = !second || best.score >= second.score * 1.5 || best.score - second.score >= 3;
    if (best.score > 0 && best.mojibakeRate < 0.01 && clearWinner) {
        return best.text;
    }
    throw new Error('编码识别失败，请转换为UTF-8后重试。');
}

function importFile() {
    fileInputRef.value?.click();
}

async function onFilePicked(ev: Event) {
    const input = ev.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ''; // 允许重复选择同一文件
    if (!file) return;

    if (file.size > MAX_IMPORT_SIZE) {
        alertInfo('文件过大', `超过1MB上限（${formatSize(file.size)}），导入失败。`);
        return;
    }

    let text: string;
    try {
        const buffer = await file.arrayBuffer();
        text = decodeImportedLrc(new Uint8Array(buffer));
    } catch (err) {
        alertInfo('解码失败', err instanceof Error ? err.message : '解码时出现异常，请转换为UTF-8后重试。');
        return;
    }

    text = filterControlChars(text);

    // 时间轴校验：至少命中若干 [mm:ss.xx] 行才算有效 LRC
    const timelineLines = text.split(/\r?\n/).filter((line) => TIMELINE_RE.test(line)).length;
    if (timelineLines < MIN_TIMELINE_LINES) {
        alertInfo('时间轴过少', `时间轴行过少，至少需要有${MIN_TIMELINE_LINES}行时间轴行。`);
        return;
    }
    if (text.trim().length === 0) {
        alertInfo('空文件', '导入的文件为空。');
        return;
    }

    previewContent.value = text;
}

function formatSize(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
}

// ===== 保存 =====
/** 空歌词保存确认弹窗 */
const showEmptyLyricDialog = ref(false);

async function save() {
    return doSave();
}

async function doSave(): Promise<boolean> {
    const song = editSong.value.trim();
    if (!song) {
        alertInfo('未配置歌名', '请填写歌名后保存。');
        return false;
    }
    if (previewContent.value.trim().length === 0) {
        // 空歌词：弹确认弹窗
        showEmptyLyricDialog.value = true;
        return false;
    }
    return performSave();
}

/** 空歌词保存确认后退出回调（由 leaveSaveAndExit 注入） */
let emptyLyricLeaveCallback: (() => void) | null = null;

/** 点「保存」确认空歌词后真正执行保存 */
function confirmSaveEmptyLyric() {
    showEmptyLyricDialog.value = false;
    if (emptyLyricLeaveCallback) {
        // 保存并退出路径
        emptyLyricLeaveCallback();
        emptyLyricLeaveCallback = null;
    } else {
        // 普通保存路径
        void performSave();
    }
}

/** 实际执行保存逻辑 */
async function performSave(): Promise<boolean> {
    const song = editSong.value.trim();
    saving.value = true;
    try {
        const entry = await invoke<LyricEntry>('save_lyrics_binding', {
            song,
            artist: editArtist.value.trim(),
            durationMs: bindingDurationMs.value,
            content: previewContent.value,
        });
        // 只有保存成功才更新本地状态：initialContent 同步为已存内容（dirty 归零）
        initialContent.value = previewContent.value;
        editingKey.value = entry.key;
        await refreshEntries();
        return true;
    } catch (err) {
        console.error('保存失败:', err);
        alertInfo('保存失败', typeof err === 'string' ? err : '保存失败，请重启后再试，或上报此问题。');
        return false;
    } finally {
        saving.value = false;
    }
}

// ===== 返回 / 切页拦截（dirty：预览区内容与进入时已存歌词不一致） =====
const showLeaveDialog = ref(false);
let leaveResolver: ((allowed: boolean) => void) | null = null;

function goBack() {
    void confirmLeave();
}

/**
 * 离开编辑页守卫：
 * - 一级列表浏览 / 内容未变（纯浏览）→ 直接放行
 * - dirty → 三键弹窗（保存并退出 / 不保存 / 返回）
 * 供组件内返回与 MainPanel 下拉切页共用；直接关窗不拦视为丢弃。
 */
function confirmLeave(): Promise<boolean> {
    if (view.value !== 'editor') return Promise.resolve(true);
    if (!isDirty()) {
        view.value = 'list';
        return Promise.resolve(true);
    }
    showLeaveDialog.value = true;
    return new Promise((resolve) => {
        leaveResolver = resolve;
    });
}

async function leaveSaveAndExit() {
    const song = editSong.value.trim();
    if (!song) {
        alertInfo('未配置歌名', '请填写歌名后保存。');
        return;
    }
    if (previewContent.value.trim().length === 0) {
        // 空歌词：先弹确认弹窗，确认后再保存并退出
        showEmptyLyricDialog.value = true;
        // 临时替换确认处理器：确认后保存并退出
        const proceed = () => {
            showEmptyLyricDialog.value = false;
            performSave().then((ok) => {
                showLeaveDialog.value = false;
                if (ok) {
                    view.value = 'list';
                    leaveResolver?.(true);
                } else {
                    leaveResolver?.(false);
                }
            });
        };
        // 关闭未保存退出弹窗，由空歌词确认弹窗接管
        showLeaveDialog.value = false;
        showEmptyLyricDialog.value = true;
        // 存储 proceed 供确认按钮调用
        emptyLyricLeaveCallback = proceed;
        return;
    }
    const ok = await performSave();
    showLeaveDialog.value = false;
    if (ok) {
        view.value = 'list';
        leaveResolver?.(true);
    } else {
        // 保存失败：留在编辑页
        leaveResolver?.(false);
    }
}

function leaveDiscard() {
    showLeaveDialog.value = false;
    view.value = 'list';
    leaveResolver?.(true);
}

function leaveStay() {
    showLeaveDialog.value = false;
    leaveResolver?.(false);
}

defineExpose({ confirmLeave });

// ===== 简易提示弹窗 =====
const confirmDialog = ref<{
    visible: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
}>({ visible: false, title: '', message: '', onConfirm: () => {} });

function alertInfo(title: string, message: string) {
    confirmDialog.value = {
        visible: true,
        title,
        message,
        onConfirm: () => {
            confirmDialog.value.visible = false;
        },
    };
}
</script>

<style scoped>
.lyrics-manager {
    display: flex;
    flex-direction: column;
    gap: 14px;
    height: 100%;
}

.lm-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    min-height: 0;
}

.lm-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
}

.lm-page-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--card-h3-color);
}

.lm-count {
    color: var(--btn-pri-bg);
    font-size: 13px;
    margin-left: 4px;
}

.lm-card {
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    border-radius: 12px;
    box-shadow: 0 2px 8px var(--card-shadow);
    overflow: auto;
    flex: 1;
    min-height: 0;
}

/* 一级列表两列布局 */
.lm-row {
    display: grid;
    grid-template-columns: 1fr 1fr 72px;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid var(--divider-border);
    gap: 8px;
}

.lm-row:last-child {
    border-bottom: none;
}

.lm-head {
    font-size: 12px;
    font-weight: 700;
    color: var(--subtitle-color);
    background: var(--btn-sec-bg);
    position: sticky;
    top: 0;
    z-index: 1;
}

.lm-entry {
    cursor: pointer;
    transition: background 0.15s ease;
}

.lm-entry:hover {
    background: var(--btn-sec-bg);
}

.lm-song,
.lm-artist {
    font-size: 13px;
    color: var(--text-body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.lm-song {
    font-weight: 600;
}

.lm-col-action {
    text-align: right;
}

.lm-delete {
    background: transparent;
    border: 1px solid var(--card-border);
    color: #ef4444;
    border-radius: 8px;
    padding: 3px 10px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s ease;
}

.lm-delete:hover {
    background: rgba(239, 68, 68, 0.1);
}

.lm-empty {
    padding: 28px 16px;
    text-align: center;
    color: var(--subtitle-color);
    font-size: 13px;
    line-height: 1.8;
}

.lm-empty-compact {
    padding: 18px 12px;
}

.lm-hint {
    margin: 0;
    font-size: 12px;
    color: var(--subtitle-color);
}

/* 按钮 */
.lm-btn {
    border: 1px solid var(--card-border);
    background: var(--control-bg);
    color: var(--text-body);
    border-radius: 10px;
    padding: 7px 16px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease, transform 0.15s ease;
}

.lm-btn:hover {
    opacity: 0.85;
}

.lm-btn:active {
    transform: scale(0.97);
}

.lm-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
}

.lm-btn-primary {
    background: var(--btn-pri-bg);
    border-color: var(--btn-pri-bg);
    color: var(--btn-pri-color, #fff);
}

.lm-btn-plain {
    background: transparent;
}

/* 搜索行：两框 + 键，与下方两区总宽对齐 */
.lm-search-row {
    display: grid;
    grid-template-columns: 1fr 1fr 96px;
    gap: 10px;
    align-items: center;
}

.lm-input {
    background: var(--control-bg);
    border: 1px solid var(--control-border);
    border-radius: 10px;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--text-body);
    outline: none;
    transition: border-color 0.15s ease;
}

.lm-input:focus {
    border-color: var(--btn-pri-bg);
}

/* 二级双列：候选列表 + 歌词预览 */
.lm-columns {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: 12px;
    flex: 1;
    min-height: 0;
}

.lm-cand-row {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1fr 52px 44px;
    gap: 6px;
    align-items: center;
    padding: 8px 10px;
    border-bottom: 1px solid var(--divider-border);
    font-size: 12px;
    color: var(--text-body);
}

.lm-cand-row span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.lm-cand-row.lm-head {
    font-size: 11px;
    color: var(--subtitle-color);
}

.lm-source {
    font-weight: 700;
    color: var(--btn-pri-bg);
}

.lm-preview-card {
    display: flex;
    flex-direction: column;
    padding: 10px;
    overflow: hidden;
}

.lm-lyric-pre {
    flex: 1;
    overflow: auto;
    margin: 0;
    padding: 8px;
    font-family: 'Cascadia Mono', 'JetBrains Mono', Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.7;
    color: var(--text-body);
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
}

.lm-preview-actions {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--divider-border);
}

.lm-preview-actions .lm-btn {
    flex: 1;
}

.lm-file-input {
    display: none;
}

/* 弹窗（适配控制台圆角卡片 + 明暗主题变量） */
.lm-modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 300;
}

.lm-modal {
    background: var(--modal-bg);
    border: 1px solid var(--card-border);
    border-radius: 14px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2);
    padding: 20px;
    width: 340px;
    max-width: 88%;
}

.lm-modal h4 {
    margin: 0 0 10px;
    font-size: 15px;
    color: var(--card-h3-color);
}

.lm-modal-body {
    margin: 0 0 16px;
    font-size: 13px;
    line-height: 1.7;
    color: var(--text-body);
}

.lm-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.lm-modal-actions-column {
    flex-direction: column;
    gap: 8px;
}

.lm-modal-actions-column .lm-btn {
    width: 100%;
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
