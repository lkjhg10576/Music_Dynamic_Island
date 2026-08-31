/**
 * 音乐同步 composable：SMTC 音乐信息状态 + 事件驱动应用逻辑 + 低频兜底轮询。
 * 从 WidgetIsland.vue 拆出；封面烘焙（bakeAndStoreBlur / fetchBrowserCover 的存储部分）
 * 与自动隐藏调度留在主组件，通过 deps 回调注入。
 *
 * 与 useLyrics 存在相互依赖（音乐同步要调 resetLyricState / fetchLyricsForCurrentTrack，
 * 歌词要读 isPlaying / 歌曲信息）：歌词函数通过晚绑定桥接（bindLyrics）解耦。
 */
import { ref, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function useMusicSync(deps: {
    displayMusic: Ref<boolean>;
    isIslandVisible: Ref<boolean>;
    isMouseOver: Ref<boolean>;
    getPlayerName: () => string;
    getConnectedAppName: (appId: string) => string;
    bakeAndStoreBlur: (trackInfo: string, url: string) => Promise<void> | void;
    scheduleAutoHide: (delay?: number) => void;
    // 晚绑定桥接：由主组件在 useLyrics() 之后调用 bindLyrics 注入
    resetLyricState: () => void;
    fetchLyricsForCurrentTrack: (song: string, artist: string) => Promise<void>;
}) {
    const isPlaying = ref(false);

    // ===== 封面状态（LRU 缓存 + 沉浸模糊背景） =====
    const coverUrl = ref('');
    const coverCache = new Map<string, string>();
    // 清理缓存/切歌时递增，使在途封面请求的结果失效
    let coverFetchVersion = 0;
    const blurredCoverUrl = ref('');
    const blurredCoverCache = new Map<string, string>();

    // ===== SMTC 来源状态 =====
    const currentAppIdStr = ref('');
    const currentIsBrowser = ref(false);
    const isBrowserMusic = ref(false);

    // ===== 双行文本状态 =====
    const currentSongName = ref('未在播放歌曲');
    const currentArtistName = ref(deps.getPlayerName());
    const currentTrackInfo = ref(`未在播放歌曲 - ${deps.getPlayerName()}`);

    /** 使所有在途封面请求失效（清理缓存时调用） */
    const bumpCoverFetchVersion = () => {
        coverFetchVersion++;
    };

    // 浏览器专用封面：只认 SMTC 本地封面，拿不到就保留应用 logo（绝不走网络兜底）
    const fetchBrowserCover = async (trackInfo: string) => {
        if (coverCache.has(trackInfo)) {
            const cached = coverCache.get(trackInfo)!;
            coverUrl.value = cached;
            const cachedBlur = blurredCoverCache.get(trackInfo);
            if (cachedBlur !== undefined) {
                blurredCoverUrl.value = cachedBlur;
            } else {
                await deps.bakeAndStoreBlur(trackInfo, cached);
            }
            isBrowserMusic.value = true;
            return;
        }
        try {
            const smtcCover = await invoke<string | null>('get_smtc_cover');
            if (currentTrackInfo.value !== trackInfo) return; // 期间已切歌，丢弃过期结果
            if (smtcCover) {
                isBrowserMusic.value = true;
                coverUrl.value = smtcCover;
                while (coverCache.size >= 50) {
                    const oldest = coverCache.keys().next().value;
                    if (oldest !== undefined) {
                        coverCache.delete(oldest);
                        blurredCoverCache.delete(oldest);
                    }
                }
                coverCache.set(trackInfo, smtcCover);
                await deps.bakeAndStoreBlur(trackInfo, smtcCover);
            }
        } catch (_) { /* 拿不到 SMTC 封面就保留应用 logo */ }
    };

    // 灵动岛显隐调度：播放中自动恢复显示，停止播放时延迟隐藏。
    // 必须在 isPlaying 赋值之后调用 —— scheduleAutoHide 内部有 !isPlaying 守卫，
    // 顺序颠倒会导致「正在播放却被判定为可隐藏」。
    const applyIslandVisibility = async (playing: boolean) => {
        // 音乐播放器模式：有音乐就显示，没音乐就隐藏
        if (!deps.displayMusic.value) return;

        if (playing && !deps.isIslandVisible.value) {
            // 有音乐播放且灵动岛被隐藏，自动恢复显示
            await getCurrentWindow().show();
            deps.isIslandVisible.value = true;
        } else if (!playing && deps.isIslandVisible.value && !deps.isMouseOver.value) {
            // 音乐停止播放且鼠标不在灵动岛上，延迟隐藏
            // scheduleAutoHide 内部会校验音乐控制器模式 + 自动隐藏开关
            deps.scheduleAutoHide();
        }
    };

    // 封面加载（含网络请求）。从 applyMusicInfo 抽出：调用方 fire-and-forget，不阻塞首帧。
    const loadCoverForTrack = async (
        song: string,
        artist: string,
        appId: string,
        newTrackInfo: string,
        fetchVersion: number
    ) => {
        if (currentIsBrowser.value) {
            // 浏览器只认 SMTC 本地封面，绝不走网络兜底（避免视频标题在网络搜图时串错封面）
            await fetchBrowserCover(newTrackInfo);
            return;
        }

        // B站 / PotPlayer 视频模式：不取封面，圆形封面回退应用 logo
        if (appId.includes('bilibili') || artist === 'potplayer') {
            coverUrl.value = '';
            return;
        }

        // 优先读取缓存（LRU：命中时刷新插入顺序，超限时淘汰最旧条目）
        if (coverCache.has(newTrackInfo)) {
            // 命中：先删再设，将该条目移到 Map 末尾（最新）
            const cached = coverCache.get(newTrackInfo)!;
            coverCache.delete(newTrackInfo);
            coverCache.set(newTrackInfo, cached);
            coverUrl.value = cached;
            const cachedBlur = blurredCoverCache.get(newTrackInfo);
            if (cachedBlur !== undefined) {
                blurredCoverCache.delete(newTrackInfo);
                blurredCoverCache.set(newTrackInfo, cachedBlur);
                blurredCoverUrl.value = cachedBlur;
            } else {
                await deps.bakeAndStoreBlur(newTrackInfo, cached);
            }
            return;
        }

        try {
            const realCoverUrl = await invoke<string>('get_random_cover_url', {
                songName: song,
                artistName: artist
            });
            // 清理缓存或切歌后，丢弃过期封面结果
            if (fetchVersion !== coverFetchVersion
                || currentTrackInfo.value !== newTrackInfo) {
                return;
            }
            coverUrl.value = realCoverUrl;
            // 写入缓存，超限逐条淘汰最旧条目（LRU）
            while (coverCache.size >= 50) {
                const oldest = coverCache.keys().next().value;
                if (oldest !== undefined) {
                    coverCache.delete(oldest);
                    blurredCoverCache.delete(oldest);
                }
            }
            coverCache.set(newTrackInfo, realCoverUrl);
            // 烘焙沉浸模式模糊封面（只烘焙一次并按曲目缓存）
            await deps.bakeAndStoreBlur(newTrackInfo, realCoverUrl);
        } catch (coverErr) {
            if (fetchVersion !== coverFetchVersion
                || currentTrackInfo.value !== newTrackInfo) {
                return;
            }
            console.error('所有封面源均获取失败', coverErr);
            // 使用本地图标或纯色背景，不要再用外部 URL 作为错误兜底
            coverUrl.value = '';
        }
    };

    // 应用音乐信息到 UI：供后端 music-info-changed 事件与低频兜底轮询共用
    // （封面/歌词/浏览器判定等完整切歌逻辑都在这里）
    //
    // ⚠️ 流程顺序至关重要：歌名 / 歌手 / 播放态 / 窗口显隐构成「首帧」，必须在任何网络 I/O 之前完成。
    // 旧实现把 isPlaying 赋值和 show() 排在封面（1~3s）与歌词（1~3s）两次串行网络请求之后，
    // 而启动时灵动岛已被自动隐藏，导致用户要等 2~6s 才看到内容。
    const applyMusicInfo = async (song: string, artist: string, playing: boolean, appId: string) => {
        // 捕获本次调用起始版本；清理缓存会递增版本，避免过期封面回写
        const fetchVersion = coverFetchVersion;

        // 记录当前 SMTC 来源应用包名，并判定是否为浏览器类应用（edge/chrome）
        currentAppIdStr.value = appId;
        currentIsBrowser.value = appId.includes('edge') || appId.includes('chrome');

        // SMTC 已连上应用但还没有有效标题：单行展示改为显示"已连接的应用名"（而不是"未在播放"）
        // （播放器刚启动时很常见：会话已建但标题还没发布）
        if (!song) {
            const connectedName = deps.getConnectedAppName(appId);
            currentSongName.value = connectedName;
            currentArtistName.value = '';
            if (currentTrackInfo.value !== connectedName) {
                deps.resetLyricState();
                currentTrackInfo.value = connectedName;
            }
            isPlaying.value = playing;
            coverUrl.value = '';
            blurredCoverUrl.value = '';
            // 播放中也要能弹出（旧实现只有 hide 分支，播放器已在播但标题未就绪时不显示）
            await applyIslandVisibility(playing);
            return;
        }

        // ===== ① 首帧：同步赋值 + 窗口显隐，零网络 I/O =====
        currentSongName.value = song;
        currentArtistName.value = artist || '未知歌手';
        isPlaying.value = playing;
        await applyIslandVisibility(playing);

        // ===== ② 后台：封面与歌词并行，不阻塞首帧 =====
        const newTrackInfo = artist ? `${song} - ${artist}` : song;

        if (currentTrackInfo.value !== newTrackInfo) {
            // 切歌：重置浏览器音乐判定与歌词状态，等封面/歌词结果再确认是音乐还是视频
            isBrowserMusic.value = false;
            deps.resetLyricState();
            currentTrackInfo.value = newTrackInfo;
            // 防止上一首歌的封面与沉浸模糊背景残留
            coverUrl.value = '';
            blurredCoverUrl.value = '';

            // 非视频类来源：发起网络歌词请求（浏览器源由封面/歌词就绪后的判定翻转触发）
            // 两者各自的过期校验（fetchVersion / lyricReqSeq）保证切歌时不串写
            void loadCoverForTrack(song, artist, appId, newTrackInfo, fetchVersion);
            void deps.fetchLyricsForCurrentTrack(song, artist);
        }
    };

    // 无可用音乐会话：清空展示
    const applyNoTrack = () => {
        currentTrackInfo.value = `未在播放歌曲 - ${deps.getPlayerName()}`;
        currentSongName.value = '未在播放歌曲';
        currentArtistName.value = deps.getPlayerName();
        isPlaying.value = false;
        coverUrl.value = ''; // 没歌时清空，显示默认的优美渐变色
        blurredCoverUrl.value = ''; // 同步清空沉浸背景，避免残留上一首歌的模糊封面
        deps.resetLyricState();

        // 音乐播放器模式：音乐停止时隐藏灵动岛
        if (deps.isIslandVisible.value && !deps.isMouseOver.value) {
            deps.scheduleAutoHide();
        }
    };

    // 低频兜底轮询：浏览器/视频类来源 SMTC 事件经常延迟或不发，兜底比对快照纠偏，不可省
    const syncMusicStatus = async () => {
        // 捕获本次调用起始版本；清理缓存会递增版本，避免过期封面回写
        const fetchVersion = coverFetchVersion;
        try {
            // 调用 Rust 提取媒体信息 [歌名, 歌手, 是否在播放, 来源应用包名]
            const res = await invoke<[string, string, boolean, string] | null>('fetch_netease_music_info');
            if (fetchVersion !== coverFetchVersion) return;

            if (res) {
                await applyMusicInfo(res[0], res[1], res[2], res[3]);
            } else {
                applyNoTrack();
            }
        } catch (err) {
            if (fetchVersion !== coverFetchVersion) return;
            console.error('音乐信息获取失败:', err);
        }
    };

    return {
        isPlaying,
        coverUrl,
        blurredCoverUrl,
        coverCache,
        blurredCoverCache,
        currentAppIdStr,
        currentIsBrowser,
        isBrowserMusic,
        currentSongName,
        currentArtistName,
        currentTrackInfo,
        bumpCoverFetchVersion,
        fetchBrowserCover,
        applyMusicInfo,
        applyNoTrack,
        syncMusicStatus,
    };
}

export type MusicSync = ReturnType<typeof useMusicSync>;
