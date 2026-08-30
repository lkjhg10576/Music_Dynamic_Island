/**
 * 通知 composable：F11 应用消息通知队列 + 灵动岛系统 toast 队列与展示状态机 + 通知点击处理。
 * 从 WidgetIsland.vue 拆出，职责：
 *   - msgQueue：后端 notification-event 增量推送的应用消息逐条展示（事件监听仍在主组件 onMounted）
 *   - sysToast 队列：系统操作通知（音量/电池/锁屏等），音量可合并续期；
 *     与消息队列共用 toastWaitToken 互斥调度（消息优先级最高）
 *   - toast 岛宽自适应（calcSysToastWidth，为频谱/实时活动区预留宽度）与展示结束后的岛尺寸恢复
 *   - 通知点击：关闭通知并启动来源应用（launch_app_by_aumid）
 *   - sysmsg（系统动态感知）事件到灵动岛 toast 类型的映射（showSysmsgToast）
 * 跨域依赖（自动隐藏调度、岛尺寸动画、频谱/实时活动区宽度判定）通过 deps 注入；
 * sysmsg 总开关 isSysmsgEnabled、消息模式开关及后端事件监听留在主组件。
 */
import { ref, watch, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import defaultLogo from '../assets/logo.png';

// 消息通知条目（后端 notification-event 事件推送）
export interface ToastItem {
    id: number;
    app_name: string;
    title: string;
    body: string;
    aumid: string;
}

// 通知权限状态
export type AccessStatus = 'ok' | 'denied' | 'unavailable';

export type SysToastType = 'app' | 'sys' | 'volume' | 'battery-charge' | 'battery-low' | 'lock' | 'unlock' | 'notify-permission';

export function useNotifications(deps: {
    isIslandVisible: Ref<boolean>;
    isPinnedToTaskbar: Ref<boolean>;
    msgExpandedWidth: Ref<number>;
    isMusicExpanded: Ref<boolean>;
    isMusicExpanding: Ref<boolean>;
    // 尺寸动画域输出（toast 展开与结束恢复都要驱动岛尺寸）
    currentWidth: Ref<number>;
    restoreIslandWidth: () => number | null;
    animateIslandSize: (targetWidth: number, targetHeight: number) => Promise<void> | void;
    // 主组件晚绑定桥接（定义在接入点之后，仅在实际调用时求值）
    getBaseSize: () => { w: number; h: number };
    scheduleAutoHide: (delay?: number) => void;
    // toast 计算目标岛宽时为右侧指示区/实时活动小图标预留宽度
    showSpectrumIndicator: () => boolean;
    showRtChip: () => boolean;
}) {
    const {
        isIslandVisible, isPinnedToTaskbar, msgExpandedWidth, isMusicExpanded, isMusicExpanding,
        currentWidth, restoreIslandWidth, animateIslandSize,
        getBaseSize, scheduleAutoHide, showSpectrumIndicator, showRtChip,
    } = deps;

    const isMsgActive = ref(false);
    const msgTitle = ref('');
    const msgAppName = ref('');
    const msgBody = ref('');
    const msgAumid = ref('');

    // 消息通知队列（替代原 5s 轮询）：后端增量推送多条时逐条展示
    const msgQueue = ref<ToastItem[]>([]);
    let isProcessingMsg = false;

    // 系统操作通知专用变量
    // volume：可合并续期的音量 toast；其它类型走普通队列
    interface SysToastItem {
        text: string;
        type: SysToastType;
    }

    const displaySysToast = ref(false);
    const sysToastText = ref('');
    const sysToastType = ref<SysToastType>('app');
    const toastQueue = ref<SysToastItem[]>([]);
    let isProcessingToast = false;

    // 音量 toast 可续期显示：以「距最后一次音量变化的静默时间」决定何时关闭
    const TOAST_DWELL_MS = 2000;
    const TOAST_LEAVE_MS = 200;
    let toastDeadlineAt = 0;
    let toastWaitToken = 0;
    let toastWaitTimer: ReturnType<typeof setTimeout> | null = null;
    // 记录最近一次已应用的 toast 岛宽，避免连续音量更新反复触发同尺寸动画
    let lastToastIslandWidth: number | null = null;

    // F11 消息通知展示计时器（手动关闭通知时中断等待）
    let msgTimer: number | null = null;

    const currentMsgIcon = ref(defaultLogo);

    // 图标映射表
    const getAppIcon = (appName: string) => {
        const name = appName.toLowerCase();

        if (name.includes('qq')) {
            // 使用 new URL 让 Vite 知道你要引入这个资源
            return new URL('../assets/qq.png', import.meta.url).href;
        }
        if (name.includes('钉钉') || name.includes('dingtalk')) {
            return new URL('../assets/dingtalk.png', import.meta.url).href;
        }
        if (name.includes('mail') || name.includes('邮件')) {
            return new URL('../assets/mail.png', import.meta.url).href;
        }
        if (name.includes('wechat') || name.includes('微信')) {
            return new URL('../assets/wechat.png', import.meta.url).href;
        }

        return defaultLogo;
    };

    const clearToastWaitTimer = () => {
        if (toastWaitTimer !== null) {
            clearTimeout(toastWaitTimer);
            toastWaitTimer = null;
        }
    };

    /** 可取消的等待：token 变化或组件卸载后旧等待立即失效 */
    const waitUntilToastDeadline = (token: number): Promise<void> => {
        return new Promise((resolve) => {
            const tick = () => {
                if (token !== toastWaitToken) {
                    resolve();
                    return;
                }
                const remaining = toastDeadlineAt - Date.now();
                if (remaining <= 0) {
                    toastWaitTimer = null;
                    resolve();
                    return;
                }
                toastWaitTimer = setTimeout(tick, remaining);
            };
            clearToastWaitTimer();
            tick();
        });
    };

    const sleepMs = (ms: number, token: number): Promise<void> => {
        return new Promise((resolve) => {
            if (token !== toastWaitToken) {
                resolve();
                return;
            }
            toastWaitTimer = setTimeout(() => {
                toastWaitTimer = null;
                resolve();
            }, ms);
        });
    };

    /** 用 canvas 测量 toast 文本像素宽度 */
    const measureToastTextWidth = (text: string): number => {
        try {
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            if (ctx) {
                ctx.font = '600 12.5px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
                return Math.ceil(ctx.measureText(text).width);
            }
        } catch (_e) { /* ignore */ }
        // 回退：中文约 12.5px，英文约 7.5px
        let w = 0;
        for (const ch of text) {
            w += /[\u4e00-\u9fff]/.test(ch) ? 12.5 : 7.5;
        }
        return Math.ceil(w);
    };

    /**
     * 计算系统 toast 目标岛宽，确保长文与频谱/实时活动区共存时文本可完整展示。
     * 布局：padding 14×2 + 图标有效占位 + 文本 + 频谱/状态点预留 +（split 时）右侧实时活动 44px
     */
    const calcSysToastWidth = (text: string, type: SysToastType): number => {
        const textW = measureToastTextWidth(text);
        // 图标 translateX(-8px) 后有效占位约 22，文本 translateX(-2px)
        const iconOccupy = 22;
        const horizontalPadding = 28; // left 14 + right 14
        const textGap = 6;
        // 频谱 5×2px + gap + 余量 ≈ 42；无频谱时仍留状态点与少量余量
        // toast 显示期间可与频谱共存，必须为右侧指示区预留宽度，避免文字被裁/遮挡
        const rightIndicator = showSpectrumIndicator() ? 42 : 16;
        // toast 时 isSplitMode 会被强制为 false，但 rt-chip 仍可能绝对定位叠在右侧，
        // 因此按 showRtChip 预留实时活动区宽度，避免长文被小图标遮挡
        const rtChipExtra = showRtChip() ? 44 : 0;
        const raw = horizontalPadding + iconOccupy + textGap + textW + rightIndicator + rtChipExtra;

        // 音量文本短，给较窄下限；电源/电池长文本给更宽下限
        // 例：「已接入电源，当前电量 100%」约 13 字 ≈ 162px + 图标/边距/频谱 ≈ 280+
        const minW = type === 'volume'
            ? 210
            : (type === 'battery-charge' || type === 'battery-low' ? 300 : 240);
        const maxW = 420;
        return Math.max(minW, Math.min(maxW, raw));
    };

    const applySysToastIslandSize = (text: string, type: SysToastType) => {
        const targetWidth = calcSysToastWidth(text, type);
        if (lastToastIslandWidth !== null && Math.abs(lastToastIslandWidth - targetWidth) < 2) {
            return; // 尺寸几乎不变，跳过动画
        }
        lastToastIslandWidth = targetWidth;
        animateIslandSize(targetWidth, 42);
    };

    // 队列处理函数
    const processToastQueue = async () => {
        if (isProcessingToast || toastQueue.value.length === 0) return;

        // 优先级判断：如果当前正在显示消息通知(最高优先级)，则挂起等待
        if (isMsgActive.value) return;

        isProcessingToast = true;
        const nextToast = toastQueue.value.shift();

        if (nextToast) {
            const token = ++toastWaitToken;
            sysToastText.value = nextToast.text;
            sysToastType.value = nextToast.type;
            displaySysToast.value = true;
            toastDeadlineAt = Date.now() + (nextToast.type === 'notify-permission' ? 6000 : TOAST_DWELL_MS);
            applySysToastIslandSize(nextToast.text, nextToast.type);

            // 自动恢复显示：当有系统通知时，如果灵动岛被隐藏，则自动恢复显示
            if (!isIslandVisible.value) {
                getCurrentWindow().show();
                isIslandVisible.value = true;
            }

            // 可续期停留：连续音量变化会推后 toastDeadlineAt。
            // waitUntilToastDeadline 的 timer 回调会重读 deadline；此处 while 再兜住
            // 「await 返回瞬间又被续期」的竞态。
            while (token === toastWaitToken) {
                await waitUntilToastDeadline(token);
                if (token !== toastWaitToken) break;
                if (Date.now() >= toastDeadlineAt) break;
            }

            // token 失效说明有新一轮处理接管，或组件已卸载
            if (token !== toastWaitToken) {
                isProcessingToast = false;
                return;
            }

            displaySysToast.value = false;
            lastToastIslandWidth = null;
            // 等待离开动画播完 (约200ms) 再处理下一个
            await sleepMs(TOAST_LEAVE_MS, token);

            if (token !== toastWaitToken) {
                isProcessingToast = false;
                return;
            }

            // 离开动画期间若队列中已有更新后的 volume，下面 processToastQueue 会接上
            // 系统 toast 结束后，重新评估自动隐藏
            // 场景：音乐控制器开启 + 无音乐播放时，toast 弹出强制显示了灵动岛，
            // toast 结束后应恢复自动隐藏
            scheduleAutoHide();
        }

        isProcessingToast = false;
        processToastQueue(); // 递归检查是否还有下一个通知
    };

    // 消息通知队列处理（与 sysmsg 共用 toastWaitToken / sleepMs；二者互斥：消息优先级最高）
    const processMsgQueue = async () => {
        if (isProcessingMsg || msgQueue.value.length === 0) return;
        // 系统 toast 进行中：消息挂起等待（与 processToastQueue 对称）
        if (displaySysToast.value) return;
        // 当前已有消息显示：等待其结束（watch(isMsgActive) 会再触发）
        if (isMsgActive.value) return;

        isProcessingMsg = true;
        const item = msgQueue.value.shift();

        if (item) {
            const token = ++toastWaitToken;
            msgAumid.value = item.aumid;
            msgTitle.value = (item.title && item.title !== item.app_name) ? item.title : '新通知';
            msgAppName.value = item.app_name;
            msgBody.value = item.body || (item.title === item.app_name ? '收到一条新通知' : item.title);
            currentMsgIcon.value = getAppIcon(item.app_name);

            if (!isMsgActive.value) {
                isMsgActive.value = true;
                if (!isIslandVisible.value) {
                    getCurrentWindow().show();
                    isIslandVisible.value = true;
                }
                if (!isPinnedToTaskbar.value) {
                    animateIslandSize(msgExpandedWidth.value, 65);
                }
            }

            // 有积压时每条缩短到 3s；仅剩 1 条显示 5s（用户决策）
            const dwell = msgQueue.value.length > 0 ? 3000 : 5000;
            await sleepMs(dwell, token);

            if (token !== toastWaitToken) {
                isProcessingMsg = false;
                return; // 被新消息/系统 toast 打断
            }

            isMsgActive.value = false;
            const { h } = getBaseSize();
            const savedWidth = restoreIslandWidth();
            const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
            animateIslandSize(targetWidth, h);
            scheduleAutoHide();
        }

        isProcessingMsg = false;
        processMsgQueue(); // 递归处理下一条
    };

    // 监听系统通知显示状态：动态宽度 + 结束后恢复用户岛宽
    watch(displaySysToast, (newVal) => {
        if (newVal) {
            applySysToastIslandSize(sysToastText.value, sysToastType.value);
        } else {
            lastToastIslandWidth = null;
            // 通知消失时，恢复到当前状态该有的尺寸
            // （前提是没有被应用消息或音乐面板霸占）
            if (!isMsgActive.value && !isMusicExpanded.value && !isMusicExpanding.value) {
                const { h } = getBaseSize();
                const savedWidth = restoreIslandWidth();
                const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
                animateIslandSize(targetWidth, h);
                // 系统 toast 结束后唤醒可能排队的消息通知
                processMsgQueue();
            }
        }
    });

    // 连续音量更新时：文本变化后重新评估宽度（仅在宽度确实变化时动画）
    watch(sysToastText, (text) => {
        if (!displaySysToast.value) return;
        applySysToastIslandSize(text, sysToastType.value);
    });

    // 点击系统 toast：notify-permission 类型跳转到 Windows 通知设置
    const onSysToastClick = () => {
        if (sysToastType.value === 'notify-permission') {
            invoke('open_notification_settings').catch(() => {});
        }
    };

    // 暴露给外部调用的触发函数
    const showToast = (text: string, type: SysToastType = 'app') => {
        // 音量：合并到当前显示或队列中的唯一 volume 项，并续期显示截止时间
        // 表现：单次弹出后数字随实际调节实时更新，不反复进场/离场
        if (type === 'volume') {
            // 1) 正在显示音量：只更新数字 + 续期，不重播进/离场
            if (displaySysToast.value && sysToastType.value === 'volume') {
                sysToastText.value = text;
                toastDeadlineAt = Date.now() + TOAST_DWELL_MS;
                return;
            }
            // 2) 已在队列中：原地更新为最新音量，避免堆积
            const queuedIdx = toastQueue.value.findIndex((item) => item.type === 'volume');
            if (queuedIdx >= 0) {
                toastQueue.value[queuedIdx] = { text, type: 'volume' };
                processToastQueue();
                return;
            }
            // 3) 其余情况（含 leave 动画窗口）正常入队；leave 结束后会立即接上
            toastQueue.value.push({ text, type: 'volume' });
            processToastQueue();
            return;
        }

        toastQueue.value.push({ text, type });
        processToastQueue();
    };

    // 把后端结构化 sysmsg-event 映射成灵动岛通知类型
    const showSysmsgToast = (p: { kind: string; level: string; text: string }) => {
        let type: SysToastType = 'sys';
        if (p.kind === 'volume') type = 'volume';
        else if (p.kind === 'unlock') type = 'unlock';
        else if (p.kind === 'lock') type = 'lock';
        else if (p.kind === 'power') type = p.level === 'success' ? 'battery-charge' : 'sys';
        else if (p.kind === 'battery') type = p.level === 'warn' ? 'battery-low' : 'battery-charge';
        else type = 'sys'; // 网络 / 默认
        showToast(p.text, type);
    };

    // 监听消息通知状态：
    // - 消息出现时：若正在显示 volume toast，先中断并塞回队列头部，避免消息结束后音量提示丢失
    // - 消息消失时：立刻唤醒可能被挂起的操作通知队列
    watch(isMsgActive, (newVal) => {
        if (newVal) {
            if (displaySysToast.value && sysToastType.value === 'volume') {
                const volumeText = sysToastText.value;
                toastWaitToken++;
                clearToastWaitTimer();
                displaySysToast.value = false;
                lastToastIslandWidth = null;
                // 合并：若队列里已有 volume，更新为最新；否则插到队首
                const queuedIdx = toastQueue.value.findIndex((item) => item.type === 'volume');
                if (queuedIdx >= 0) {
                    toastQueue.value[queuedIdx] = { text: volumeText, type: 'volume' };
                } else {
                    toastQueue.value.unshift({ text: volumeText, type: 'volume' });
                }
                isProcessingToast = false;
            }
            return;
        }
        processToastQueue();
        processMsgQueue(); // 消息结束后接上下一条排队消息
    });

    // ===== F11 通知点击打开：关闭通知显示并启动来源应用 =====
    // 关闭消息通知，恢复灵动岛到通知弹出前的状态
    const dismissMsgNotification = () => {
        if (!isMsgActive.value) return;
        if (msgTimer) {
            clearTimeout(msgTimer);
            msgTimer = null;
        }
        isMsgActive.value = false;
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
        // 手动关闭通知后重新评估自动隐藏
        scheduleAutoHide();
    };

    // 点击灵动岛上的通知：立即返回通知弹出前状态，并打开来源应用
    const handleNotificationClick = async () => {
        const aumid = msgAumid.value;
        // 先关闭通知显示，恢复灵动岛状态
        dismissMsgNotification();
        // 再启动来源应用
        if (aumid) {
            try {
                await invoke('launch_app_by_aumid', { aumid });
            } catch (e) {
                console.error('打开来源应用失败:', e);
            }
        }
    };

    // 组件卸载清理（主组件 onUnmounted 调用）：
    // 使进行中的 toast 等待立即失效，避免卸载后继续改状态
    const cleanupNotifications = () => {
        toastWaitToken++;
        clearToastWaitTimer();
        toastQueue.value = [];
        isProcessingToast = false;
    };

    return {
        isMsgActive,
        msgTitle,
        msgAppName,
        msgBody,
        currentMsgIcon,
        msgQueue,
        displaySysToast,
        sysToastText,
        sysToastType,
        showToast,
        showSysmsgToast,
        onSysToastClick,
        handleNotificationClick,
        processMsgQueue,
        cleanupNotifications,
    };
}

export type Notifications = ReturnType<typeof useNotifications>;
