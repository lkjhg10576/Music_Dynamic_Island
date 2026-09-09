/**
 * useWeather composable
 * 监听 weather-tick / weather-toast / weather-light-alert 事件，暴露 ref 给卡片消费
 */

import { ref, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';

export interface WeatherCity {
    cityId: string;
    name: string;
    lat: number;
    lon: number;
    province?: string;
}

export interface WeatherCurrent {
    temperature: number;
    weatherCode: number;
    weatherText: string;
    feelsLike: number;
    humidity: number;
    windSpeed: number;
    pm25: number | null;
    aqi: number | null;
}

export interface WeatherDaily {
    tempMax: number;
    tempMin: number;
    dayCode: number;
    nightCode: number;
    precipProb: number;
    sunrise: string;
    sunset: string;
}

export interface WeatherAlert {
    alertId: string;
    type: string;
    level: string;
    levelText: string;
    title: string;
    pubTime: string | null;
}

export interface WeatherTickPayload {
    city: WeatherCity | null;
    current: WeatherCurrent | null;
    today: WeatherDaily | null;
    tomorrow: WeatherDaily | null;
    alerts: WeatherAlert[];
    lastFetchAt: number;
    fetchStatus: 'ok' | 'failed';
    fetchError?: string;
}

export interface WeatherToastPayload {
    kind: 'severe' | 'morning' | 'noon' | 'evening';
    icon: string;
    title: string;
    body: string;
    severity: 'info' | 'warn' | 'danger';
    dedupKey: string;
}

export interface WeatherLightAlertPayload {
    alertId: string;
    level: string;
    levelText: string;
    type: string;
    title: string;
}

export function useWeather() {
    const city = ref<WeatherCity | null>(null);
    const current = ref<WeatherCurrent | null>(null);
    const today = ref<WeatherDaily | null>(null);
    const tomorrow = ref<WeatherDaily | null>(null);
    const alerts = ref<WeatherAlert[]>([]);
    const lastFetchAt = ref(0);
    const fetchStatus = ref<'ok' | 'failed'>('ok');
    const fetchError = ref<string | undefined>(undefined);

    // 轻提示态
    const isWeatherLightAlerting = ref(false);
    const weatherLightAlert = ref<WeatherLightAlertPayload | null>(null);
    let lightAlertTimer: number | null = null;

    const unlistenFns: Array<() => void> = [];

    onMounted(async () => {
        // 监听 weather-tick
        unlistenFns.push(await listen<WeatherTickPayload>('weather-tick', (event) => {
            const p = event.payload;
            city.value = p.city;
            current.value = p.current;
            today.value = p.today;
            tomorrow.value = p.tomorrow;
            alerts.value = p.alerts || [];
            lastFetchAt.value = p.lastFetchAt;
            fetchStatus.value = p.fetchStatus;
            fetchError.value = p.fetchError;
        }));

        // 监听 weather-toast（由 useNotifications 处理 toast 渲染）
        unlistenFns.push(await listen<WeatherToastPayload>('weather-toast', () => {
            // 实际 toast 触发由 useNotifications 内部处理
        }));

        // 监听 weather-light-alert
        unlistenFns.push(await listen<WeatherLightAlertPayload>('weather-light-alert', (event) => {
            const p = event.payload;
            weatherLightAlert.value = p;
            isWeatherLightAlerting.value = true;

            // 5s 倒计时
            if (lightAlertTimer) {
                clearTimeout(lightAlertTimer);
            }
            lightAlertTimer = window.setTimeout(() => {
                isWeatherLightAlerting.value = false;
                lightAlertTimer = null;
            }, 5000);
        }));
    });

    onUnmounted(() => {
        unlistenFns.forEach(fn => fn());
        if (lightAlertTimer) {
            clearTimeout(lightAlertTimer);
        }
    });

    // 用户点击 chip 展开面板后，关闭时立即隐藏
    function dismissWeatherLightAlert() {
        isWeatherLightAlerting.value = false;
        if (lightAlertTimer) {
            clearTimeout(lightAlertTimer);
            lightAlertTimer = null;
        }
    }

    return {
        city,
        current,
        today,
        tomorrow,
        alerts,
        lastFetchAt,
        fetchStatus,
        fetchError,
        isWeatherLightAlerting,
        weatherLightAlert,
        dismissWeatherLightAlert,
    };
}

export type WeatherComposable = ReturnType<typeof useWeather>;
