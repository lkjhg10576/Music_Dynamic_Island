/**
 * 网络歌词 composable：LRC 解析 + 同步显示时钟。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - 拉取网络歌词（Rust 侧 QQ→网易双引擎）并解析 LRC
 *   - 按 SMTC Timeline（get_music_timeline）+ 本地时钟精确调度歌词逐句推进
 *   - 暂停/恢复/seek/切歌时冻结、重启与重置歌词时钟和调度状态
 * 音乐域状态（播放态/歌曲信息/浏览器判定）通过 deps 只读注入；
 * 浏览器源拉到歌词判定为音乐时，通过 onBrowserMusicDetected 回调通知音乐域，
 * 歌词域不直接写音乐域状态。
 */
import { ref, computed, watch, type Ref, type ComputedRef } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface MusicTimelineResponse { position_ms: number; end_ms: number; can_seek: boolean; }

export function useLyrics(deps: {
    isPlaying: Ref<boolean>;
    displayMusic: Ref<boolean> | ComputedRef<boolean>;
    isVideoLikeSource: ComputedRef<boolean> | Ref<boolean>;
    currentIsBrowser: Ref<boolean>;
    currentSongName: Ref<string>;
    currentArtistName: Ref<string>;
    // 歌词延迟（秒）：正值表示歌词整体延后显示
    nsdLyricDelay: Ref<number>;
    // 展开态判定：展开态下进度条轮询已在拉 Timeline，歌词时钟避免重复 IPC
    isMusicExpanded: Ref<boolean>;
    // 浏览器源拉到歌词 → 判定为播放音乐（而非视频），由音乐域翻转 isBrowserMusic
    onBrowserMusicDetected: () => void;
}) {
    const {
        isPlaying, displayMusic, isVideoLikeSource, currentIsBrowser,
        currentSongName, currentArtistName, nsdLyricDelay, isMusicExpanded,
        onBrowserMusicDetected,
    } = deps;

    const parsedLyrics = ref<{ time: number; text: string }[]>([]);
    const currentLyricText = ref('');
    // 歌词防吞字队列：极快节奏的短句排队展示，避免渲染过渡打架
    const lyricQueue = ref<string[]>([]);
    let lastLyricChangeTime = 0;
    let currentMatchedIndex = -1;
    // 当前曲目是否已发起过歌词请求（浏览器源判定翻转后补拉时防重复）
    let lyricRequested = false;
    // 歌词请求序号：切歌时递增，丢弃过期请求的结果
    let lyricReqSeq = 0;

    // 展开态标题位：优先显示歌词，无歌词（纯音乐/视频类/未拉到）时回退歌名
    const expandedLyricText = computed(() => currentLyricText.value || currentSongName.value);

    // 简单的 LRC 解析器
    const parseLrc = (lrcStr: string) => {
        const lines = lrcStr.split('\n');
        const result: { time: number; text: string }[] = [];
        const timeReg = /\[(\d{2}):(\d{2})\.(\d{2,3})\]/;

        for (const line of lines) {
            const match = timeReg.exec(line);
            if (match) {
                const min = parseInt(match[1]);
                const sec = parseInt(match[2]);
                const msStr = match[3].length === 2 ? match[3] + '0' : match[3];
                const ms = parseInt(msStr);
                const time = min * 60000 + sec * 1000 + ms;
                const text = line.replace(timeReg, '').trim();

                // 过滤掉只有全角空格、零宽字符的"幽灵歌词"
                const realText = text.replace(/[\s\u200B-\u200D\uFEFF\u3000]/g, '');

                if (realText.length > 0 && !text.includes('纯音乐') && text !== 'lrc' && text !== '//') {
                    result.push({ time, text });
                }
            }
        }
        return result.sort((a, b) => a.time - b.time);
    };

    // 歌词同步时钟：1s 轮询 get_music_timeline 校准 + 本地时钟平滑推进
    const lyricTimelinePos = ref(0);
    const lyricTimelineEnd = ref(0);
    const lyricSyncedAt = ref(Date.now());
    let lyricTimelineTimer: number | null = null;
    // 按下一句 LRC 时间戳精确 setTimeout 调度（替代原 50ms 高频扫描）；暂停即清除
    let lyricScheduleTimer: number | null = null;
    // 队列消费定时器：每句歌词至少稳定停留 800ms，避免闪烁
    let lyricConsumeTimer: number | null = null;

    const fetchLyricTimeline = async () => {
        try {
            const res = await invoke<MusicTimelineResponse | null>('get_music_timeline');
            if (res && res.end_ms > 0) {
                lyricTimelinePos.value = res.position_ms;
                lyricTimelineEnd.value = res.end_ms;
                lyricSyncedAt.value = Date.now();
                return true;
            }
        } catch (_) { /* 进度不可用时保持上一状态，匹配器静默等待 */ }
        return false;
    };

    // 当前播放位置：播放中按本地时钟推进，暂停时冻结
    const currentLyricPosition = () => {
        const elapsed = isPlaying.value ? Math.max(0, Date.now() - lyricSyncedAt.value) : 0;
        return Math.min(Math.max(0, lyricTimelineEnd.value), lyricTimelinePos.value + elapsed);
    };

    // 消费队列：每句歌词至少稳定停留 800ms，避免闪烁
    const tryConsumeLyricQueue = () => {
        if (lyricConsumeTimer !== null) {
            clearTimeout(lyricConsumeTimer);
            lyricConsumeTimer = null;
        }
        if (lyricQueue.value.length === 0) return;
        const now = performance.now();
        const wait = Math.max(0, 800 - (now - lastLyricChangeTime));
        if (wait === 0) {
            const nextLyric = lyricQueue.value.shift();
            if (nextLyric && nextLyric !== currentLyricText.value) {
                currentLyricText.value = nextLyric;
                lastLyricChangeTime = now;
            }
            if (lyricQueue.value.length > 0) tryConsumeLyricQueue();
        } else {
            lyricConsumeTimer = window.setTimeout(tryConsumeLyricQueue, wait) as unknown as number;
        }
    };

    // 清除歌词调度（暂停 / 停止时调用）
    const clearLyricSchedule = () => {
        if (lyricScheduleTimer !== null) {
            clearTimeout(lyricScheduleTimer);
            lyricScheduleTimer = null;
        }
        if (lyricConsumeTimer !== null) {
            clearTimeout(lyricConsumeTimer);
            lyricConsumeTimer = null;
        }
    };

    // 调度下一句：按 LRC 时间戳精确 setTimeout；暂停即清除
    const scheduleNextLyric = () => {
        if (lyricScheduleTimer !== null) {
            clearTimeout(lyricScheduleTimer);
            lyricScheduleTimer = null;
        }
        if (!isPlaying.value) return; // 暂停即清除
        if (parsedLyrics.value.length === 0 || isVideoLikeSource.value) return;
        const end = lyricTimelineEnd.value;
        if (end <= 0) return; // 尚未拿到播放进度

        // 正值延迟 = 歌词延后显示；抢跑 150ms 抵消渲染链路延迟
        const target = currentLyricPosition() + 150 - nsdLyricDelay.value * 1000;
        const next = parsedLyrics.value.find(l => l.time > target);
        if (!next) return; // 全部唱完
        const delay = Math.max(0, next.time - target);
        lyricScheduleTimer = window.setTimeout(() => {
            lyricScheduleTimer = null;
            advanceLyric();
        }, delay) as unknown as number;
    };

    // 推进匹配：找到当前应显示的句子并入队，然后调度下一句
    const advanceLyric = () => {
        if (parsedLyrics.value.length === 0 || isVideoLikeSource.value) return;
        const end = lyricTimelineEnd.value;
        if (end <= 0) return; // 尚未拿到播放进度

        // 正值延迟 = 歌词延后显示；抢跑 150ms 抵消渲染链路延迟
        const target = currentLyricPosition() + 150 - nsdLyricDelay.value * 1000;
        let matchedIndex = -1;
        for (let i = 0; i < parsedLyrics.value.length; i++) {
            if (parsedLyrics.value[i].time <= target) matchedIndex = i;
            else break;
        }

        if (matchedIndex > currentMatchedIndex) {
            if (currentMatchedIndex === -1 || matchedIndex - currentMatchedIndex > 2) {
                // 首次匹配或大幅快进：直接跳到目标句
                lyricQueue.value = [parsedLyrics.value[matchedIndex].text];
            } else {
                // 正常连续推进：把期间极快节奏的短句全部推入队列排队
                for (let i = currentMatchedIndex + 1; i <= matchedIndex; i++) {
                    lyricQueue.value.push(parsedLyrics.value[i].text);
                }
            }
            currentMatchedIndex = matchedIndex;
        } else if (matchedIndex !== -1 && matchedIndex < currentMatchedIndex) {
            // 用户回退了进度
            lyricQueue.value = [parsedLyrics.value[matchedIndex].text];
            currentMatchedIndex = matchedIndex;
        }

        tryConsumeLyricQueue();
        scheduleNextLyric();
    };

    const startLyricClock = () => {
        stopLyricClock();
        fetchLyricTimeline().then(() => advanceLyric());
        lyricTimelineTimer = window.setInterval(() => {
            // 展开态已有进度条轮询在拉 timeline，避免重复 IPC
            if (!isMusicExpanded.value) {
                // 1s 校准后重排调度（处理 seek / 时钟漂移）
                fetchLyricTimeline().then(() => advanceLyric());
            }
        }, 1000);
    };

    const stopLyricClock = () => {
        if (lyricTimelineTimer) {
            clearInterval(lyricTimelineTimer);
            lyricTimelineTimer = null;
        }
        clearLyricSchedule();
    };

    // 暂停即清除调度，恢复播放立即推进并重新调度
    watch(isPlaying, (playing) => {
        if (playing) {
            // 恢复播放：重置本地时钟基准，避免暂停期间 elapsed 累积造成进度跳变
            lyricSyncedAt.value = Date.now();
            advanceLyric();
        } else {
            clearLyricSchedule();
        }
    });

    // 歌词可用时启动同步时钟，不可用/切换内容时停止
    watch([parsedLyrics, displayMusic, isVideoLikeSource], () => {
        if (displayMusic.value && parsedLyrics.value.length > 0 && !isVideoLikeSource.value) {
            startLyricClock();
        } else {
            stopLyricClock();
        }
    });

    // 暂停/恢复时冻结/重启本地时钟，避免暂停时长被计入播放进度
    watch(isPlaying, (now, prev) => {
        if (now === prev) return;
        if (prev && !now) {
            // 暂停：把已推进的进度固化到基准值
            const elapsed = Math.max(0, Date.now() - lyricSyncedAt.value);
            lyricTimelinePos.value = lyricTimelinePos.value + elapsed;
        }
        // 恢复：从现在重新起算
        lyricSyncedAt.value = Date.now();
    });

    // 拉取网络歌词（QQ→网易双引擎在 Rust 侧），失败/未命中静默回退显示歌名
    const fetchLyricsForCurrentTrack = async (song: string, artist: string) => {
        const mySeq = ++lyricReqSeq;
        lyricRequested = true;
        // 时长用于歌词三重校验；折叠态下进度条未轮询，这里单独取一次
        let durationMs = 0;
        try {
            const tl = await invoke<MusicTimelineResponse | null>('get_music_timeline');
            if (tl && tl.end_ms > 0) durationMs = tl.end_ms;
        } catch (_) { /* 拿不到时长就按 0 走纯名称匹配 */ }
        try {
            const lrc = await invoke<string>('fetch_netease_lyrics', { songName: song, artistName: artist, durationMs });
            if (mySeq !== lyricReqSeq) return; // 已切歌，丢弃过期结果
            if (lrc) {
                parsedLyrics.value = parseLrc(lrc);
                // 浏览器拉到歌词 → 判定为播放音乐（而非视频），回调通知音乐域翻转判定
                if (currentIsBrowser.value) onBrowserMusicDetected();
                currentMatchedIndex = -1;
                lyricQueue.value = [];
                lastLyricChangeTime = 0;
            }
        } catch (_) { /* 静默失败：回退显示歌名 */ }
    };

    // 重置歌词相关状态（切歌/停止播放/无标题时调用）
    const resetLyricState = () => {
        lyricReqSeq++; // 使在途请求失效
        lyricRequested = false;
        parsedLyrics.value = [];
        lyricQueue.value = [];
        currentMatchedIndex = -1;
        currentLyricText.value = '';
        lastLyricChangeTime = 0;
        lyricTimelinePos.value = 0;
        lyricTimelineEnd.value = 0;
        lyricSyncedAt.value = Date.now();
    };

    // 浏览器来源拿到封面判定为音乐后（视频类 → 音乐），补拉网络歌词
    watch(isVideoLikeSource, (now, prev) => {
        if (prev && !now && currentIsBrowser.value && !lyricRequested && parsedLyrics.value.length === 0) {
            const artist = currentArtistName.value === '未知歌手' ? '' : currentArtistName.value;
            if (currentSongName.value && currentSongName.value !== '未在播放歌曲') {
                fetchLyricsForCurrentTrack(currentSongName.value, artist);
            }
        }
    });

    return {
        parsedLyrics,
        currentLyricText,
        expandedLyricText,
        lyricTimelinePos,
        lyricTimelineEnd,
        lyricSyncedAt,
        fetchLyricsForCurrentTrack,
        resetLyricState,
        advanceLyric,
        stopLyricClock,
    };
}

export type Lyrics = ReturnType<typeof useLyrics>;
