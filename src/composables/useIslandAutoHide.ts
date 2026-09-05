/**
 * 灵动岛自动隐藏 / 自动折叠 / 全屏自动隐藏 composable。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - scheduleAutoHide：统一自动隐藏调度。前置守卫（开关 + 音乐控制器开启 + 无播放）+
 *     到期复检（鼠标不在岛上、岛可见），防止定时期间状态变化导致误隐藏
 *   - handleMouseLeave / handleMouseEnter：离开时启动自动折叠/隐藏定时器，进入时全部取消
 *   - 全屏自动隐藏：fullscreen-changed 事件监听（本 composable 自带挂载/卸载与 unlisten 清理）
 *   - isAutoHiding / isHidingForFullscreen：区分"自动/全屏隐藏"与"用户主动关闭"，
 *     供主组件 onLeave 动画决定是否向控制台同步 island-status-sync
 * isPlaying（useMusicSync 输出）与 collapseMusic（音乐域折叠）定义在主组件接入点之后，
 * 经访问器/包装函数晚绑定注入；本 composable 需在 useMusicSync 之前接入以提供 isMouseOver
 * 与 scheduleAutoHide。isPendingCollapse 属于音乐展开域，主组件以 ref 形式传入共享。
 */
import { ref, onMounted, onUnmounted, type Ref } from 'vue';
import { listen, emit } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getSettingRaw } from '../utils/settings';
import {
    NSD_AUTO_HIDE_DELAY, NSD_AUTO_HIDE_ENABLED,
    NSD_AUTO_COLLAPSE_DELAY, NSD_AUTO_COLLAPSE_ENABLED,
    NSD_AUTO_HIDE_FS,
} from '../constants/storageKeys';

export function useIslandAutoHide(deps: {
    isIslandVisible: Ref<boolean>;
    isMusicCtlEnabled: Ref<boolean>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    isPlaying: () => boolean;
    mouseNearEdge: Ref<'left' | 'right' | null>;
    isPendingCollapse: Ref<boolean>;
    collapseMusic: () => void;
}) {
    const {
        isIslandVisible, isMusicCtlEnabled, isMusicExpanded, isMusicExpanding,
        isPlaying, mouseNearEdge, isPendingCollapse, collapseMusic,
    } = deps;

    // 自动隐藏相关变量
    const isMouseOver = ref(false);
    let autoHideTimer: number | null = null;
    const autoHideDelay = ref(Number(getSettingRaw(NSD_AUTO_HIDE_DELAY) || '2000')); // 默认2秒
    const isAutoHideEnabled = ref(getSettingRaw(NSD_AUTO_HIDE_ENABLED) === 'true'); // 自动隐藏功能开关

    // 自动折叠相关变量（灵动岛展开后，鼠标离开自动折叠回小岛状态）
    let autoCollapseTimer: number | null = null;
    const autoCollapseDelay = ref(Number(getSettingRaw(NSD_AUTO_COLLAPSE_DELAY) || '2000')); // 默认2秒
    const isAutoCollapseEnabled = ref(getSettingRaw(NSD_AUTO_COLLAPSE_ENABLED) === 'true'); // 自动折叠功能开关

    // 全屏自动隐藏相关
    const isAutoHideFullscreen = ref(getSettingRaw(NSD_AUTO_HIDE_FS) === 'true');
    let wasVisibleBeforeFullscreen = false;
    const isHidingForFullscreen = ref(false);
    const isAutoHiding = ref(false); // 标记当前隐藏是否由自动隐藏触发（区别于用户主动关闭）

    // 统一的自动隐藏定时器管理函数
    // 自动隐藏仅在以下条件全部满足时触发：
    //   1. 自动隐藏开关已开启 (isAutoHideEnabled)
    //   2. 音乐控制器模式已打开 (isMusicCtlEnabled)
    //   3. 没有音乐在播放 (!isPlaying)
    // 其余任何情况（如临时 toast、通知弹出、鼠标离开但非音乐模式等）均不隐藏
    const scheduleAutoHide = (delay?: number) => {
        // 前置守卫：不满足条件时直接返回，不设定定时器
        if (!isAutoHideEnabled.value || !isMusicCtlEnabled.value || isPlaying()) {
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
                && !isPlaying()) {
                isAutoHiding.value = true;
                isIslandVisible.value = false;
            }
        }, delay ?? autoHideDelay.value);
    };

    // 鼠标离开灵动岛时：自动折叠或自动隐藏
    const handleMouseLeave = () => {
        // 清除鼠标边缘检测状态
        mouseNearEdge.value = null;
        isMouseOver.value = false;

        // 1. 自动折叠逻辑：当灵动岛展开时，鼠标离开后延迟折叠回小岛状态
        if (isAutoCollapseEnabled.value && (isMusicExpanded.value || isMusicExpanding.value)) {
            // 启动自动折叠定时器
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

    // 鼠标重新移入灵动岛时：立刻打断收缩企图
    const handleMouseEnter = () => {
        // 如果之前移出留下了收缩案底，但动画还没播完鼠标又回来了，直接取消这个案底
        isPendingCollapse.value = false;
        isMouseOver.value = true;

        // 取消自动隐藏定时器
        if (autoHideTimer) {
            clearTimeout(autoHideTimer);
            autoHideTimer = null;
        }

        // 取消自动折叠定时器
        if (autoCollapseTimer) {
            clearTimeout(autoCollapseTimer);
            autoCollapseTimer = null;
        }
    };

    // 监听 Rust 发来的全屏状态变化（全屏自动隐藏）
    let unlistenFullscreen: (() => void) | null = null;
    onMounted(async () => {
        unlistenFullscreen = await listen<boolean>('fullscreen-changed', async (event) => {
            const isFullscreen = event.payload;
            if (!isAutoHideFullscreen.value) return;
            if (isFullscreen) {
                if (isIslandVisible.value) {
                    wasVisibleBeforeFullscreen = true;
                    isHidingForFullscreen.value = true;
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
    });

    onUnmounted(() => {
        try { unlistenFullscreen?.(); } catch (_) { /* 窗口已销毁 */ }
        unlistenFullscreen = null;
        if (autoHideTimer) {
            clearTimeout(autoHideTimer);
            autoHideTimer = null;
        }
        if (autoCollapseTimer) {
            clearTimeout(autoCollapseTimer);
            autoCollapseTimer = null;
        }
    });

    return {
        isMouseOver,
        isAutoHideEnabled,
        autoHideDelay,
        isAutoCollapseEnabled,
        autoCollapseDelay,
        isAutoHideFullscreen,
        isAutoHiding,
        isHidingForFullscreen,
        scheduleAutoHide,
        handleMouseLeave,
        handleMouseEnter,
    };
}
