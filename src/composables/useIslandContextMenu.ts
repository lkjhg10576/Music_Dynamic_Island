/**
 * 灵动岛右键菜单 composable：打开设置 / 流光边框开关 / 重置位置 / 重置宽度 / 锁定位置 / 关闭。
 * 从 WidgetIsland.vue 拆出；isMenuOpen 供主组件 handleForceTopmost（blur 置顶）判断菜单弹出态，
 * 菜单弹出/收起期间跳过强制置顶。音乐/通知态展开期间禁止呼出菜单。
 */
import { ref, type Ref } from 'vue';
import { Menu, MenuItem } from '@tauri-apps/api/menu';
import { emit } from '@tauri-apps/api/event';
import { LogicalPosition } from '@tauri-apps/api/window';
import { setSettingRaw, removeSetting } from '../utils/settings';
import { NSD_GLOW_BORDER, NSD_POSITION_LOCKED, NSD_ISLAND_WIDTH } from '../constants/storageKeys';
import type { SysToastType } from './useNotifications';

export function useIslandContextMenu(deps: {
    isIslandVisible: Ref<boolean>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    isMsgActive: Ref<boolean>;
    displaySysToast: Ref<boolean>;
    isGlowBorderEnabled: Ref<boolean>;
    isPinnedToTaskbar: Ref<boolean>;
    isPositionLocked: Ref<boolean>;
    currentWidth: Ref<number>;
    showToast: (text: string, type?: SysToastType) => unknown;
    adjustWindowPosition: () => Promise<void>;
    saveIslandPosition: () => Promise<void>;
    saveIslandWidth: () => void;
    getBaseSize: () => { w: number; h: number };
    animateIslandSize: (targetWidth: number, targetHeight: number) => unknown;
}) {
    const {
        isIslandVisible, isMusicExpanded, isMusicExpanding, isMsgActive, displaySysToast,
        isGlowBorderEnabled, isPinnedToTaskbar, isPositionLocked, currentWidth,
        showToast, adjustWindowPosition, saveIslandPosition, saveIslandWidth,
        getBaseSize, animateIslandSize,
    } = deps;

    const isMenuOpen = ref(false);

    const handleRightClick = async (event: MouseEvent) => {
        event.preventDefault();
        event.stopPropagation(); // 阻止冒泡

        // 如果音乐灵动岛正在展开或已完全展开，强制禁止呼出右键菜单
        if (isMusicExpanded.value || isMusicExpanding.value || isMsgActive.value || displaySysToast.value) {
            return;
        }

        // 打开设置
        const openSettingsItem = await MenuItem.new({
            text: '打开设置',
            id: 'open_settings',
            action: async () => {
                await emit('open-settings-panel');
                showToast('已打开设置');
            }
        });

        // 切换流光边框
        const toggleGlowBorderItem = await MenuItem.new({
            text: isGlowBorderEnabled.value ? '关闭流光边框' : '开启流光边框',
            id: 'toggle_glow_border',
            enabled: true,
            action: () => {
                isGlowBorderEnabled.value = !isGlowBorderEnabled.value;
                setSettingRaw(NSD_GLOW_BORDER, String(isGlowBorderEnabled.value));
                showToast(isGlowBorderEnabled.value ? '已开启流光边框' : '已关闭流光边框');
            }
        });

        // 重置位置
        const resetPositionItem = await MenuItem.new({
            text: isPinnedToTaskbar.value ? '重置位置 (已锁定)' : '重置位置',
            id: 'reset_position',
            enabled: !isPinnedToTaskbar.value,
            action: async () => {
                try {
                    await adjustWindowPosition();
                    // 如果已锁定，重置后重新保存新位置
                    if (isPositionLocked.value) {
                        await saveIslandPosition();
                    }
                    showToast('已重置位置');
                } catch (error) {
                    console.error(error);
                }
            }
        });

        // 重置宽度
        const resetWidthItem = await MenuItem.new({
            text: '重置宽度',
            id: 'reset_width',
            enabled: !isPositionLocked.value,
            action: async () => {
                try {
                    // 删除保存的自定义宽度
                    removeSetting(NSD_ISLAND_WIDTH);
                    // 恢复到默认宽度
                    const { w, h } = getBaseSize();
                    currentWidth.value = w;
                    animateIslandSize(w, h);
                    showToast('已重置宽度');
                } catch (error) {
                    console.error(error);
                }
            }
        });

        // 锁定位置菜单项
        const toggleLockItem = await MenuItem.new({
            text: isPositionLocked.value ? '解锁 (当前已锁定)' : '锁定',
            id: 'toggle_lock',
            enabled: !isPinnedToTaskbar.value,
            action: async () => {
                isPositionLocked.value = !isPositionLocked.value;
                setSettingRaw(NSD_POSITION_LOCKED, String(isPositionLocked.value));
                // 锁定时保存当前位置和宽度，以便下次启动恢复
                if (isPositionLocked.value) {
                    await saveIslandPosition();
                    saveIslandWidth();
                }
                // 同步状态给设置面板
                await emit('position-lock-sync', { locked: isPositionLocked.value });
                // 根据状态触发 lock / unlock 专属通知
                showToast(
                    isPositionLocked.value ? '锁定位置成功' : '位置已解锁',
                    isPositionLocked.value ? 'lock' : 'unlock'
                );
            }
        });

        // 关闭灵动岛
        const closeItem = await MenuItem.new({
            text: '关闭',
            id: 'close',
            action: () => {
                isIslandVisible.value = false;
            }
        });

        // 使用客户端坐标转逻辑坐标（避免无边框裁剪带来的漂移）
        const position = new LogicalPosition(
            event.clientX,
            event.clientY
        );

        // 3. 创建菜单并按顺序追加进去
        const menu = await Menu.new();
        await menu.append(openSettingsItem);
        await menu.append(toggleGlowBorderItem);
        await menu.append(resetPositionItem);
        await menu.append(resetWidthItem);
        await menu.append(toggleLockItem);
        await menu.append(closeItem);

        // 4. 弹出菜单
        try {
            isMenuOpen.value = true; // 👈 弹出前，告诉系统菜单打开了
            await menu.popup(position);
        } catch (error) {
            console.error('菜单弹出失败:', error);
        } finally {
            isMenuOpen.value = false; // 👈 无论用户是点击了菜单，还是点空白处取消了，都会瞬间恢复置顶状态
        }
    };

    return {
        isMenuOpen,
        handleRightClick,
    };
}
