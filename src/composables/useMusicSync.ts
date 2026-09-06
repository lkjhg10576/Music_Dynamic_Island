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
import { NSD_TARGET_PLAYER } from '../constants/storageKeys';
import { getSettingRaw } from '../utils/settings';

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

    // ==================== 浏览器Pro 音乐/视频判定（移植自上游 2.4.5） ====================
    // 用户是否选择了 browserPro 媒体模式（与 SMTC 来源无关）：
    // 浏览器Pro = 用户主动选择 browserPro 平台 + 实际 SMTC 来源是浏览器，两者都满足才算数
    const isBrowserProMode = () => getSettingRaw(NSD_TARGET_PLAYER) === 'browserPro';
    // 浏览器Pro标签页判定的额外结果：'music'|'video'；null 表示不做额外判定（按歌词兜底）
    const browserContentOverride = ref<'music' | 'video' | null>(null);
    // 浏览器标题命中视频站后缀（如优酷 "-电视剧-高清完整正版视频在线观看-优酷"）时强制判定为视频模式
    const isBrowserVideoTitle = ref(false);

    // 浏览器视频站标题后缀列表：命中任一后缀即判定为浏览器视频模式，并统一删除该后缀。
    // 注意：判定要用清理前的原始标题（清理后后缀已被删掉，无法再判）
    const BROWSER_VIDEO_SUFFIX_RE = [
        /_[ _]*哔哩哔哩[ _]*bilibili\s*$/i,
        /-电视剧-高清完整正版视频在线观看-优酷\s*$/i,
        /-电影-高清完整正版视频在线观看-优酷\s*$/i,
        /-综艺-高清完整正版视频在线观看-优酷\s*$/i,
        /-最新热门短剧大全-免费短剧在线观看\s*$/i,
        /-动漫-高清完整正版视频在线观看-优酷\s*$/i,
        /-少儿-高清完整正版视频在线观看-优酷\s*$/i,
        /-纪录片-高清完整正版视频在线观看-优酷\s*$/i,
        /-体育-高清完整正版视频在线观看-优酷\s*$/i,
        /-文化-高清完整正版视频在线观看-优酷\s*$/i,
        /-游戏-高清完整正版视频在线观看-优酷\s*$/i,
        /-音乐-高清完整正版视频在线观看-优酷\s*$/i,
    ];

    // 统一后缀删除函数：去掉标题里的所有视频站后缀及残留分隔符
    const cleanSongTitle = (title: string) => {
        let s = title;
        for (const re of BROWSER_VIDEO_SUFFIX_RE) {
            s = s.replace(re, '');
        }
        return s.replace(/[_\- ]+$/, '').trim();
    };

    // 从浏览器窗口标题中识别音乐类标签页，返回 { song, artist }（artist 可能为空）：
    // ① "正在播放: 歌名 - 歌手"（网易云/QQ音乐等网页版）
    // ② "歌名MP3/FLAC免费下载-下载站"（音乐下载站，如"青花瓷MP3免费下载-音乐下载网"）
    // ③ "歌名 - 歌手 - 平台"（如"青花瓷 - 周杰伦 - 网易云音乐"）
    // ④ "歌名 - 平台"（如"青花瓷 - 网易云音乐"，artist 留空交给搜索兜底）
    // 真实窗口标题在歌名/歌手之后还带浏览器附加的尾巴（如" - 个人 - Microsoft Edge"、
    // " 和另外 N 个页面"、" 和另外 N 个标签页"），所以先统一清理浏览器后缀与多标签尾巴再匹配；
    // ① 取前缀后前两段为歌名/歌手；② 只取格式词前的歌名；
    // ③④ 靠标题末尾的平台词收尾来识别，分别取前两段/前一段为歌名/歌手。
    // 音乐关键词（统一数据源，统一小写）：既供 ③④ 的平台收尾匹配，也供 judgeBrowserMode ② 的标签页关键词判定复用
    const TAB_MUSIC_KEYWORDS = ['music', '音乐', 'spotify', '网易云', '云音乐', 'netease', 'qq音乐', 'qqmusic', '酷狗', 'kugou', '酷我', 'kuwo', '虾米', '咪咕', '汽水音乐', '5sing', 'apple music', 'itunes', 'youtube music', 'soundcloud', 'bandcamp', 'tidal', 'deezer', 'pandora', 'amazon music'];
    // ③④ 平台收尾判定正则（由 TAB_MUSIC_KEYWORDS 派生，大小写不敏感；词内空白用 \s* 容忍任意空格，如"Apple Music"/"AppleMusic"）
    const TAB_MUSIC_PLATFORM_RE = new RegExp(TAB_MUSIC_KEYWORDS.map(k => k.replace(/\s+/g, '\\s*')).join('|'), 'i');

    const parsePlayingTabTitle = (tabs: string[]): { song: string; artist: string } | null => {
        for (const raw of tabs) {
            // 去掉窗口标题尾部的浏览器后缀（如" - Microsoft Edge" / " - Google Chrome"）
            // 以及多标签尾巴（" 和另外 N 个页面/标签页" / "and N other tabs"），统一清理后再匹配各模式
            const s = raw.trim()
                .replace(/\s*[-－–]\s*(Microsoft Edge Canary|Microsoft Edge|Google Chrome|Edge|Chrome)\s*$/i, '')
                .replace(/\s*和另外\s*\d+\s*(?:个页面|个标签页)\s*$/g, '')
                .replace(/\s+and\s+\d+\s+other\s+tabs?\s*$/gi, '')
                .replace(/\s+/g, ' ') // 连续空白折叠为单个空格，避免"Apple  Music"这类多余空格导致平台词匹配不到
                .trim();
            // ① 正在播放: 歌名 - 歌手
            const m = s.match(/^(正在播放|Now Playing|Playing)\s*[:：]\s*(.+)$/);
            if (m) {
                const parts = m[2].trim().split(/\s*[-－–]\s*/).map(p => p.trim()).filter(Boolean);
                // 歌名/歌手是前两段；后面的" - 个人"等窗口尾巴直接忽略，
                // 歌手段可能被 Edge/Chrome 追加" 和另外 N 个页面 / 和另外 N 个标签页 / and N other tabs"尾巴，需要清掉
                if (parts.length >= 2) {
                    const artist = parts[1]
                        .replace(/\s*和另外\s*\d+\s*(?:个页面|个标签页)\s*$/g, '')
                        .replace(/\s+and\s+\d+\s+other\s+tabs?\s*$/gi, '')
                        .trim();
                    return { song: parts[0], artist };
                }
            }
            // ② 歌名MP3/FLAC免费下载-下载站（音乐下载站标题）
            // 必须带音频格式/品质词，避免"某某视频免费下载"这类视频站标题被误判为音乐
            const m2 = s.match(/^(.+?)(?:MP3|FLAC|WAV|APE|AAC|OGG|M4A|WMA|DSD|320\s?[Kk]|无损|高品质|高音质)\s*(?:免费)?下载/i);
            if (m2) {
                return { song: m2[1].trim(), artist: '' };
            }
            // ③ 歌名 - 歌手 - 平台（如"青花瓷 - 周杰伦 - 网易云音乐"）
            // 最后一段必须以平台词收尾才命中，避免把"xxx - 腾讯视频"等视频标题误判为音乐
            const m3 = s.match(/^(.+?)\s*[-－–]\s*(.+?)\s*[-－–]\s*(.+?)\s*$/);
            if (m3 && TAB_MUSIC_PLATFORM_RE.test(m3[3])) {
                return { song: m3[1].trim(), artist: m3[2].trim() };
            }
            // ④ 歌名 - 平台（如"青花瓷 - 网易云音乐"），artist 留空交给搜索兜底
            const m4 = s.match(/^(.+?)\s*[-－–]\s*(.+?)\s*$/);
            if (m4 && TAB_MUSIC_PLATFORM_RE.test(m4[2])) {
                return { song: m4[1].trim(), artist: '' };
            }
        }
        return null;
    };

    // 同步中枢：基于当前 reactive 状态立即得出音乐/视频结论。
    // 判定优先级（高→低）：视频站标题后缀 > 浏览器Pro标签页"正在播放"匹配 > 浏览器Pro标签页关键词 > 拉到歌词兜底
    const resolveBrowserMode = (): 'music' | 'video' => {
        if (isBrowserVideoTitle.value) return 'video';          // ① 标题命中视频站后缀 → 强制视频
        if (browserContentOverride.value) return browserContentOverride.value; // ② 浏览器Pro标签页判定
        return isBrowserMusic.value ? 'music' : 'video';        // ③ 兜底：拉到歌词/封面即音乐，否则视频
    };

    // 刷新浏览器Pro的标签页额外判定并改写展示用的歌名/歌手。
    // 仅 browserPro 模式 + 浏览器来源才读活动标签页；命中"正在播放/平台标题"模式时，
    // 用解析出的歌名/歌手直接替换 SMTC 原始值（如"正在播放: xxx"/"edge"），
    // 展示、封面、歌词搜索都走干净数据（上游经由后端 fetch_song_meta 二次修正，分支直接用解析值）。
    const judgeBrowserMode = async (song: string, artist: string): Promise<{ song: string; artist: string }> => {
        if (isBrowserProMode() && currentIsBrowser.value) {
            try {
                const tabs = await invoke<string[]>('get_active_browser_tabs');
                // ① 高优先级：SMTC 标题（作为"伪标签页"）+ 标签页标题命中正则 → 直接采用解析值并判定为音乐
                const playing = parsePlayingTabTitle(song ? [song, ...tabs] : tabs);
                if (playing) {
                    isBrowserMusic.value = true;
                    browserContentOverride.value = 'music';
                    return { song: playing.song || song, artist: playing.artist || artist };
                }
                // ② 关键词判定：标题与关键词都去掉空白后做子串匹配，容忍多余空格（如"Apple  Music"/"QQ 音乐"）
                const VideoKeywords = ['bilibili', '哔哩哔哩', 'qqlive', '腾讯视频', 'youku', '优酷', 'youtube', 'iqiyi', '爱奇艺', '芒果tv', 'tv', '芒果TV', '影视', 'Tv', 'TV', 'cctv', 'CCTV', '央视'];
                const lowerTabs = tabs.map(t => t.toLowerCase().replace(/\s+/g, ''));
                const isVideo = VideoKeywords.some(keyword => lowerTabs.some(t => t.includes(keyword)));
                const isMusic = TAB_MUSIC_KEYWORDS.some(keyword => lowerTabs.some(t => t.includes(keyword.replace(/\s+/g, ''))));
                isBrowserMusic.value = (!isVideo || isMusic) && isMusic !== isVideo;
                browserContentOverride.value = isBrowserMusic.value ? 'music' : 'video';
            } catch {
                browserContentOverride.value = null; // 标签页读取失败 → 不做额外判定，走歌词兜底
            }
        } else {
            browserContentOverride.value = null; // 非浏览器Pro：无标签页信号，交给歌词兜底
        }
        return { song, artist };
    };

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
            // 标题为空时浏览器判定状态不可信，一并复位
            isBrowserVideoTitle.value = false;
            browserContentOverride.value = null;
            // 播放中也要能弹出（旧实现只有 hide 分支，播放器已在播但标题未就绪时不显示）
            await applyIslandVisibility(playing);
            return;
        }

        // ===== ⓪ 标题清洗 + 浏览器Pro判定：改写展示用的歌名/歌手 =====
        // 优酷/B站等视频站后缀先删除（判定用原始标题，展示/搜索用干净标题）；
        // browserPro 模式下读活动标签页做"正在播放/平台标题"解析，命中时用解析值
        // 替换 SMTC 原始值（如"正在播放: xxx"/"edge"），首帧直接显示真实歌名/歌手
        const cleanedSong = cleanSongTitle(song);
        isBrowserVideoTitle.value = cleanedSong !== song;
        const judged = currentIsBrowser.value
            ? await judgeBrowserMode(cleanedSong, artist)
            : { song: cleanedSong, artist };

        // ===== ① 首帧：同步赋值 + 窗口显隐，零网络 I/O =====
        currentSongName.value = judged.song;
        currentArtistName.value = judged.artist || '未知歌手';
        isPlaying.value = playing;
        await applyIslandVisibility(playing);

        // ===== ② 后台：封面与歌词并行，不阻塞首帧 =====
        const newTrackInfo = judged.artist ? `${judged.song} - ${judged.artist}` : judged.song;

        if (currentTrackInfo.value !== newTrackInfo) {
            // 切歌：重置浏览器音乐判定与歌词状态，等封面/歌词结果再确认是音乐还是视频
            // （browserPro 标签页已判为音乐时上面已置 isBrowserMusic，此处复位只影响通用模式的歌词兜底链路）
            if (!(currentIsBrowser.value && browserContentOverride.value === 'music')) {
                isBrowserMusic.value = false;
            }
            deps.resetLyricState();
            currentTrackInfo.value = newTrackInfo;
            // 防止上一首歌的封面与沉浸模糊背景残留
            coverUrl.value = '';
            blurredCoverUrl.value = '';

            // 非视频类来源：发起网络歌词请求（浏览器源由封面/歌词就绪后的判定翻转触发）
            // 两者各自的过期校验（fetchVersion / lyricReqSeq）保证切歌时不串写
            void loadCoverForTrack(judged.song, judged.artist, appId, newTrackInfo, fetchVersion);
            void deps.fetchLyricsForCurrentTrack(judged.song, judged.artist);
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
        // 浏览器判定状态一并复位，避免残留到下一次会话
        isBrowserVideoTitle.value = false;
        browserContentOverride.value = null;
        isBrowserMusic.value = false;
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
        browserContentOverride,
        isBrowserVideoTitle,
        resolveBrowserMode,
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
