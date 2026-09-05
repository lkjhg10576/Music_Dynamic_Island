<template>
    <div class="music-ctl-box" :class="{ 'expanded': isMusicExpanded }" @click="onRootClick"
        style="cursor: pointer;">
        <div class="music-top-row">
            <div class="album-cover" :class="{ 'is-playing': isPlaying }">
                <div class="cover-inner"
                    :style="displayCoverUrl ? { backgroundImage: `url(${displayCoverUrl})`, backgroundSize: 'cover' } : {}">
                </div>
            </div>
            <div class="music-info-mask-box" ref="maskBoxRef">
                <div class="music-info-text single-line" :class="{ 'fade-out': isMusicExpanded }"
                    style="position: relative; width: 100%; height: 100%;">
                    <transition name="lyric-fade">
                        <span class="lyric-render-text" :key="currentLyricText || collapsedTrackText">
                            <span class="scroll-inner" ref="textInnerRef"
                                :class="{ 'is-scrolling': scrollDist > 0 }"
                                :style="scrollDist > 0 ? { '--scroll-dist': scrollDist + 'px', '--scroll-duration': scrollDuration } : {}">
                                {{ currentLyricText || collapsedTrackText }}
                            </span>
                        </span>
                    </transition>
                </div>
                <div class="music-info-text double-line" :class="{ 'fade-in': isMusicExpanded }">
                    <div class="song-title" ref="expandedLyricBoxRef">
                        <transition name="lyric-fade">
                            <span class="lyric-render-text" :key="expandedLyricText">
                                <span class="scroll-inner" ref="expandedLyricRef"
                                    :class="{ 'is-scrolling': expandedLyricScrollDist > 0 }"
                                    :style="expandedLyricScrollDist > 0 ? { '--scroll-dist': expandedLyricScrollDist + 'px', '--scroll-duration': expandedLyricScrollDuration } : {}">
                                    {{ expandedLyricText }}
                                </span>
                            </span>
                        </transition>
                    </div>
                    <div class="song-artist" ref="expandedArtistBoxRef" v-show="!isVideoLikeSource">
                        <span class="scroll-inner" ref="expandedArtistInnerRef"
                            :class="{ 'is-scrolling': expandedArtistScrollDist > 0 }"
                            :style="expandedArtistScrollDist > 0 ? { '--scroll-dist': expandedArtistScrollDist + 'px', '--scroll-duration': expandedArtistScrollDuration } : {}">
                            {{ expandedSubText }}
                        </span>
                    </div>
                </div>
            </div>
        </div>
        <transition name="fade">
            <div class="music-controls" v-show="isMusicExpanded">
                <button class="ctl-btn" @click.stop="emit('prev')">
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
                    </svg>
                </button>
                <button class="ctl-btn play-btn" @click.stop="emit('toggle-play')">
                    <svg v-if="isPlaying" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                    </svg>
                    <svg v-else viewBox="0 0 24 24" fill="currentColor"
                        style="transform: translateX(1px);">
                        <path d="M8 5v14l11-7z" />
                    </svg>
                </button>
                <button class="ctl-btn" @click.stop="emit('next')">
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
                        :class="{ disabled: !canSeek }"
                        :aria-disabled="!canSeek"
                        @pointerdown.stop="emit('progress-pointerdown', $event)"
                        @pointermove="emit('progress-pointermove', $event)"
                        @pointerup="emit('progress-pointerup', $event)"
                        @pointercancel="emit('progress-pointercancel', $event)"
                        @lostpointercapture="emit('progress-pointercancel', $event)">
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
</template>

<script setup lang="ts">
/**
 * 音乐控制岛态（从 WidgetIsland.vue 拆出）：
 *   - 展示：封面 / 折叠态单行（歌词或歌名）/ 展开态双行（歌词 + 歌手）/ 播放控件 / 进度条
 *   - 文本溢出滚动测量（measureOverflowScroll + 两个 watch）整体随迁：依赖全部是本组件内部
 *     DOM ref 与 props，父组件不再持有这些 ref，行为保持零变化
 *   - 播放进度的时间轴状态（musicTimeline / fetchTimeline / seek）与歌词时钟桥接留在父组件；
 *     进度条 DOM 通过 defineExpose(progressBarRef) 暴露，指针事件原样回传父组件处理
 */
import { ref, watch, nextTick, onMounted } from 'vue';

const props = defineProps<{
    isMusicExpanded: boolean;
    isPlaying: boolean;
    displayCoverUrl: string;
    currentLyricText: string;
    collapsedTrackText: string;
    expandedLyricText: string;
    expandedSubText: string;
    isVideoLikeSource: boolean;
    displayMusic: boolean;
    currentTrackInfo: string;
    timelineStatus: 'idle' | 'loading' | 'available' | 'unavailable';
    progressPosition: number;
    progressEnd: number;
    canSeek: boolean;
    progressPercent: number;
    isDraggingProgress: boolean;
}>();

const emit = defineEmits<{
    (e: 'activate', event: MouseEvent): void;
    (e: 'prev'): void;
    (e: 'toggle-play'): void;
    (e: 'next'): void;
    (e: 'progress-pointerdown', event: PointerEvent): void;
    (e: 'progress-pointermove', event: PointerEvent): void;
    (e: 'progress-pointerup', event: PointerEvent): void;
    (e: 'progress-pointercancel', event: PointerEvent): void;
}>();

// 供父组件的拖动定位逻辑读取进度条几何信息（getBoundingClientRect）
const progressBarRef = ref<HTMLElement | null>(null);
defineExpose({ progressBarRef });

const onRootClick = (e: MouseEvent) => {
    emit('activate', e);
};

// 毫秒 → m:ss
const formatTime = (ms: number) => {
    if (ms < 0 || isNaN(ms)) ms = 0;
    const totalSec = Math.floor(ms / 1000);
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
};

// ===== 音乐滚动相关变量（从父组件迁入）=====
const maskBoxRef = ref<HTMLElement | null>(null);
const textInnerRef = ref<HTMLElement | null>(null);
const scrollDist = ref(0);
const scrollDuration = ref('0s');

// 展开态双行滚动：标题位（歌词）与副标题位（歌手 - 歌名）
const expandedLyricBoxRef = ref<HTMLElement | null>(null);
const expandedLyricRef = ref<HTMLElement | null>(null);
const expandedLyricScrollDist = ref(0);
const expandedLyricScrollDuration = ref('0s');
const expandedArtistBoxRef = ref<HTMLElement | null>(null);
const expandedArtistInnerRef = ref<HTMLElement | null>(null);
const expandedArtistScrollDist = ref(0);
const expandedArtistScrollDuration = ref('0s');

// 共用测量：文本超出容器时给出滚动距离与时长（按 30px/s 阅读速度，首尾停留融入总时长）
const measureOverflowScroll = (textEl: HTMLElement, boxEl: HTMLElement) => {
    // 使用 getBoundingClientRect() 获取无视父级限制的真实渲染宽度
    const textWidth = textEl.getBoundingClientRect().width;
    const containerWidth = boxEl.clientWidth;

    if (textWidth <= containerWidth) {
        return { dist: 0, duration: '0s' };
    }

    // Math.ceil() 强制取整，绝对不允许出现小数像素
    const dist = Math.ceil(textWidth - containerWidth + 20);

    // 按照 30px/s 的速度阅读，计算纯移动时间；按 60% 占比融入首尾停留，确保匀速
    const totalDuration = (dist / 30) / 0.6;

    return { dist, duration: `${Math.max(totalDuration, 4.5)}s` };
};

// 折叠态单行歌词滚动计算
const calculateScroll = () => {
    // 展开状态下不执行滚动
    if (props.isMusicExpanded) {
        scrollDist.value = 0;
        return;
    }
    if (!textInnerRef.value || !maskBoxRef.value) return;

    const result = measureOverflowScroll(textInnerRef.value, maskBoxRef.value);
    scrollDist.value = result.dist;
    scrollDuration.value = result.duration;
};

// 展开态双行滚动计算：歌词位 + "歌手 - 歌名"位
const calculateExpandedScrolls = () => {
    if (expandedLyricRef.value && expandedLyricBoxRef.value) {
        const lyric = measureOverflowScroll(expandedLyricRef.value, expandedLyricBoxRef.value);
        expandedLyricScrollDist.value = lyric.dist;
        expandedLyricScrollDuration.value = lyric.duration;
    }
    if (expandedArtistInnerRef.value && expandedArtistBoxRef.value) {
        const artist = measureOverflowScroll(expandedArtistInnerRef.value, expandedArtistBoxRef.value);
        expandedArtistScrollDist.value = artist.dist;
        expandedArtistScrollDuration.value = artist.duration;
    }
};

// 展开态文本/状态变化时重算双行滚动：先立即算一次让文字马上开始滚动，
// 等 500ms 弹簧展开动画彻底结束后再量一次，用稳定后的宽度修正滚动距离
watch([() => props.expandedLyricText, () => props.expandedSubText, () => props.isMusicExpanded, () => props.isVideoLikeSource], async () => {
    await nextTick();
    if (props.isMusicExpanded) {
        calculateExpandedScrolls();
        setTimeout(() => {
            if (props.isMusicExpanded) calculateExpandedScrolls();
        }, 500);
    } else {
        expandedLyricScrollDist.value = 0;
        expandedArtistScrollDist.value = 0;
    }
});

// 核心修复 2：监听数组必须带上 displayMusic，并在 nextTick 后加上微小延迟，防止 v-else-if 导致宽度拿到 0
watch([() => props.currentTrackInfo, () => props.currentLyricText, () => props.collapsedTrackText, () => props.displayMusic, () => props.isMusicExpanded], async () => {
    await nextTick();
    setTimeout(() => {
        if (props.displayMusic) {
            calculateScroll();
        } else {
            // 切到其他界面（比如网速）时，归零重置
            scrollDist.value = 0;
        }
    }, 100);
});

onMounted(() => {
    // 初始化时触发一次折叠态滚动计算（原父组件 onMounted 的 700ms 延迟测量）
    setTimeout(() => {
        calculateScroll();
    }, 700);
});
</script>

<style scoped>
/* 让两个盒子脱离彼此的影响，在同一个包裹层内完美的“重叠”放置 */
.music-ctl-box {
    position: absolute;
    /* 改为绝对定位，实现无缝平移 */
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

.music-ctl-box {
    justify-content: flex-start;
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
    /* 默认让动画处于暂停状态 */
}

/* 正在播放时的旋转动画 */
.is-playing .cover-inner {
    animation-play-state: running;
    /* 当有播放状态时，让动画跑起来 */
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

/* 只有在 hover 的时候才出现背景色 */
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

/* 播放键稍微比切歌键大一点点，突出视觉中心 */
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

/* 歌曲信息遮罩容器：挨着封面靠左，占据右侧剩余空间 */
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
    /* 强制单行不换行 */
    overflow: hidden;
    color: inherit;
    opacity: 0.9;
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

/* 顶部容器：取消 all 过渡，让它跟着 Rust 窗口的拉伸严丝合缝地重排 */
.music-top-row {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    position: relative;
    transition: none !important;
    /* 核心防抖魔法，取消 CSS 的挣扎 */
}

.music-ctl-box.expanded .music-top-row {
    height: 40px;
    margin-top: 14px !important;
    margin-left: 5px !important;
    border: none;
}

/* 封面：覆盖掉上面的 transition: all，只保留变形和圆角的过渡 */
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

/* 歌曲文本遮罩：取消过渡，随窗口大小瞬间变化 */
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
    position: relative;
    /* 固定高度 = 15px × 1.2 行高：内部歌词层是绝对定位，需要确定的盒子高度 */
    height: 18px;
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 2px;
    white-space: nowrap;
    overflow: hidden;
    line-height: 1.2;
    width: 100%;
    text-align: left !important;
}

.song-artist {
    font-size: 12.5px;
    opacity: 0.65;
    white-space: nowrap;
    overflow: hidden;
    line-height: 1.2;
    width: 100%;
    text-align: left !important;
}

/* 媒体控件与频谱 */
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

/* F6 音乐进度条 */
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

/* 强制靠左对齐，干掉原本的 align-items: center。否则长文本会向两边溢出，导致开头被裁 */
.music-info-text.single-line {
    overflow: visible !important;
    align-items: flex-start !important;
    text-align: left !important;
}

/* 歌词渲染单句定位：绝对定位叠层，换句时新旧两层原位重叠，才能做交叉叠化 */
.lyric-render-text {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    white-space: nowrap;
    overflow: hidden;
    text-align: left !important;
    display: inline-block;
    will-change: opacity, filter;
}

/* 歌词换句过渡：新句从模糊透明渐入，旧句同步在原地模糊淡出（总时长约 220ms） */
.lyric-fade-enter-active,
.lyric-fade-leave-active {
    transition: opacity 0.2s ease, filter 0.22s ease;
}

.lyric-fade-enter-from {
    opacity: 0;
    filter: blur(8px);
}

.lyric-fade-enter-to {
    opacity: 1;
    filter: blur(0px);
}

.lyric-fade-leave-from {
    opacity: 1;
    filter: blur(0px);
}

.lyric-fade-leave-to {
    opacity: 0;
    filter: blur(8px);
}

/* 滚动的内部容器 */
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

/* 滚动动画帧：利用 0-20% 和 80-100% 的区间实现两端停留 */
@keyframes scroll-ping-pong {

    0%,
    20% {
        transform: translateX(0);
    }

    80%,
    100% {
        /* JS 里已经拼好了 px 单位，这里直接 -1 乘过去即可 */
        transform: translateX(calc(-1 * var(--scroll-dist)));
    }
}
</style>
