/**
 * 剪贴板历史 composable（plan-20260906 功能二）：
 * - 拉取全量历史（后端已按展示顺序排序：置顶区按置顶时间倒序，其余按复制时间倒序）
 * - 订阅 clipboard-changed：新条目到达时重拉（条目 ≤100，开销可忽略）
 * - 命令封装：启停 / 复制 / 置顶 / 删除 / 清空
 * - 相对时间与体积格式化（供页面与搜索过滤共用）
 */
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** 剪贴板历史条目（与后端 ClipItem 对应；img_path/thumb_path 后端返回时已拼绝对路径） */
export interface ClipItem {
    id: string;
    kind: 'text' | 'image';
    ts_ms: number;
    pinned: boolean;
    pin_ts_ms: number;
    text: string | null;
    char_len: number;
    img_path: string | null;
    thumb_path: string | null;
    img_w: number;
    img_h: number;
    img_bytes: number;
}

/** clipboard-changed 事件载荷 */
export interface ClipChangedPayload {
    id: string;
    kind: string;
    preview: string;
    char_len: number;
}

export function useClipboard() {
    const items = ref<ClipItem[]>([]);

    /** 全量拉取：后端已排序 + 已拼绝对路径 */
    async function refresh() {
        try {
            items.value = await invoke<ClipItem[]>('clipboard_get_history');
        } catch (e) {
            console.error('读取剪贴板历史失败:', e);
            items.value = [];
        }
    }

    /** 启停后台监听线程（命令幂等，两窗口调用安全） */
    async function setEnabled(enabled: boolean) {
        await invoke('clipboard_set_enabled', { enabled });
    }

    /** 点击条目复制（后端带自写豁免：不产生新历史 / 不触发岛提示 / 不重排列表） */
    async function copyItem(id: string) {
        await invoke('clipboard_copy_item', { id });
    }

    /** 置顶 / 取消置顶：仅变更图标，列表顺序不变（下次进入页面才重排） */
    async function togglePin(id: string) {
        await invoke('clipboard_toggle_pin', { id });
    }

    async function deleteItem(id: string) {
        await invoke('clipboard_delete_item', { id });
    }

    async function clearAll() {
        await invoke('clipboard_clear');
    }

    let unlisten: UnlistenFn | null = null;

    /** 订阅新条目事件：增量刷新（简单重拉全量，保证排序与配额一致） */
    async function bindEvents() {
        unlisten = await listen<ClipChangedPayload>('clipboard-changed', () => {
            refresh();
        });
    }

    function unbindEvents() {
        unlisten?.();
        unlisten = null;
    }

    return { items, refresh, setEnabled, copyItem, togglePin, deleteItem, clearAll, bindEvents, unbindEvents };
}

/** 相对时间：刚刚（1 分钟内）→ x 分钟前（1 小时内）→ x 小时前（24 小时内）→ M月d日 */
export function formatRelativeTime(tsMs: number): string {
    const diff = Date.now() - tsMs;
    if (diff < 60_000) return '刚刚';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`;
    const d = new Date(tsMs);
    return `${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 体积格式化：图片条目底栏显示（如 3.2 MB） */
export function formatBytes(bytes: number): string {
    if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
}
