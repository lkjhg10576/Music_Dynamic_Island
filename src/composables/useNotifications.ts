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
import { getChengyuByCode, resolveWeatherIconKey, weatherCodeToIcon } from '../utils/weather';
import { measureTextWidth, FONT_TOAST_TITLE, FONT_MSG_TITLE, FONT_MSG_BODY } from '../utils/textMeasure';
import defaultLogo from '../assets/logo.png';

// 消息通知条目（后端 notification-event 事件推送）
export interface ToastItem {
    id: number;
    app_name: string;
    title: string;
    body: string;
    aumid: string;
    /** 来源应用 logo（后端解码为 data URI）；null 表示后端未取到，回退白名单/默认图标 */
    icon?: string | null;
}

// 通知权限状态
export type AccessStatus = 'ok' | 'denied' | 'unavailable';

export type SysToastType = 'app' | 'sys' | 'volume' | 'battery-charge' | 'battery-low' | 'lock' | 'unlock' | 'notify-permission' | 'clipboard' | 'calendar' | 'weather' | 'weather-morning' | 'weather-noon' | 'weather-evening';

// ── toast 类型分组（岛宽计算与 IslandWeatherAlert 的排版/关闭按钮判定共用同一份口径）──

/** 天气类：恶劣天气 + 早/午/晚报（两行排版，图标与配色由 icon/severity 决定） */
export const WEATHER_TOAST_TYPES: ReadonlySet<SysToastType> = new Set<SysToastType>([
    'weather', 'weather-morning', 'weather-noon', 'weather-evening',
]);

/** 早/午/晚报档位：常驻停留（点右侧 X 关闭）+ 预留按钮位 */
export const BRIEF_TOAST_TYPES: ReadonlySet<SysToastType> = new Set<SysToastType>([
    'weather-morning', 'weather-noon', 'weather-evening',
]);

/** 两行排版（标题在上、正文在下）的 toast 类型：天气类 + 日程提醒 */
export const TWO_LINE_TOAST_TYPES: ReadonlySet<SysToastType> = new Set<SysToastType>([
    ...WEATHER_TOAST_TYPES, 'calendar',
]);

/** 到点提醒 toast 的停留时长：比常规 2s 长，长标题要看得完（不常驻，避免挡住岛） */
export const CALENDAR_REMINDER_DWELL_MS = 4000;

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
    // volume：可合并续期的音量 toast；clipboard：可合并续期的剪贴板 toast（高频，必须 noWake）；
    // 其它类型走普通队列
    interface SysToastItem {
        text: string;
        type: SysToastType;
        /** 高频操作（剪贴板复制）专用：不唤醒隐藏的岛，避免每次复制都弹出岛 */
        noWake?: boolean;
        // ── 天气类扩展（选填）：岛上传两行（标题 + 正文小字），并按 icon/severity 决定图标与配色 ──
        /** 标题行：速报为「问好 · 成语」，恶劣天气为事件标题 */
        title?: string;
        /** 正文行（小字）：天气详情，为空时回退单行渲染 */
        body?: string;
        /** 图标键：sun/moon/cloud/rain/snow/sleet/fog/haze/alert/temp */
        iconKey?: string;
        /** 严重程度：info/warn/danger（驱动恶劣天气预警配色） */
        severity?: string;
        /** 天气代码（速报成语取词用） */
        code?: number;
        /** weather-toast 的 kind：severe/morning/noon/evening */
        kind?: string;
        /**
         * 持久停留：显示后不自动隐藏，直到用户点击 X 关闭，或被下一条通知顶替。
         * 用于早/午/晚报（速报 2s 太短看不完，用户要求改为常驻直到主动关闭）。
         */
        persistent?: boolean;
    }

    const displaySysToast = ref(false);
    const sysToastText = ref('');
    const sysToastType = ref<SysToastType>('app');
    // 天气类 toast 的两行渲染与图标/配色数据（非天气类型为空）
    const sysToastTitle = ref('');
    const sysToastBody = ref('');
    const sysToastIcon = ref('');
    const sysToastSeverity = ref('');
    // 当前显示的 toast 是否为持久停留项（persistent 速报），供让位/阻塞判定
    const sysToastPersistent = ref(false);
    const toastQueue = ref<SysToastItem[]>([]);
    let isProcessingToast = false;

    // 音量 toast 可续期显示：以「距最后一次音量变化的静默时间」决定何时关闭
    const TOAST_DWELL_MS = 2000;
    const TOAST_LEAVE_MS = 200;
    let toastDeadlineAt = 0;
    let toastWaitToken = 0;
    let toastWaitTimer: ReturnType<typeof setTimeout> | null = null;
    // 当前 toast 等待的唤醒器：可续期截止等待（音量/普通 toast）与持久停留等待（速报）共用。
    // 任何"提前结束当前 toast"的路径（点 X / 新通知让位 / 消息插队 / 组件卸载）都只唤醒它，
    // 隐藏 + 恢复尺寸 + 处理下一条一律交回 processToastQueue 统一收尾 ——
    // 绝不在外部复制收尾逻辑，也不要靠自增 token 去"取消"等待：那样 await 永不 resolve，队列会停摆。
    let toastWaitResolve: (() => void) | null = null;

    /** 结束当前等待（幂等）：唤醒 await 中的 processToastQueue */
    const releaseToastWait = () => {
        const resolve = toastWaitResolve;
        toastWaitResolve = null;
        if (resolve) resolve();
    };
    // 记录最近一次已应用的 toast 岛宽，避免连续音量更新反复触发同尺寸动画
    let lastToastIslandWidth: number | null = null;
    // 同理记录岛高：两行天气类 65px、其余 42px，宽同高不同也不能跳过动画
    let lastToastIslandHeight: number | null = null;

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

    /** 可取消的等待：token 变化或组件卸载后旧等待立即失效；唤醒器登记在 toastWaitResolve 供外部中断 */
    const waitUntilToastDeadline = (token: number): Promise<void> => {
        return new Promise((resolve) => {
            const finish = () => {
                if (toastWaitResolve === finish) toastWaitResolve = null;
                resolve();
            };
            const tick = () => {
                if (token !== toastWaitToken) {
                    finish();
                    return;
                }
                const remaining = toastDeadlineAt - Date.now();
                if (remaining <= 0) {
                    toastWaitTimer = null;
                    finish();
                    return;
                }
                toastWaitTimer = setTimeout(tick, remaining);
            };
            clearToastWaitTimer();
            toastWaitResolve = finish;
            tick();
        });
    };

    /** 持久停留项的等待：不设截止时间，由 releaseToastWait() 唤醒（点 X / 被顶替 / 消息插队） */
    const waitUntilPersistentDismissed = (token: number): Promise<void> => {
        return new Promise((resolve) => {
            if (token !== toastWaitToken) {
                resolve();
                return;
            }
            toastWaitResolve = resolve;
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

    /** 用 canvas 测量 toast 文本像素宽度（测量实现与字体口径统一在 utils/textMeasure） */
    const measureToastTextWidth = (text: string): number => measureTextWidth(text, FONT_TOAST_TITLE);

    /**
     * 计算系统 toast 目标岛宽，确保长文与频谱/实时活动区共存时文本可完整展示。
     * 布局：padding 14×2 + 图标有效占位 + 文本 + 频谱/状态点预留 +（split 时）右侧实时活动 44px
     */
    const calcSysToastWidth = (text: string, type: SysToastType, title?: string, body?: string): number => {
        // 两行排版（天气速报 / 恶劣天气 / 日程提醒）宽度取两行中较宽者。
        // 岛高已随两行天气类抬到 65（对齐消息通知展开态），字号同步放大
        // （标题 14px / 正文 12.5px），测量口径必须与放大后的 CSS 一致
        const textW = (title || body)
            ? Math.max(
                measureTextWidth(title || '', FONT_MSG_TITLE),
                measureTextWidth(body || '', FONT_MSG_BODY),
            )
            : measureToastTextWidth(text);
        // 图标占位 = 盒左边缘到文本起点的距离，随排版变化：
        //   单行：padding-left 0 + 图标 translateX(-8px) + 30px + gap 2 + 文本 -2px ≈ 22（另计 textGap）
        //   两行：padding-left 6 + 图标 translateX(2px) + 30px + gap 10 + 文本 -2px ≈ 46（gap 已含在内，故 textGap 记 0）
        // 单行口径保持原值不动，避免既有通知（音量/电源/电池）的岛宽被改窄后文字被截断
        const isTwoLine = TWO_LINE_TOAST_TYPES.has(type) && !!body;
        const iconOccupy = isTwoLine ? 46 : 22;
        const horizontalPadding = 28; // left 14 + right 14（island-core-content 内边距）
        const textGap = isTwoLine ? 0 : 6;
        // 频谱 5×2px + gap + 余量 ≈ 42；无频谱时仍留状态点与少量余量
        // toast 显示期间可与频谱共存，必须为右侧指示区预留宽度，避免文字被裁/遮挡
        const rightIndicator = showSpectrumIndicator() ? 42 : 16;
        // toast 时 isSplitMode 会被强制为 false，但 rt-chip 仍可能绝对定位叠在右侧，
        // 因此按 showRtChip 预留实时活动区宽度，避免长文被小图标遮挡
        const rtChipExtra = showRtChip() ? 44 : 0;
        // 早/午/晚报持久卡片右侧有 X 关闭按钮：额外预留按钮位，避免长文压到 X 下
        const dismissExtra = BRIEF_TOAST_TYPES.has(type) ? 26 : 0;
        const raw = horizontalPadding + iconOccupy + textGap + textW + rightIndicator + rtChipExtra + dismissExtra;

        // 音量/剪贴板文本短，给较窄下限；电源/电池长文本给更宽下限
        // 例：「已接入电源，当前电量 100%」约 13 字 ≈ 162px + 图标/边距/频谱 ≈ 280+
        const minW = (type === 'volume' || type === 'clipboard')
            ? 210
            : (type === 'battery-charge' || type === 'battery-low' ? 300
                : (WEATHER_TOAST_TYPES.has(type) ? 360 : 240));
        const maxW = WEATHER_TOAST_TYPES.has(type) ? 500 : 420;
        return Math.max(minW, Math.min(maxW, raw));
    };

    const applySysToastIslandSize = (text: string, type: SysToastType, title?: string, body?: string) => {
        const targetWidth = calcSysToastWidth(text, type, title, body);
        // 天气类两行通知（速报/恶劣天气）展开到 65px 岛高，样式与消息通知展开态一致；
        // 此前恒为 42px，两行文字被严重压缩（字小、正文截断）
        const targetHeight = TWO_LINE_TOAST_TYPES.has(type) && !!body ? 65 : 42;
        if (lastToastIslandWidth !== null && Math.abs(lastToastIslandWidth - targetWidth) < 2
            && lastToastIslandHeight === targetHeight) {
            return; // 尺寸几乎不变，跳过动画
        }
        lastToastIslandWidth = targetWidth;
        lastToastIslandHeight = targetHeight;
        animateIslandSize(targetWidth, targetHeight);
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
            sysToastTitle.value = nextToast.title || '';
            sysToastBody.value = nextToast.body || '';
            sysToastIcon.value = nextToast.iconKey || '';
            sysToastSeverity.value = nextToast.severity || '';
            displaySysToast.value = true;
            sysToastPersistent.value = !!nextToast.persistent;
            // 停留时长：权限提示最长 6s；日程到点提醒 4s（长标题要看得完，但不常驻）；
            // 其余沿用 2s（音量/剪贴板可续期）
            const dwellMs = nextToast.type === 'notify-permission'
                ? 6000
                : (nextToast.type === 'calendar' ? CALENDAR_REMINDER_DWELL_MS : TOAST_DWELL_MS);
            toastDeadlineAt = Date.now() + dwellMs;
            applySysToastIslandSize(nextToast.text, nextToast.type, nextToast.title, nextToast.body);

            // 自动恢复显示：当有系统通知时，如果灵动岛被隐藏，则自动恢复显示。
            // 剪贴板复制是高频操作：noWake 项跳过该逻辑，绝不把隐藏的岛弹出来
            if (!nextToast.noWake && !isIslandVisible.value) {
                getCurrentWindow().show();
                isIslandVisible.value = true;
            }

            if (nextToast.persistent) {
                // 持久停留（早/午/晚报）：不设截止时间，直到用户点 X 关闭，或被下一条通知顶替。
                // releaseToastWait() 由 dismissSysToast / 新通知让位 / 消息插队三处触发；
                // 唤醒后继续走下面的统一收尾（隐藏 + 恢复尺寸 + 处理下一条）。
                await waitUntilPersistentDismissed(token);
            } else {
                // 可续期停留：连续音量变化会推后 toastDeadlineAt。
                // waitUntilToastDeadline 的 timer 回调会重读 deadline；此处 while 再兜住
                // 「await 返回瞬间又被续期」的竞态。
                while (token === toastWaitToken) {
                    await waitUntilToastDeadline(token);
                    if (token !== toastWaitToken) break;
                    if (Date.now() >= toastDeadlineAt) break;
                }
            }

            // token 失效说明有新一轮处理接管，或组件已卸载
            if (token !== toastWaitToken) {
                isProcessingToast = false;
                return;
            }

            displaySysToast.value = false;
            sysToastPersistent.value = false;
            lastToastIslandWidth = null;
            lastToastIslandHeight = null;
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
        // 持久停留的速报不阻塞消息（消息优先级最高）：先释放其停留让它收尾，
        // 收尾过程中 watch(displaySysToast=false) 会再次唤醒本函数
        if (displaySysToast.value && sysToastPersistent.value) {
            releaseToastWait();
            return;
        }
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
            // 优先用后端解码的应用 logo；取不到时回退本地化名白名单，最后落默认图标
            currentMsgIcon.value = item.icon || getAppIcon(item.app_name);

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
            // 两行排版（天气类）必须把 title/body 一并传入：
            // 只传拼接后的 sysToastText 会把「标题 · 正文」按单行测宽，岛宽偏大且与两行布局不符
            applySysToastIslandSize(sysToastText.value, sysToastType.value, sysToastTitle.value, sysToastBody.value);
        } else {
            lastToastIslandWidth = null;
            lastToastIslandHeight = null;
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
        applySysToastIslandSize(text, sysToastType.value, sysToastTitle.value, sysToastBody.value);
    });

    // 点击系统 toast：notify-permission 类型跳转到 Windows 通知设置
    const onSysToastClick = () => {
        if (sysToastType.value === 'notify-permission') {
            invoke('open_notification_settings').catch(() => {});
        }
    };

    // 暴露给外部调用的触发函数
    // 第 4 个参数承载天气类的两行/图标/配色扩展（其它类型只用 text + type）
    const showToast = (
        text: string,
        type: SysToastType = 'app',
        opts?: {
            noWake?: boolean;
            title?: string;
            body?: string;
            iconKey?: string;
            severity?: string;
            code?: number;
            kind?: string;
            /** 持久停留项（速报）：显示后不自动隐藏，直到用户点 X 关闭或被下一条通知顶替 */
            persistent?: boolean;
        },
    ) => {
        // 新通知到来：正在显示的持久速报立即让位（用户要求：有其他通知要显示时速报不占着岛）。
        // 只释放等待，隐藏/恢复尺寸/切换下一条交回 processToastQueue 统一收尾。
        // persistent 新项（如新一条速报）不触发让位，直接走队列自然切换。
        if (opts?.persistent !== true && displaySysToast.value && sysToastPersistent.value) {
            releaseToastWait();
        }
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

        // 剪贴板：连续复制合并成一条并续期（同 volume 机制），否则复制 10 次岛就抖 10 次；
        // 且必须 noWake：绝不因复制把隐藏的岛弹出来
        if (type === 'clipboard') {
            const item: SysToastItem = { text, type: 'clipboard', noWake: opts?.noWake ?? true };
            // 1) 正在显示剪贴板提示：只更新文案 + 续期
            if (displaySysToast.value && sysToastType.value === 'clipboard') {
                sysToastText.value = text;
                toastDeadlineAt = Date.now() + TOAST_DWELL_MS;
                return;
            }
            // 2) 已在队列中：原地更新为最新文案，避免堆积
            const queuedIdx = toastQueue.value.findIndex((t) => t.type === 'clipboard');
            if (queuedIdx >= 0) {
                toastQueue.value[queuedIdx] = item;
                processToastQueue();
                return;
            }
            // 3) 其余情况（含 leave 动画窗口）正常入队；leave 结束后会立即接上
            toastQueue.value.push(item);
            processToastQueue();
            return;
        }

        // 其余类型（含天气类）：完整承载 opts 扩展字段。
        // 此前只入队 { text, type }，把 title/body/iconKey/severity/code/kind 全部丢掉，
        // 导致 processToastQueue 读到的 nextToast.body 恒为空：
        //   1) 天气速报退化成单行渲染（title · body 拼成长串，nowrap 截断）；
        //   2) sysToastIcon 恒空，图标回退成 'sun'，所有速报一律显示太阳。
        toastQueue.value.push({
            text,
            type,
            noWake: opts?.noWake,
            title: opts?.title,
            body: opts?.body,
            iconKey: opts?.iconKey,
            severity: opts?.severity,
            code: opts?.code,
            kind: opts?.kind,
            persistent: opts?.persistent,
        });
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

    // 把后端 weather-toast 事件映射成灵动岛通知类型
    // 本地日期键 YYYYMMDD：成语按「日期 + 档位」取词，保证同一天同一档位稳定
    const weatherDateKey = (): string => {
        const d = new Date();
        const m = String(d.getMonth() + 1).padStart(2, '0');
        const day = String(d.getDate()).padStart(2, '0');
        return `${d.getFullYear()}${m}${day}`;
    };

    // 把后端 weather-toast 事件映射成灵动岛通知类型。
    // - 早/午/晚报（brief）：标题 = 问好 + 成语标注，正文 = 天气详情（岛上分两行：标题 + 正文小字）；
    // - 恶劣天气（severe）：两行（事件标题 + 详情），按 severity 着色（info 蓝 / warn 橙 / danger 红），
    //   图标由后端 icon 字段决定（rain/fog/haze/alert/temp）。
    const showWeatherToast = (p: {
        kind: string;
        icon?: string;
        title: string;
        body?: string;
        severity?: string;
        code?: number;
    }) => {
        let type: SysToastType = 'weather';
        if (p.kind === 'morning') type = 'weather-morning';
        else if (p.kind === 'noon') type = 'weather-noon';
        else if (p.kind === 'evening') type = 'weather-evening';
        const isBrief = type === 'weather-morning' || type === 'weather-noon' || type === 'weather-evening';
        const title = isBrief
            ? `${p.title} · ${getChengyuByCode(typeof p.code === 'number' ? p.code : 0, weatherDateKey(), p.kind)}`
            : p.title;
        const body = p.body || '';
        const text = body ? `${title} · ${body}` : title;
        showToast(text, type, {
            title,
            body,
            // 早/午/晚报：后端 icon 只有 sun/cloud/rain 粒度，按天气代码细化为
            // rain-light / rain / rain-heavy / thunder…（雨强图标在岛上可辨程度档位）；
            // 恶劣天气仍走 resolveWeatherIconKey（事件标题细化情景）
            iconKey: isBrief && typeof p.code === 'number'
                ? weatherCodeToIcon(p.code)
                : resolveWeatherIconKey(p.icon, title),
            severity: p.severity,
            code: p.code,
            kind: p.kind,
            // 早/午/晚报持久停留：显示后不自动隐藏，直到用户点 X 关闭或被下一条通知顶替；
            // 恶劣天气（severe）保持原有自动消失行为，不加 persistent
            persistent: isBrief,
        });
    };

    // 暴露给外部调用（供 WidgetIsland 监听 weather-toast 事件）
    (window as any).__nsd_showWeatherToast = showWeatherToast;
    void showWeatherToast;

    // 监听消息通知状态：
    // - 消息出现时：若正在显示 volume toast，先中断并塞回队列头部，避免消息结束后音量提示丢失
    // - 消息消失时：立刻唤醒可能被挂起的操作通知队列
    watch(isMsgActive, (newVal) => {
        if (newVal) {
            if (displaySysToast.value && sysToastType.value === 'volume') {
                const volumeText = sysToastText.value;
                toastWaitToken++;
                clearToastWaitTimer();
                // 唤醒可能在 await 中的等待：token 已自增，协程会沿"token 失效"分支直接退出
                // （不唤醒的话 promise 永不 resolve，收尾协程会永久悬挂）
                releaseToastWait();
                displaySysToast.value = false;
                lastToastIslandWidth = null;
                lastToastIslandHeight = null;
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

    // 手动关闭当前系统 toast（速报卡右侧 X）：
    // 只把等待唤醒，隐藏/恢复尺寸/切换下一条全部交回 processToastQueue 统一收尾。
    // 不在这里自增 token 复制一套收尾逻辑：那样 await 中的等待永不 resolve，队列会停摆。
    const dismissSysToast = () => {
        if (!displaySysToast.value) return;
        // 先令可续期等待立刻到期（音量类 toast 走的是「deadline 到了才算结束」的分支）
        toastDeadlineAt = Date.now();
        if (toastWaitResolve) {
            clearToastWaitTimer();
            releaseToastWait();
            return;
        }
        // 兜底：等待已结束（处于 200ms 离场窗口）时直接收尾，避免点了 X 没反应
        displaySysToast.value = false;
        sysToastPersistent.value = false;
        lastToastIslandWidth = null;
        lastToastIslandHeight = null;
        isProcessingToast = false;
        const { h } = getBaseSize();
        const savedWidth = restoreIslandWidth();
        const targetWidth = savedWidth !== null ? savedWidth : currentWidth.value;
        animateIslandSize(targetWidth, h);
        // 手动关闭后重新评估自动隐藏，并唤醒可能排队的消息通知
        scheduleAutoHide();
        processMsgQueue();
    };

    // 组件卸载清理（主组件 onUnmounted 调用）：
    // 使进行中的 toast 等待立即失效，避免卸载后继续改状态
    const cleanupNotifications = () => {
        toastWaitToken++;
        clearToastWaitTimer();
        // 唤醒可能在 await 中的等待：token 已自增，协程会沿"token 失效"分支直接退出，不回写任何状态
        releaseToastWait();
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
        sysToastTitle,
        sysToastBody,
        sysToastIcon,
        sysToastSeverity,
        sysToastPersistent,
        dismissSysToast,
        showToast,
        showSysmsgToast,
        showWeatherToast,
        onSysToastClick,
        handleNotificationClick,
        processMsgQueue,
        cleanupNotifications,
    };
}

export type Notifications = ReturnType<typeof useNotifications>;
