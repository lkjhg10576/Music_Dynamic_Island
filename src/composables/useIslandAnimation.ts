/**
 * 灵动岛尺寸动画 composable：宽高形变弹簧动画 + 尺寸镜像状态 + 宽度持久化 + 自定义横向拖拽。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - animateIslandSize：呼叫 Rust 弹簧动画（start_island_animation）；带全局动画锁
 *     isSizeAnimating（形变动画期间禁止拖拽，500ms 自动解锁防死锁）
 *   - currentWidth / currentHeight：DOM 实际宽高镜像（Rust "island-resize" 像素流监听仍在主组件 onMounted）
 *   - 宽度持久化：saveIslandWidth / restoreIslandWidth / getExpandTargetWidth（展开态最小宽度回调）
 *   - 任务栏模式自定义横向拖拽：文档级 mousemove/mouseup 监听，cleanupIslandAnimation 供主组件卸载清理
 * 宽度调整交互路由（handleResizeStart/Move/End、边缘检测光标、handleMouseDown/Move/Up）
 * 与音乐展开/消息通知守卫及点击展开判定交织，留在主组件；
 * 入场弹簧过渡（onEnter/onLeave，AE 公式转化）与隐藏域（isAutoHiding/全屏隐藏）耦合，同样留在主组件。
 */
import { ref, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, currentMonitor, PhysicalPosition } from '@tauri-apps/api/window';
import { getSettingRaw, setSettingRaw } from '../utils/settings';
import { NSD_ISLAND_WIDTH } from '../constants/storageKeys';

export function useIslandAnimation(deps: {
    // 是否固定在任务栏（作为 isPinned 参数传给 Rust 弹簧动画）
    isPinnedToTaskbar: Ref<boolean>;
}) {
    const { isPinnedToTaskbar } = deps;

    // 控制 DOM 真正的高宽变量（由 Rust "island-resize" 事件无缝同步）
    const currentWidth = ref(150);
    const currentHeight = ref(34);

    // 记录全局灵动岛是否正在执行形变动画（动画锁：任何变形动画期间，绝对禁止拖拽）
    const isSizeAnimating = ref(false);
    let sizeAnimTimer: number | null = null;

    // 灵动岛核心代码！（完美防漂移+防裁切+防打断抖动）
    const animateIslandSize = async (targetWidth: number, targetHeight: number) => {
        try {
            // 1. 触发形变前：立刻上锁
            isSizeAnimating.value = true;
            if (sizeAnimTimer) clearTimeout(sizeAnimTimer);

            // 2. 设定 500ms 后自动解锁（覆盖大多数弹簧动画的持续时间）。
            sizeAnimTimer = window.setTimeout(() => {
                isSizeAnimating.value = false;
            }, 500);

            const appWindow = getCurrentWindow();
            const realSize = await appWindow.innerSize();
            const scaleFactor = window.devicePixelRatio;

            const realStartW = realSize.width / scaleFactor;
            const realStartH = realSize.height / scaleFactor;

            await invoke('start_island_animation', {
                startWidth: realStartW,
                startHeight: realStartH,
                targetWidth: targetWidth,
                targetHeight: targetHeight,
                isPinned: isPinnedToTaskbar.value
            });
        } catch (err) {
            console.error('呼叫 Rust 动画失败:', err);
            // 如果调用失败，安全起见立刻解锁，防止死锁
            isSizeAnimating.value = false;
        }
    };

    // ===== 宽度持久化 =====
    const MIN_WIDTH = 100; // 最小宽度
    const MAX_WIDTH = 500; // 最大宽度

    // 保存用户自定义的宽度
    const saveIslandWidth = () => {
        setSettingRaw(NSD_ISLAND_WIDTH, String(currentWidth.value));
    };

    // 恢复用户自定义的宽度
    const restoreIslandWidth = () => {
        const saved = getSettingRaw(NSD_ISLAND_WIDTH);
        if (saved) {
            const width = parseInt(saved, 10);
            if (width >= MIN_WIDTH && width <= MAX_WIDTH) {
                return width;
            }
        }
        return null;
    };

    // 用户当前设定宽度：拖拽保存值优先，其次当前实际宽度。
    // 收起/恢复岛宽时都用它，避免用 getBaseSize().w（写死的默认基准）覆盖用户的设定
    const getUserIslandWidth = () => restoreIslandWidth() ?? currentWidth.value;

    // 展开实时活动的最小宽度：低于该值时 CPU/RAM 详情、关闭按钮等信息会被压缩，影响观感
    const MIN_EXPAND_WIDTH = 200;

    // 展开实时活动的目标宽度：用户宽度充足（> 200px）时保持当前设定宽度不变；
    // 过窄（≤ 200px）时临时回调到最小展开宽度，关闭后仍恢复用户原宽度。
    // 打印队列详情需要额外宽度，走自己的按需加宽逻辑，不在此列
    const getExpandTargetWidth = () => Math.max(getUserIslandWidth(), MIN_EXPAND_WIDTH);

    // ===== 自定义横向拖拽（任务栏模式下仅允许 X 轴移动）=====
    // ref 化：主组件 handleForceTopmost（blur 置顶）需要读取拖拽进行中标志
    const isCustomDragging = ref(false);
    let customDragStartScreenX = 0;
    let customDragStartWindowX = 0;
    let customDragStartWindowY = 0;
    let customDragMonitor: { position: { x: number; y: number }; size: { width: number; height: number } } | null = null;
    let customDragWindowWidth = 0;

    const startCustomHorizontalDrag = async (event: MouseEvent) => {
        try {
            const appWindow = getCurrentWindow();
            // 获取窗口当前物理坐标，作为拖拽起点
            const pos = await appWindow.outerPosition();
            customDragStartWindowX = pos.x;
            customDragStartWindowY = pos.y;
            customDragStartScreenX = event.screenX;

            // 获取显示器信息与窗口宽度，用于边界约束
            customDragMonitor = await currentMonitor();
            const size = await appWindow.innerSize();
            customDragWindowWidth = size.width;

            isCustomDragging.value = true;

            // 添加文档级监听器，确保鼠标移出灵动岛窗口后仍能持续追踪
            document.addEventListener('mousemove', handleCustomDragMove);
            document.addEventListener('mouseup', handleCustomDragEnd);
        } catch (e) {
            console.error('横向拖拽初始化失败', e);
        }
    };

    const handleCustomDragMove = async (event: MouseEvent) => {
        if (!isCustomDragging.value) return;

        const scaleFactor = window.devicePixelRatio;
        const deltaXLogical = event.screenX - customDragStartScreenX;
        const deltaXPhysical = deltaXLogical * scaleFactor;
        let newX = customDragStartWindowX + deltaXPhysical;

        // 边界约束：防止拖出屏幕左右边缘
        if (customDragMonitor) {
            const monitorLeft = customDragMonitor.position.x;
            const monitorRight = customDragMonitor.position.x + customDragMonitor.size.width;
            newX = Math.max(monitorLeft, Math.min(newX, monitorRight - customDragWindowWidth));
        }

        try {
            await getCurrentWindow().setPosition(
                new PhysicalPosition(Math.round(newX), Math.round(customDragStartWindowY))
            );
        } catch (e) {
            console.error('横向拖拽失败:', e);
        }
    };

    const handleCustomDragEnd = () => {
        if (!isCustomDragging.value) return;
        isCustomDragging.value = false;
        document.removeEventListener('mousemove', handleCustomDragMove);
        document.removeEventListener('mouseup', handleCustomDragEnd);
    };

    // 组件卸载时清理文档级拖拽监听（主组件 onUnmounted 调用），
    // 防止拖拽进行中卸载后监听器残留
    const cleanupIslandAnimation = () => {
        document.removeEventListener('mousemove', handleCustomDragMove);
        document.removeEventListener('mouseup', handleCustomDragEnd);
    };

    return {
        currentWidth,
        currentHeight,
        isSizeAnimating,
        animateIslandSize,
        MIN_WIDTH,
        MAX_WIDTH,
        saveIslandWidth,
        restoreIslandWidth,
        getExpandTargetWidth,
        isCustomDragging,
        startCustomHorizontalDrag,
        handleCustomDragEnd,
        cleanupIslandAnimation,
    };
}

export type IslandAnimation = ReturnType<typeof useIslandAnimation>;
