/**
 * 天气代码表、预警类型表、成语库、idempotent 文案生成器
 */

// 天气现象代码对照表
export const WEATHER_CODE_MAP: Record<number, string> = {
    0: '晴',
    1: '多云',
    2: '阴',
    3: '阵雨',
    4: '雷阵雨',
    5: '雷阵雨伴有冰雹',
    6: '雨夹雪',
    7: '小雨',
    8: '中雨',
    9: '大雨',
    10: '暴雨',
    11: '大暴雨',
    12: '特大暴雨',
    13: '阵雪',
    14: '小雪',
    15: '中雪',
    16: '大雪',
    17: '暴雪',
    18: '雾',
    19: '冻雨',
    20: '沙尘暴',
    21: '小到中雨',
    22: '中到大雨',
    23: '大到暴雨',
    24: '暴雨到大暴雨',
    25: '大暴雨到特大暴雨',
    26: '小到中雪',
    27: '中到大雪',
    28: '大到暴雪',
    32: '浓雾',
    35: '轻雾',
    49: '强浓雾',
    53: '霾',
    54: '中度霾',
    55: '重度霾',
    56: '严重霾',
    57: '大雾',
    58: '特强浓雾',
    301: '雨',
    302: '雪',
};

// 预警中文 type → 图标归类。
// 查表按声明顺序做子串匹配、命中即返回，因此**具体词必须排在泛化词之前**：
// 末尾四条是泛化兜底（覆盖「大雪预警」「雷阵雨」等未逐一列举的类型名）。
export const ALERT_TYPE_KEYWORDS: Record<string, string> = {
    '台风': 'typhoon',
    '暴雨': 'rain',
    '雷雨大风': 'rain',
    '强对流': 'rain',
    '冰雹': 'rain',
    '暴雪': 'snow',
    '道路结冰': 'snow',
    '低温': 'snow',
    '寒潮': 'wind',
    '大风': 'wind',
    '霜冻': 'wind',
    '沙尘暴': 'wind',
    '高温': 'heat',
    '大雾': 'fog',
    '霾': 'fog',
    '浮尘': 'fog',
    '雷电': 'thunder',
    '干旱': 'drought',
    '地质灾害': 'geo',
    '森林': 'fire',
    '草原': 'fire',
    '火险': 'fire',
    // ── 泛化兜底（务必保持在本表末尾）──
    '雪': 'snow',
    '雨': 'rain',
    '风': 'wind',
    '雾': 'fog',
};

// 天气级图标映射
export function weatherCodeToIcon(code: number): string {
    if (code === 0) return 'sun';
    if (code === 1 || code === 2) return 'cloud';
    if (code === 6) return 'sleet';
    if ([3, 4, 5, 7, 8, 9, 10, 11, 12, 19, 21, 22, 23, 24, 25, 301].includes(code)) return 'rain';
    if ([13, 14, 15, 16, 17, 26, 27, 28, 302].includes(code)) return 'snow';
    if ([18, 32, 35, 49, 57, 58].includes(code)) return 'fog';
    if ([53, 54, 55, 56].includes(code)) return 'haze';
    return 'sun';
}

// 预警级图标映射
export function alertLevelToIcon(level: string): string {
    switch (level) {
        case 'B': return 'alert-blue';
        case 'Y': return 'alert-yellow';
        case 'O': return 'alert-orange';
        case 'R': return 'alert-red';
        case 'W': return 'alert-white';
        default: return 'alert-blue';
    }
}

// ──────────────────────────────────────────────
// 岛上天气图标（toast 左侧图标 / 实时活动小图标 / 轻提示面板共用）
// 此前这套 SVG 私有在 IslandWeatherAlert 内，只有系统 toast 用得到，
// 小图标与轻提示面板只能一律画同一个三角；抽到此处后三处共用同一份图标与解析口径。
// ──────────────────────────────────────────────

/** 图标键 → SVG 字符串（stroke 用 currentColor，由调用方着色） */
export const WEATHER_ICON_SVG: Record<string, string> = {
    sun: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>',
    moon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>',
    cloud: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path></svg>',
    rain: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="21" x2="7" y2="23"></line><line x1="12" y1="21" x2="11" y2="23"></line><line x1="16" y1="21" x2="15" y2="23"></line></svg>',
    snow: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="20" x2="8" y2="22"></line><line x1="12" y1="20" x2="12" y2="22"></line><line x1="16" y1="20" x2="16" y2="22"></line></svg>',
    sleet: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"></path><line x1="8" y1="21" x2="7" y2="23"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="16" y1="21" x2="15" y2="23"></line></svg>',
    fog: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="9" x2="20" y2="9"></line><line x1="4" y1="13" x2="20" y2="13"></line><line x1="4" y1="17" x2="20" y2="17"></line></svg>',
    haze: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="8" x2="20" y2="8"></line><line x1="6" y1="12" x2="18" y2="12"></line><line x1="4" y1="16" x2="20" y2="16"></line></svg>',
    temp: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z"></path></svg>',
    wind: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 8h10a3 3 0 1 0-3-3"></path><path d="M3 16h13a3 3 0 1 1-3 3"></path><path d="M3 12h7"></path></svg>',
    alert: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>',
};

/** 预警归类 → 岛上 SVG 图标键 */
const ALERT_GROUP_TO_ICON: Record<string, string> = {
    typhoon: 'wind',
    rain: 'rain',
    snow: 'snow',
    wind: 'wind',
    heat: 'temp',
    fog: 'fog',
    thunder: 'alert',
    drought: 'temp',
    geo: 'alert',
    fire: 'alert',
};

/**
 * 预警类型文本 → 岛上图标键（按"具体情况"区分图标）。
 * 霾/浮尘/沙尘先于查表单独判为 haze：它们与雾同属能见度障碍，但视觉上更"黄"，
 * 混在 fog 里就分不出「大雾」和「沙尘」了。
 */
export function weatherAlertIconKey(typeText: string, fallback = 'alert'): string {
    const text = (typeText || '').trim();
    if (!text) return fallback;
    if (text.includes('霾') || text.includes('浮尘') || text.includes('沙尘')) return 'haze';
    for (const [keyword, group] of Object.entries(ALERT_TYPE_KEYWORDS)) {
        if (text.includes(keyword)) return ALERT_GROUP_TO_ICON[group] || fallback;
    }
    return fallback;
}

/**
 * 解析最终图标键：后端 icon 优先；但它对"预警"类事件一律发通用 alert（三角），
 * 此时用事件标题/预警类型文本细化（暴雨→rain、大雾→fog、高温→temp…），
 * 让不同情景的预警在岛上一眼可分。文本也推不出具体情景时保持 alert。
 */
export function resolveWeatherIconKey(iconKey: string | undefined, typeText: string | undefined): string {
    const key = (iconKey || '').trim();
    if (!key || key === 'alert') {
        const specific = weatherAlertIconKey(typeText || '');
        if (specific !== 'alert') return specific;
    }
    return key || 'alert';
}

/** 取图标 SVG；未知键回退 fallback（默认 alert 三角） */
export function weatherIconSvgOf(iconKey: string | undefined, fallback = 'alert'): string {
    const key = (iconKey || '').trim();
    return WEATHER_ICON_SVG[key] || WEATHER_ICON_SVG[fallback] || WEATHER_ICON_SVG.alert;
}

// 成语库（按天气代码分组）
export const CHENGYU_MAP: Record<string, string[]> = {
    'sunny': ['风和日丽', '晴空万里', '阳光明媚', '万里无云'],
    'cloudy': ['浮云蔽日', '云卷云舒', '天高云淡', '薄云遮日'],
    'overcast': ['乌云密布', '阴云密布', '天色阴沉', '彤云密布'],
    'rain': ['雨水如注', '风雨如磐', '大雨倾盆', '风雨大作'],
    'sleet': ['雨雪交加', '雨雪霏霏', '雪雨兼程', '半雨半雪'],
    'snow': ['银装素裹', '大雪纷飞', '雪花飘零', '飞雪漫天'],
    'fog': ['雾气弥漫', '云雾迷蒙', '雾色茫茫', '浓雾重重'],
    'haze': ['雾霾笼罩', '尘霾蔽空', '烟尘弥漫', '浮尘蔽日'],
    'unknown': ['风云变幻'],
};

// 根据天气代码获取成语
export function getChengyuByCode(code: number, dateKey: string, kind: string): string {
    let group: string;
    if (code === 0) group = 'sunny';
    else if (code === 1) group = 'cloudy';
    else if (code === 2) group = 'overcast';
    else if ([3, 4, 5, 7, 8, 9, 10, 11, 12, 19, 21, 22, 23, 24, 25, 301].includes(code)) group = 'rain';
    else if (code === 6) group = 'sleet';
    else if ([13, 14, 15, 16, 17, 26, 27, 28, 302].includes(code)) group = 'snow';
    else if ([18, 32, 35, 49, 57, 58].includes(code)) group = 'fog';
    else if ([53, 54, 55, 56].includes(code)) group = 'haze';
    else group = 'unknown';

    const arr = CHENGYU_MAP[group] || CHENGYU_MAP['unknown'];
    // 使用 dateKey + kind 作为种子，确保同一天同一档位成语一致
    const seed = hashString(`${dateKey}-${kind}`);
    return arr[seed % arr.length];
}

// 简单字符串 hash
function hashString(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash; // Convert to 32bit integer
    }
    return Math.abs(hash);
}

// 生成 toast 内容文案
export function generateToastBody(params: {
    weatherText: string;
    temp: number;
    feelsLike: number;
    todayMax: number;
    todayMin: number;
    tomorrowWeatherText?: string;
    tomorrowMax?: number;
    tomorrowMin?: number;
    isEvening?: boolean;
}): string {
    const { weatherText, temp, feelsLike, todayMax, todayMin, tomorrowWeatherText, tomorrowMax, tomorrowMin, isEvening } = params;
    const tempRounded = Math.round(temp);
    const feelsLikeRounded = Math.round(feelsLike);
    const tempGap = Math.abs(temp - feelsLike);

    let body = '';
    if (isEvening && tomorrowWeatherText !== undefined && tomorrowMax !== undefined && tomorrowMin !== undefined) {
        // 晚报：今日...转...
        body = `今日${weatherText}转${tomorrowWeatherText}，${tempRounded}℃`;
    } else {
        body = `今日${weatherText}，${tempRounded}℃`;
    }

    // 体感省略规则：差 ≤ 1℃ 省略
    if (tempGap > 1) {
        body += `，体感${feelsLikeRounded}℃`;
    }

    // 温差较大：差 > 5℃
    if (tempGap > 5) {
        body += '，温差较大';
    }

    if (isEvening && tomorrowMax !== undefined && tomorrowMin !== undefined) {
        body += `，${Math.round(tomorrowMax)}/${Math.round(tomorrowMin)}℃`;
    } else {
        body += `，${Math.round(todayMax)}/${Math.round(todayMin)}℃`;
    }

    return body;
}

// 预警等级映射
export const ALERT_LEVEL_MAP: Record<string, string> = {
    'B': '蓝色预警',
    'Y': '黄色预警',
    'O': '橙色预警',
    'R': '红色预警',
    'W': '白色预警',
    '蓝色': '蓝色预警',
    '黄色': '黄色预警',
    '橙色': '橙色预警',
    '红色': '红色预警',
    '白色': '白色预警',
};

export function getAlertLevelText(level: string): string {
    return ALERT_LEVEL_MAP[level] || '蓝色预警';
}

// 预警等级排序值
export const ALERT_LEVEL_RANK: Record<string, number> = {
    'B': 0, '蓝色': 0,
    'Y': 1, '黄色': 1,
    'O': 2, '橙色': 2,
    'R': 3, '红色': 3,
    'W': -1, '白色': -1,
};

export function getAlertLevelRank(level: string): number {
    return ALERT_LEVEL_RANK[level] ?? 0;
}
