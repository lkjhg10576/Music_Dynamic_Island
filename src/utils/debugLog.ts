/**
 * 临时诊断日志（带外 9.9.9-1 专用）：前端统一经 append_frontend_log 落盘到
 * 后端应用数据目录 nsd_debug.log（与 Rust 侧 debug_log.rs 写同一文件）。
 *
 * 覆盖三类信号：
 *   - 全局错误捕获：window error / unhandledrejection / console.error|warn 镜像；
 *   - 显式埋点：dlog()（点位见各调用处，均针对「点击不响应 / 字符图标不显示」两问题）；
 *   - describeEventTarget：命中元素类名链，用于核对"用户看到的元素"与"实际命中的元素"。
 *
 * main / widget 两个窗口共用本入口（main.ts 调用），每条日志自带窗口标签前缀。
 * 诊断结束后：本文件、main.ts 的初始化调用与全部 dlog 埋点随 9.9.9-1 一起移除。
 */
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

type LogLevel = 'info' | 'warn' | 'error';

const MAX_MESSAGE_CHARS = 2000;
let windowLabel = 'unknown';
let installed = false;
// 防递归：镜像 console 输出时内部再报错不能再次进入镜像
let mirroring = false;

function fmtPart(part: unknown): string {
    if (typeof part === 'string') return part;
    if (part instanceof Error) {
        return `${part.name}: ${part.message}${part.stack ? ` | ${part.stack}` : ''}`;
    }
    try {
        return JSON.stringify(part) ?? String(part);
    } catch (_e) {
        return String(part);
    }
}

function truncate(message: string): string {
    if (message.length <= MAX_MESSAGE_CHARS) return message;
    return `${message.slice(0, MAX_MESSAGE_CHARS)}…(截断,原长${message.length})`;
}

/** 写入一条日志到后端日志文件（fire-and-forget；失败静默，避免形成反馈循环） */
export function dlog(level: LogLevel, tag: string, ...parts: unknown[]): void {
    const message = truncate(`[${windowLabel}] ${parts.map(fmtPart).join(' ')}`);
    try {
        invoke('append_frontend_log', { level, tag, message }).catch(() => { /* 落盘失败静默 */ });
    } catch (_e) { /* 后端命令缺失（非 9.9.9-1 构建）时静默 */ }
}

/**
 * 事件目标描述：标签 + 自身及向上最多 2 层的类名链。
 * 用于核对点击命中：用户以为点在 A 上，实际 DOM 命中的可能是别的元素。
 */
export function describeEventTarget(e: Event): string {
    const chain: string[] = [];
    let node: Element | null = e.target instanceof Element ? e.target : null;
    for (let depth = 0; node && depth < 3; depth++) {
        // SVG 元素的 className 是 SVGAnimatedString，先做类型守卫
        const rawClass = typeof node.className === 'string' ? node.className : '';
        const cls = rawClass.trim() ? `.${rawClass.trim().split(/\s+/).join('.')}` : '';
        chain.push(`<${node.tagName.toLowerCase()}>${cls}`);
        node = node.parentElement;
    }
    return chain.join(' <- ');
}

function mirrorConsole(): void {
    const originalError = console.error.bind(console);
    const originalWarn = console.warn.bind(console);
    console.error = (...args: unknown[]) => {
        originalError(...args);
        if (!mirroring) {
            mirroring = true;
            try { dlog('error', 'console.error', ...args); } finally { mirroring = false; }
        }
    };
    console.warn = (...args: unknown[]) => {
        originalWarn(...args);
        if (!mirroring) {
            mirroring = true;
            try { dlog('warn', 'console.warn', ...args); } finally { mirroring = false; }
        }
    };
}

/**
 * 在 main.ts 挂载应用前调用，安装全局错误捕获并写初始化横幅。幂等。
 */
export function initFrontendDebugLog(): void {
    if (installed) return;
    installed = true;
    try {
        windowLabel = getCurrentWindow().label;
    } catch (_e) { /* IPC 未就绪时保持 unknown */ }

    window.addEventListener('error', (e) => {
        dlog('error', 'js-error', e.message, `@${e.filename}:${e.lineno}:${e.colno}`,
            e.error instanceof Error ? (e.error.stack ?? '') : '');
    });
    window.addEventListener('unhandledrejection', (e) => {
        dlog('error', 'js-rejection', e.reason instanceof Error
            ? `${e.reason.name}: ${e.reason.message} | ${e.reason.stack ?? ''}`
            : String(e.reason));
    });
    mirrorConsole();

    dlog('info', 'boot', `前端日志初始化 label=${windowLabel} dpr=${window.devicePixelRatio}`,
        `zoom=${document.documentElement.style.zoom || '(未设置)'}`,
        `viewport=${window.innerWidth}x${window.innerHeight}`);
}
