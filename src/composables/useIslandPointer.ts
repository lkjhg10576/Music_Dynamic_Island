/**
 * 灵动岛指针交互 composable：宽度调整（左右手柄 + 边缘检测光标）+ 拖拽判定与路由。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - handleResizeStart / handleResizeMove / handleResizeEnd：左右拖宽手柄的窗口 resize
 *     （左侧拖动时同步平移窗口保持右侧固定；边界约束 minWidth ~ maxWidth）
 *   - isNearEdge / mouseNearEdge / canResize：边缘区域检测与可调整判定
 *   - handleMouseDown / handleMouseMove / handleMouseUp：按下-移动-抬起主流程，
 *     按锁定/展开/通知态路由到 resize 或窗口拖拽（岛模式 startDragging / 任务栏模式横向拖拽）
 * mouseDownX / mouseDownY 同时供主组件 expandMusic 做点击位移判定（超过 5px 视为拖拽非点击）。
 * 自定义横向拖拽的底层实现（startCustomHorizontalDrag / handleCustomDragEnd）在 useIslandAnimation
 * 内，经依赖注入；位置锁定 / 弹簧动画锁等状态 ref 由主组件传入。
 */
import { ref, computed, type Ref } from 'vue';
import { getCurrentWindow, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/window';

export function useIslandPointer(deps: {
    isPositionLocked: Ref<boolean>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    isMsgActive: Ref<boolean>;
    displaySysToast: Ref<boolean>;
    isSizeAnimating: Ref<boolean>;
    isPinnedToTaskbar: Ref<boolean>;
    currentWidth: Ref<number>;
    currentHeight: Ref<number>;
    minWidth: number;
    maxWidth: number;
    startCustomHorizontalDrag: (event: MouseEvent) => Promise<void> | void;
    handleCustomDragEnd: () => void;
    saveIslandWidth: () => void;
}) {
    const {
        isPositionLocked, isMusicExpanded, isMusicExpanding, isMsgActive, displaySysToast,
        isSizeAnimating, isPinnedToTaskbar, currentWidth, currentHeight,
        minWidth, maxWidth, startCustomHorizontalDrag, handleCustomDragEnd, saveIslandWidth,
    } = deps;

    const isResizing = ref(false);
    const resizeSide = ref<'left' | 'right' | null>(null);
    let resizeStartX = 0;
    let resizeStartWidth = 0;

    // 鼠标是否在边缘区域（用于光标样式）
    const mouseNearEdge = ref<'left' | 'right' | null>(null);

    // 按下坐标：宽度调整 / 窗口拖拽 / 点击展开判定共用（expandMusic 也读取，故导出）
    const mouseDownX = ref(0);
    const mouseDownY = ref(0);
    let isMouseDown = false;

    // 计算是否可以调整宽度
    const canResize = computed(() => {
        return !isPositionLocked.value && !isMusicExpanded.value && !isMusicExpanding.value && !isMsgActive.value && !displaySysToast.value;
    });

    const handleMouseDown = (event: MouseEvent) => {
        if ((event.target as HTMLElement).closest('.ctl-btn')) return;
        if ((event.target as HTMLElement).closest('.resize-handle')) return;

        // 检测是否在边缘区域，如果是则开始宽度调整
        if (!isPositionLocked.value && !isMusicExpanded.value && !isMusicExpanding.value && !isMsgActive.value && !displaySysToast.value) {
            if (isNearEdge(event, 'left')) {
                handleResizeStart(event, 'left');
                return;
            }
            if (isNearEdge(event, 'right')) {
                handleResizeStart(event, 'right');
                return;
            }
        }

        // 无论有没有锁定，都必须老老实实记录坐标，给后面的"点击展开"提供判断依据。
        mouseDownX.value = event.clientX;
        mouseDownY.value = event.clientY;
        isMouseDown = true;
    };

    // ===== 宽度调整相关函数 =====
    const handleResizeStart = (event: MouseEvent, side: 'left' | 'right') => {
        // 位置锁定时禁止调整
        if (isPositionLocked.value) return;

        // 音乐展开、消息通知等状态下禁止调整
        if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) return;

        event.preventDefault();
        event.stopPropagation();

        isResizing.value = true;
        resizeSide.value = side;
        resizeStartX = event.screenX;
        resizeStartWidth = currentWidth.value;

        document.addEventListener('mousemove', handleResizeMove);
        document.addEventListener('mouseup', handleResizeEnd);
    };

    const handleResizeMove = async (event: MouseEvent) => {
        if (!isResizing.value || !resizeSide.value) return;

        const scaleFactor = window.devicePixelRatio;
        const deltaXLogical = event.screenX - resizeStartX;

        let newWidth: number;
        if (resizeSide.value === 'right') {
            newWidth = resizeStartWidth + deltaXLogical;
        } else {
            newWidth = resizeStartWidth - deltaXLogical;
        }

        // 边界约束
        newWidth = Math.max(minWidth, Math.min(maxWidth, newWidth));

        // 更新灵动岛宽度
        try {
            const appWindow = getCurrentWindow();
            await appWindow.setSize(new PhysicalSize(Math.ceil(newWidth * scaleFactor), Math.ceil(currentHeight.value * scaleFactor)));

            // 如果是左侧调整，需要同时移动窗口位置以保持右侧固定
            if (resizeSide.value === 'left') {
                const pos = await appWindow.outerPosition();
                const widthDelta = (newWidth - currentWidth.value) * scaleFactor;
                await appWindow.setPosition(new PhysicalPosition(Math.round(pos.x + widthDelta), Math.round(pos.y)));
            }

            // 更新当前宽度
            currentWidth.value = newWidth;
        } catch (error) {
            console.error('调整宽度失败:', error);
        }
    };

    const handleResizeEnd = () => {
        isResizing.value = false;
        resizeSide.value = null;
        document.removeEventListener('mousemove', handleResizeMove);
        document.removeEventListener('mouseup', handleResizeEnd);
    };

    // 检测鼠标是否在灵动岛边缘（用于显示调整光标）
    const isNearEdge = (event: MouseEvent, side: 'left' | 'right'): boolean => {
        if (isPositionLocked.value) return false;

        const target = event.currentTarget as HTMLElement;
        if (!target) return false;

        const rect = target.getBoundingClientRect();
        const EDGE_THRESHOLD = 8; // 边缘检测阈值（像素）

        if (side === 'left') {
            return event.clientX - rect.left <= EDGE_THRESHOLD;
        } else {
            return rect.right - event.clientX <= EDGE_THRESHOLD;
        }
    };

    const handleMouseMove = async (event: MouseEvent) => {
        // 宽度调整模式
        if (isResizing.value) {
            await handleResizeMove(event);
            return;
        }

        // 检测鼠标是否在边缘区域（用于光标样式）
        if (canResize.value) {
            const target = event.currentTarget as HTMLElement;
            if (target) {
                const rect = target.getBoundingClientRect();
                const EDGE_THRESHOLD = 8;
                const leftDist = event.clientX - rect.left;
                const rightDist = rect.right - event.clientX;

                if (leftDist <= EDGE_THRESHOLD && leftDist >= 0) {
                    mouseNearEdge.value = 'left';
                } else if (rightDist <= EDGE_THRESHOLD && rightDist >= 0) {
                    mouseNearEdge.value = 'right';
                } else {
                    mouseNearEdge.value = null;
                }
            }
        } else {
            mouseNearEdge.value = null;
        }

        if (!isMouseDown) return;

        // 1. 全局动画锁：任何变形动画期间，绝对禁止拖拽
        if (isSizeAnimating.value) return;

        // 2. 状态锁：音乐展开、消息通知、系统提示期间，统统禁止拖拽。
        if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) {
            // 发现企图拖拽，立刻打断施法
            isMouseDown = false;
            return;
        }

        // 3. 位置已锁定时，禁止一切拖拽
        if (isPositionLocked.value) return;

        // 4. 任务栏模式 + 已解锁：仅允许横向拖拽（自定义实现，约束 Y 轴不变）
        if (isPinnedToTaskbar.value) {
            if (Math.abs(event.clientX - mouseDownX.value) > 5) {
                isMouseDown = false;
                await startCustomHorizontalDrag(event);
            }
            return;
        }

        // 5. 岛模式 + 已解锁：自由拖拽（原生 startDragging，X/Y 均可移动）
        if (Math.abs(event.clientX - mouseDownX.value) > 5 || Math.abs(event.clientY - mouseDownY.value) > 5) {
            isMouseDown = false;
            try {
                await getCurrentWindow().startDragging();
            } catch (error) {
                console.error('拖拽失败:', error);
            }
        }
    };

    const handleMouseUp = () => {
        // 宽度调整结束时保存宽度
        if (isResizing.value) {
            handleResizeEnd();
            saveIslandWidth();
            return;
        }
        isMouseDown = false;
        handleCustomDragEnd();
    };

    return {
        mouseNearEdge,
        mouseDownX,
        mouseDownY,
        handleMouseDown,
        handleMouseMove,
        handleMouseUp,
        handleResizeStart,
    };
}
