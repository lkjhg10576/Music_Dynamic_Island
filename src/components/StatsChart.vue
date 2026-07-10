<template>
    <div class="stats-chart-wrapper">
        <!-- 图例 -->
        <div class="stats-legend">
            <span class="legend-item">
                <span class="legend-dot" :style="{ background: upColor }"></span>
                上传 (MB)
            </span>
            <span class="legend-item">
                <span class="legend-dot" :style="{ background: downColor }"></span>
                下载 (MB)
            </span>
        </div>
        <canvas ref="canvasRef" class="stats-canvas"></canvas>
        <!-- Tooltip -->
        <div v-if="tooltip.visible" class="stats-tooltip" :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }">
            <div class="tooltip-header">{{ tooltip.title }}</div>
            <div class="tooltip-divider"></div>
            <div class="tooltip-body">
                <div class="tooltip-row">
                    <span class="tooltip-dot" :style="{ background: upColor }"></span>
                    <span class="tooltip-label">上传</span>
                    <span class="tooltip-value">{{ tooltip.up }} MB</span>
                </div>
                <div class="tooltip-row">
                    <span class="tooltip-dot" :style="{ background: downColor }"></span>
                    <span class="tooltip-label">下载</span>
                    <span class="tooltip-value">{{ tooltip.down }} MB</span>
                </div>
            </div>
            <div class="tooltip-arrow"></div>
        </div>
        <!-- 折线图轴指示线 -->
        <div v-if="tooltip.visible && chartType === 'line'" class="axis-pointer-line" :style="{ left: tooltip.lineX + 'px' }"></div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, reactive } from 'vue';

const props = defineProps<{
    days: string[];
    upData: number[];
    downData: number[];
    chartType: 'bar' | 'line';
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let rafId: number | null = null;

const tooltip = reactive({
    visible: false,
    x: 0,
    y: 0,
    lineX: 0,
    title: '',
    up: '0',
    down: '0',
    index: -1,
});

const getColors = () => {
    const isDark = document.documentElement.classList.contains('dark-theme');
    return {
        textColor: isDark ? '#94a3b8' : '#64748b',
        splitLine: isDark ? '#383c41' : '#f1f5f9',
        upColor: isDark ? '#60a5fa' : '#3b82f6',
        downColor: isDark ? '#34d399' : '#10b981',
    };
};

const upColor = ref('#3b82f6');
const downColor = ref('#10b981');

const barRects: { x: number; y: number; w: number; h: number; index: number; series: 'up' | 'down' }[] = [];

const draw = () => {
    if (!canvasRef.value) return;

    const canvas = canvasRef.value;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;

    if (canvas.width !== rect.width * dpr || canvas.height !== rect.height * dpr) {
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
    }

    ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const w = rect.width;
    const h = rect.height;
    const padding = { top: 4, bottom: 28, left: 10, right: 10 };
    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    ctx.clearRect(0, 0, w, h);

    const colors = getColors();
    upColor.value = colors.upColor;
    downColor.value = colors.downColor;

    const { days, upData, downData, chartType } = props;
    if (!days.length) return;

    const maxVal = Math.max(...upData, ...downData, 1);

    barRects.length = 0;

    // 绘制 X 轴标签
    ctx.fillStyle = colors.textColor;
    ctx.font = '11px -apple-system, sans-serif';
    ctx.textAlign = 'center';
    const categoryW = chartW / days.length;
    for (let i = 0; i < days.length; i++) {
        const x = padding.left + categoryW * i + categoryW / 2;
        ctx.fillText(days[i], x, h - padding.bottom + 18);
    }

    // 绘制水平网格线
    const gridLines = 4;
    ctx.strokeStyle = colors.splitLine;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    for (let i = 0; i <= gridLines; i++) {
        const y = padding.top + (chartH / gridLines) * i;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(w - padding.right, y);
        ctx.stroke();

        // Y 轴标签
        const val = ((maxVal * (gridLines - i)) / gridLines).toFixed(1);
        ctx.fillStyle = colors.textColor;
        ctx.textAlign = 'left';
        ctx.fillText(val, padding.left, y - 4);
    }
    ctx.setLineDash([]);

    if (chartType === 'bar') {
        // 柱状图
        const groupWidth = categoryW * 0.6;
        const barWidth = groupWidth / 2 - 2;
        const barGap = 4;

        for (let i = 0; i < days.length; i++) {
            const groupX = padding.left + categoryW * i + (categoryW - groupWidth) / 2;

            // 上传柱
            const upH = (upData[i] / maxVal) * chartH;
            const upX = groupX;
            const upY = padding.top + chartH - upH;

            ctx.fillStyle = colors.upColor;
            ctx.beginPath();
            ctx.roundRect(upX, upY, barWidth, upH, [3, 3, 0, 0]);
            ctx.fill();
            barRects.push({ x: upX, y: upY, w: barWidth, h: upH, index: i, series: 'up' });

            // 下载柱
            const downH = (downData[i] / maxVal) * chartH;
            const downX = groupX + barWidth + barGap;
            const downY = padding.top + chartH - downH;

            ctx.fillStyle = colors.downColor;
            ctx.beginPath();
            ctx.roundRect(downX, downY, barWidth, downH, [3, 3, 0, 0]);
            ctx.fill();
            barRects.push({ x: downX, y: downY, w: barWidth, h: downH, index: i, series: 'down' });
        }
    } else {
        // 折线图
        const drawLine = (data: number[], color: string) => {
            const points: { x: number; y: number }[] = [];
            for (let i = 0; i < data.length; i++) {
                const x = padding.left + categoryW * i + categoryW / 2;
                const y = padding.top + chartH - (data[i] / maxVal) * chartH;
                points.push({ x, y });
            }

            // 渐变填充
            const gradient = ctx!.createLinearGradient(0, padding.top, 0, padding.top + chartH);
            gradient.addColorStop(0, color + '30');
            gradient.addColorStop(1, color + '00');

            ctx!.beginPath();
            ctx!.moveTo(points[0].x, padding.top + chartH);
            ctx!.lineTo(points[0].x, points[0].y);
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
                ctx!.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
            }
            ctx!.lineTo(points[points.length - 1].x, padding.top + chartH);
            ctx!.closePath();
            ctx!.fillStyle = gradient;
            ctx!.fill();

            // 线条
            ctx!.beginPath();
            ctx!.moveTo(points[0].x, points[0].y);
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
                ctx!.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
            }
            ctx!.strokeStyle = color;
            ctx!.lineWidth = 2;
            ctx!.stroke();

            // 数据点
            for (const p of points) {
                ctx!.beginPath();
                ctx!.arc(p.x, p.y, 3, 0, Math.PI * 2);
                ctx!.fillStyle = color;
                ctx!.fill();
                ctx!.strokeStyle = '#fff';
                ctx!.lineWidth = 1.5;
                ctx!.stroke();
            }
        };

        drawLine(downData, colors.downColor);
        drawLine(upData, colors.upColor);
    }
};

// 鼠标悬停 tooltip
const handleMouseMove = (e: MouseEvent) => {
    if (!canvasRef.value) return;
    const rect = canvasRef.value.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const padding = { top: 4, bottom: 28, left: 10, right: 10 };
    const tooltipW = 130;
    const tooltipH = 80;

    // 辅助函数：修正 tooltip 位置防止超出边界
    const clampTooltip = (tx: number, ty: number) => {
        let clampedX = tx;
        let clampedY = ty;
        if (clampedX < 0) clampedX = 0;
        if (clampedX + tooltipW > rect.width) clampedX = rect.width - tooltipW;
        if (clampedY < 0) clampedY = 4;
        return { x: clampedX, y: clampedY };
    };

    if (props.chartType === 'bar') {
        let found = false;
        for (const bar of barRects) {
            if (x >= bar.x && x <= bar.x + bar.w && y >= bar.y && y <= bar.y + bar.h) {
                tooltip.visible = true;
                const centerX = bar.x + bar.w / 2;
                const pos = clampTooltip(centerX - tooltipW / 2, bar.y - tooltipH - 6);
                tooltip.x = pos.x;
                tooltip.y = pos.y;
                tooltip.lineX = centerX;
                tooltip.title = props.days[bar.index];
                tooltip.up = props.upData[bar.index].toFixed(2);
                tooltip.down = props.downData[bar.index].toFixed(2);
                tooltip.index = bar.index;
                found = true;
                break;
            }
        }
        if (!found) tooltip.visible = false;
    } else {
        // 折线图：检测最近的数据点
        const chartW = rect.width - padding.left - padding.right;
        const categoryW = chartW / props.days.length;
        let nearest = -1;
        let minDist = Infinity;
        for (let i = 0; i < props.days.length; i++) {
            const cx = padding.left + categoryW * i + categoryW / 2;
            const dist = Math.abs(x - cx);
            if (dist < minDist && dist < categoryW / 2) {
                minDist = dist;
                nearest = i;
            }
        }
        if (nearest >= 0) {
            const centerX = padding.left + categoryW * nearest + categoryW / 2;
            const maxVal = Math.max(...props.upData, ...props.downData, 1);
            const chartH = rect.height - padding.top - padding.bottom;
            const maxData = Math.max(props.upData[nearest], props.downData[nearest]);
            const topY = padding.top + chartH - (maxData / maxVal) * chartH;

            tooltip.visible = true;
            const pos = clampTooltip(centerX - tooltipW / 2, topY - tooltipH - 6);
            tooltip.x = pos.x;
            tooltip.y = pos.y;
            tooltip.lineX = centerX;
            tooltip.title = props.days[nearest];
            tooltip.up = props.upData[nearest].toFixed(2);
            tooltip.down = props.downData[nearest].toFixed(2);
            tooltip.index = nearest;
        } else {
            tooltip.visible = false;
        }
    }
};

const handleMouseLeave = () => {
    tooltip.visible = false;
};

watch(() => [props.upData, props.downData, props.chartType, props.days], () => {
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
    rafId = requestAnimationFrame(draw);
}, { deep: true });

let themeObserver: MutationObserver | null = null;

onMounted(() => {
    draw();
    canvasRef.value?.addEventListener('mousemove', handleMouseMove);
    canvasRef.value?.addEventListener('mouseleave', handleMouseLeave);

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
    canvasRef.value?.removeEventListener('mousemove', handleMouseMove);
    canvasRef.value?.removeEventListener('mouseleave', handleMouseLeave);
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
});

const resize = () => {
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
    }
    rafId = requestAnimationFrame(draw);
};

defineExpose({ resize });
</script>

<style scoped>
.stats-chart-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
}

.stats-legend {
    display: flex;
    gap: 16px;
    justify-content: center;
    margin-bottom: 4px;
    font-size: 12px;
    color: var(--speed-label, #64748b);
}

.legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
}

.legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
}

.stats-canvas {
    width: 100%;
    height: calc(100% - 24px);
    display: block;
}

.stats-tooltip {
    position: absolute;
    background: rgba(255, 255, 255, 0.96);
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 0;
    font-size: 12px;
    pointer-events: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    z-index: 10;
    white-space: nowrap;
    min-width: 120px;
}

:global(.dark-theme) .stats-tooltip {
    background: rgba(41, 43, 46, 0.96);
    border-color: #474c53;
}

.tooltip-header {
    font-weight: 600;
    font-size: 13px;
    padding: 6px 10px 4px;
    color: #1e293b;
    border-bottom: none;
}

:global(.dark-theme) .tooltip-header {
    color: #f1f5f9;
}

.tooltip-divider {
    height: 1px;
    background: #e2e8f0;
    margin: 0;
}

:global(.dark-theme) .tooltip-divider {
    background: #474c53;
}

.tooltip-body {
    padding: 4px 10px 6px;
}

.tooltip-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    color: #64748b;
}

:global(.dark-theme) .tooltip-row {
    color: #94a3b8;
}

.tooltip-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    display: inline-block;
    flex-shrink: 0;
}

.tooltip-label {
    flex: 1;
}

.tooltip-value {
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: #1e293b;
}

:global(.dark-theme) .tooltip-value {
    color: #f1f5f9;
}

.tooltip-arrow {
    position: absolute;
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%);
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid #e2e8f0;
}

:global(.dark-theme) .tooltip-arrow {
    border-top-color: #474c53;
}

.axis-pointer-line {
    position: absolute;
    top: 28px;
    bottom: 28px;
    width: 1px;
    background: rgba(0, 0, 0, 0.1);
    pointer-events: none;
    z-index: 5;
}

:global(.dark-theme) .axis-pointer-line {
    background: rgba(255, 255, 255, 0.1);
}
</style>
