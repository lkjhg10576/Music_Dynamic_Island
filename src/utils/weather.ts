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

// 预警中文 type → 图标归类
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
