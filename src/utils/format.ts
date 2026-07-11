/**
 * 网速格式化：将字节数转换为人类可读的速度字符串
 */
export const formatSpeed = (bytes: number): string => {
    if (bytes < 1024) return bytes + ' B/s';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB/s';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB/s';
};
