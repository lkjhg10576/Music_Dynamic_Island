<template>
    <canvas ref="canvasRef" class="speed-canvas"></canvas>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';

const props = defineProps<{
    data: number[];
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let rafId: number | null = null;

const getColors = () => {
    const isDark = document.documentElement.classList.contains('dark-theme');
    return {
        line: isDark ? '#60a5fa' : '#3b82f6',
        areaStart: isDark ? 'rgba(96, 165, 250, 0.4)' : 'rgba(59, 130, 246, 0.4)',
        areaEnd: isDark ? 'rgba(96, 165, 250, 0.0)' : 'rgba(59, 130, 246, 0.0)',
        grid: isDark ? 'rgba(255,255,255,0.04)' : 'rgba(0,0,0,0.04)',
    };
};

// 贝塞尔平滑曲线（仅添加路径，不开启新路径）
const drawSmoothLine = (points: { x: number; y: number }[]) => {
    if (!ctx || points.length < 2) return;

    for (let i = 0; i < points.length - 1; i++) {
        const p0 = points[Math.max(i - 1, 0)];
        const p1 = points[i];
        const p2 = points[i + 1];
        const p3 = points[Math.min(i + 2, points.length - 1)];

        const tension = 0.3;
        const cp1x = p1.x + (p2.x - p0.x) * tension;
        const cp1y = p1.y + (p2.y - p0.y) * tension;
        const cp2x = p2.x - (p3.x - p1.x) * tension;
        const cp2y = p2.y - (p3.y - p1.y) * tension;

        ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
    }
};

const draw = () => {
    if (!canvasRef.value) return;

    const canvas = canvasRef.value;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    // 设置实际像素尺寸
    if (canvas.width !== rect.width * dpr || canvas.height !== rect.height * dpr) {
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
    }

    ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const w = rect.width;
    const h = rect.height;
    const padding = { top: 4, bottom: 4, left: 0, right: 0 };
    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    // 清除
    ctx.clearRect(0, 0, w, h);

    const data = props.data;
    if (!data || data.length === 0) return;

    const colors = getColors();

    // 计算数据点坐标
    const maxVal = Math.max(...data, 0.01); // 防止全 0
    const points: { x: number; y: number }[] = [];
    for (let i = 0; i < data.length; i++) {
        const x = padding.left + (i / (data.length - 1)) * chartW;
        const y = padding.top + chartH - (data[i] / maxVal) * chartH;
        points.push({ x, y });
    }

    // 绘制渐变填充区域
    const gradient = ctx.createLinearGradient(0, padding.top, 0, h - padding.bottom);
    gradient.addColorStop(0, colors.areaStart);
    gradient.addColorStop(1, colors.areaEnd);

    ctx.beginPath();
    ctx.moveTo(points[0].x, h - padding.bottom);
    ctx.lineTo(points[0].x, points[0].y);
    drawSmoothLine(points);
    ctx.lineTo(points[points.length - 1].x, h - padding.bottom);
    ctx.closePath();
    ctx.fillStyle = gradient;
    ctx.fill();

    // 绘制线条
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    drawSmoothLine(points);
    ctx.strokeStyle = colors.line;
    ctx.lineWidth = 2;
    ctx.stroke();
};

// 监听数据变化重绘
watch(() => props.data, () => {
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
    rafId = requestAnimationFrame(draw);
}, { deep: true });

// 监听主题变化
let themeObserver: MutationObserver | null = null;

onMounted(() => {
    draw();

    // 监听主题切换
    themeObserver = new MutationObserver(() => {
        requestAnimationFrame(draw);
    });
    themeObserver.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['class'],
    });
});

onUnmounted(() => {
    themeObserver?.disconnect();
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
});

// 暴露 resize 方法
const resize = () => {
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
    rafId = requestAnimationFrame(draw);
};

defineExpose({ resize });
</script>

<style scoped>
.speed-canvas {
    width: 100%;
    height: 100%;
    display: block;
}
</style>
