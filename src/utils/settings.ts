/**
 * 设置统一存取层：Rust 侧 config.json 单一数据源。
 * - 读：内存缓存（启动时 initSettings 从后端拉取全量），同步返回，保持 localStorage 的字符串语义
 * - 写：同步更新缓存 + invoke config_set（后端落盘并广播 config-changed）
 * - 跨窗口：监听 config-changed 更新缓存，双窗口不再依赖手动 emit 同步设置值
 * - 迁移：initSettings 把本窗口 localStorage 的 nsd_* 旧键一次性交给后端合并，成功后清除
 *
 * 使用约束：组件 setup 在 main.ts 中 await initSettings() 之后执行，
 * 因此 setup 期间的同步读取一定能拿到后端数据。
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const cache = new Map<string, string>();
let initialized = false;

/** 收集本窗口 localStorage 中需要迁移的旧键（流量统计已下沉 Rust，不参与） */
function collectLegacyKeys(): Record<string, string> {
    const out: Record<string, string> = {};
    for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k && k.startsWith('nsd_') && k !== 'nsd_traffic_stats') {
            out[k] = localStorage.getItem(k) ?? '';
        }
    }
    return out;
}

/** 应用启动时调用一次（两个窗口各自调用）：迁移旧数据 → 拉取全量 → 订阅广播 */
export async function initSettings(): Promise<void> {
    if (initialized) return;
    initialized = true;
    try {
        const legacy = collectLegacyKeys();
        if (Object.keys(legacy).length > 0) {
            await invoke('config_migrate_legacy', { legacy });
            for (const k of Object.keys(legacy)) localStorage.removeItem(k);
        }
    } catch {
        // 后端不可用时保留 localStorage 原数据，不阻塞启动
    }
    try {
        const all = await invoke<Record<string, string>>('config_get_all');
        for (const [k, v] of Object.entries(all)) cache.set(k, v);
    } catch {
        // 缓存保持为空，getSettingRaw 返回 null，由各调用方 fallback 兜底
    }
    try {
        await listen<{ key: string; value: string | null }>('config-changed', (e) => {
            if (e.payload.value === null) {
                cache.delete(e.payload.key);
            } else {
                cache.set(e.payload.key, String(e.payload.value));
            }
        });
    } catch {
        // 事件不可用时仅失去跨窗口实时同步，不影响本窗口读写
    }
}

/** 兼容 localStorage.getItem 语义：返回字符串，未设置返回 null */
export function getSettingRaw(key: string): string | null {
    return cache.get(key) ?? null;
}

/** 兼容 localStorage.setItem 语义：值必须为字符串 */
export function setSettingRaw(key: string, value: string): void {
    cache.set(key, value);
    invoke('config_set', { key, value }).catch(() => { /* 后端不可用时仅内存生效 */ });
}

/** 删除设置项（对应 localStorage.removeItem） */
export function removeSetting(key: string): void {
    cache.delete(key);
    invoke('config_remove', { key }).catch(() => { /* 忽略 */ });
}
