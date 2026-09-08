<template>
    <div class="weather-panel">
        <!-- 城市选择 -->
        <div class="weather-city-row">
            <div class="weather-city-selector" @click="toggleCitySearch">
                <span class="weather-city-name">{{ cityName || '未配置城市' }}</span>
                <svg class="weather-city-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
            </div>
            <div v-if="showCitySearch" class="weather-city-dropdown">
                <input
                    v-model="searchKeyword"
                    class="weather-city-search"
                    placeholder="搜索城市..."
                    @input="onSearchInput"
                />
                <div v-if="searchResults.length > 0" class="weather-city-results">
                    <div
                        v-for="result in searchResults"
                        :key="result.cityId"
                        class="weather-city-result"
                        @click="selectCity(result)"
                    >
                        <span class="weather-city-result-name">{{ result.name }}</span>
                        <span class="weather-city-result-province">{{ result.province }}</span>
                    </div>
                </div>
                <div v-else-if="searchKeyword.length > 0" class="weather-city-no-result">
                    未找到相关城市
                </div>
            </div>
        </div>

        <div class="weather-divider"></div>

        <!-- 当前天气 -->
        <div v-if="current" class="weather-current">
            <div class="weather-current-main">
                <span class="weather-current-temp">{{ Math.round(current.temperature) }}℃</span>
                <span class="weather-current-text">{{ current.weatherText }}</span>
            </div>
            <div class="weather-current-feels">体感 {{ Math.round(current.feelsLike) }}℃</div>
            <div class="weather-current-details">
                <span>湿度 {{ Math.round(current.humidity) }}%</span>
                <span>风 {{ Math.round(current.windSpeed) }}km/h</span>
                <span v-if="current.pm25 !== null">PM2.5 {{ Math.round(current.pm25) }}</span>
            </div>
        </div>

        <!-- 今日天气 -->
        <div v-if="today" class="weather-today">
            <div class="weather-today-temp">{{ Math.round(today.tempMax) }}/{{ Math.round(today.tempMin) }}℃</div>
            <div class="weather-today-details">
                <span>降水 {{ Math.round(today.precipProb) }}%</span>
                <span v-if="today.sunrise">日出 {{ today.sunrise }}</span>
                <span v-if="today.sunset">日落 {{ today.sunset }}</span>
            </div>
        </div>

        <!-- 预警条 -->
        <div v-if="alerts.length > 0" class="weather-alerts">
            <div v-for="alert in alerts" :key="alert.alertId" class="weather-alert-item" :class="'level-' + alert.level.toLowerCase()">
                <span class="weather-alert-level">{{ alert.levelText }}</span>
                <span class="weather-alert-type">{{ alert.type }}</span>
            </div>
        </div>

        <div class="weather-divider"></div>

        <!-- 预警阈值 -->
        <div class="weather-setting-row">
            <span class="weather-setting-label">预警阈值</span>
            <select class="weather-setting-select" v-model="alertThreshold" @change="onAlertThresholdChange">
                <option value="B">蓝色及以上</option>
                <option value="Y">黄色及以上</option>
                <option value="O">橙色及以上</option>
                <option value="R">红色及以上</option>
            </select>
        </div>

        <!-- 低等级预警轻提示（联动开关） -->
        <div v-if="alertThreshold !== 'B'" class="weather-setting-row">
            <span class="weather-setting-label">低等级预警轻提示</span>
            <label class="custom-switch mini">
                <input type="checkbox" v-model="lightAlertEnabled" @change="onLightAlertChange">
                <span class="slider"></span>
            </label>
        </div>

        <!-- 拉取间隔 -->
        <div class="weather-setting-row">
            <span class="weather-setting-label">拉取间隔</span>
            <select class="weather-setting-select" v-model="pollInterval" @change="onPollIntervalChange">
                <option :value="1800">0.5 小时</option>
                <option :value="3600">1 小时</option>
                <option :value="5400">1.5 小时</option>
                <option :value="7200">2 小时</option>
                <option :value="9000">2.5 小时</option>
                <option :value="10800">3 小时</option>
            </select>
        </div>
        <div v-if="pollInterval > 3600" class="weather-setting-hint">
            间隔过长，会影响实时恶劣天气的准确性
        </div>

        <div class="weather-divider"></div>

        <!-- 早/午/晚报开关 -->
        <div class="weather-setting-row">
            <span class="weather-setting-label">早报</span>
            <label class="custom-switch mini">
                <input type="checkbox" v-model="morningBrief" @change="onBriefChange">
                <span class="slider"></span>
            </label>
        </div>
        <div class="weather-setting-row">
            <span class="weather-setting-label">午报</span>
            <label class="custom-switch mini">
                <input type="checkbox" v-model="noonBrief" @change="onBriefChange">
                <span class="slider"></span>
            </label>
        </div>
        <div class="weather-setting-row">
            <span class="weather-setting-label">晚报</span>
            <label class="custom-switch mini">
                <input type="checkbox" v-model="eveningBrief" @change="onBriefChange">
                <span class="slider"></span>
            </label>
        </div>

        <div class="weather-divider"></div>

        <!-- 数据来源 -->
        <div class="weather-source">数据：小米天气</div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSettingRaw, setSettingRaw } from '../../utils/settings';
import {
    NSD_WEATHER_CITY,
    NSD_WEATHER_DAILY_BRIEF,
    NSD_WEATHER_ALERT_THRESHOLD,
    NSD_WEATHER_POLL_INTERVAL,
    NSD_WEATHER_LIGHT_ALERT_ENABLED,
} from '../../constants/storageKeys';
import type { WeatherCity, WeatherCurrent, WeatherDaily, WeatherAlert } from '../../composables/useWeather';

const props = defineProps<{
    city: WeatherCity | null;
    current: WeatherCurrent | null;
    today: WeatherDaily | null;
    tomorrow: WeatherDaily | null;
    alerts: WeatherAlert[];
}>();

const emit = defineEmits<{
    (e: 'city-change', city: WeatherCity): void;
}>();

// 城市搜索
const showCitySearch = ref(false);
const searchKeyword = ref('');
const searchResults = ref<WeatherCity[]>([]);
let searchDebounceTimer: number | null = null;

const cityName = computed(() => props.city?.name || '');

function toggleCitySearch() {
    showCitySearch.value = !showCitySearch.value;
    if (showCitySearch.value) {
        searchKeyword.value = '';
        searchResults.value = [];
    }
}

function onSearchInput() {
    if (searchDebounceTimer) {
        clearTimeout(searchDebounceTimer);
    }
    searchDebounceTimer = window.setTimeout(async () => {
        if (searchKeyword.value.trim().length === 0) {
            searchResults.value = [];
            return;
        }
        try {
            searchResults.value = await invoke<WeatherCity[]>('weather_search_city', { kw: searchKeyword.value.trim() });
        } catch (_e) {
            searchResults.value = [];
        }
    }, 350);
}

function selectCity(city: WeatherCity) {
    showCitySearch.value = false;
    searchKeyword.value = '';
    searchResults.value = [];
    setSettingRaw(NSD_WEATHER_CITY, JSON.stringify(city));
    emit('city-change', city);
}

// 预警阈值
const alertThreshold = ref(getSettingRaw(NSD_WEATHER_ALERT_THRESHOLD) || 'B');

function onAlertThresholdChange() {
    setSettingRaw(NSD_WEATHER_ALERT_THRESHOLD, alertThreshold.value);
    invoke('weather_set_alert_threshold', { level: alertThreshold.value }).catch(() => {});
}

// 低等级预警轻提示
const lightAlertEnabled = ref(getSettingRaw(NSD_WEATHER_LIGHT_ALERT_ENABLED) === 'true');

function onLightAlertChange() {
    setSettingRaw(NSD_WEATHER_LIGHT_ALERT_ENABLED, String(lightAlertEnabled.value));
    invoke('weather_set_light_alert_enabled', { enabled: lightAlertEnabled.value }).catch(() => {});
}

// 拉取间隔
const pollIntervalRef = ref(Number(getSettingRaw(NSD_WEATHER_POLL_INTERVAL) || '3600'));

function onPollIntervalChange() {
    setSettingRaw(NSD_WEATHER_POLL_INTERVAL, String(pollIntervalRef.value));
    invoke('weather_set_poll_interval', { secs: pollIntervalRef.value }).catch(() => {});
}

// 早/午/晚报
const briefValue = getSettingRaw(NSD_WEATHER_DAILY_BRIEF) || '';
const morningBrief = ref(briefValue.includes('morning'));
const noonBrief = ref(briefValue.includes('noon'));
const eveningBrief = ref(briefValue.includes('evening'));

function onBriefChange() {
    const briefs: string[] = [];
    if (morningBrief.value) briefs.push('morning');
    if (noonBrief.value) briefs.push('noon');
    if (eveningBrief.value) briefs.push('evening');
    const value = briefs.join(',');
    setSettingRaw(NSD_WEATHER_DAILY_BRIEF, value);
    invoke('weather_set_daily_brief', { brief: value }).catch(() => {});
}

onMounted(() => {
    // 初始化
});
</script>

<style scoped>
.weather-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    min-width: 220px;
    max-width: 280px;
}

.weather-city-row {
    position: relative;
}

.weather-city-selector {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--control-bg, rgba(255,255,255,0.06));
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    cursor: pointer;
    transition: all 0.2s ease;
}

.weather-city-selector:hover {
    border-color: var(--accent-color, #0ea5e9);
}

.weather-city-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--item-title-color, #fff);
}

.weather-city-arrow {
    width: 16px;
    height: 16px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.weather-city-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background: var(--card-bg, #1a1a1a);
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    border-radius: 8px;
    z-index: 100;
    max-height: 200px;
    overflow-y: auto;
}

.weather-city-search {
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: transparent;
    color: var(--item-title-color, #fff);
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
}

.weather-city-results {
    display: flex;
    flex-direction: column;
}

.weather-city-result {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    cursor: pointer;
    transition: background 0.2s ease;
}

.weather-city-result:hover {
    background: var(--control-bg, rgba(255,255,255,0.06));
}

.weather-city-result-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--item-title-color, #fff);
}

.weather-city-result-province {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.weather-city-no-result {
    padding: 8px 12px;
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
    text-align: center;
}

.weather-divider {
    height: 1px;
    background: var(--control-border, rgba(255,255,255,0.08));
}

.weather-current {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.weather-current-main {
    display: flex;
    align-items: baseline;
    gap: 8px;
}

.weather-current-temp {
    font-size: 28px;
    font-weight: 700;
    color: var(--item-title-color, #fff);
}

.weather-current-text {
    font-size: 14px;
    color: var(--item-desc-color, rgba(255,255,255,0.6));
}

.weather-current-feels {
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.weather-current-details {
    display: flex;
    gap: 12px;
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.weather-today {
    display: flex;
    align-items: center;
    gap: 12px;
}

.weather-today-temp {
    font-size: 14px;
    font-weight: 600;
    color: var(--item-title-color, #fff);
}

.weather-today-details {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
}

.weather-alerts {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.weather-alert-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
}

.weather-alert-item.level-b,
.weather-alert-item.level-blue {
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
}

.weather-alert-item.level-y,
.weather-alert-item.level-yellow {
    background: rgba(234, 179, 8, 0.1);
    color: #eab308;
}

.weather-alert-item.level-o,
.weather-alert-item.level-orange {
    background: rgba(249, 115, 22, 0.1);
    color: #f97316;
}

.weather-alert-item.level-r,
.weather-alert-item.level-red {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
}

.weather-alert-level {
    font-weight: 600;
}

.weather-alert-type {
    opacity: 0.8;
}

.weather-setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
}

.weather-setting-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--item-title-color, #fff);
}

.weather-setting-select {
    padding: 4px 8px;
    border-radius: 4px;
    border: 1px solid var(--control-border, rgba(255,255,255,0.15));
    background: var(--input-bg, rgba(255,255,255,0.08));
    color: var(--item-title-color, #fff);
    font-size: 11px;
    outline: none;
    cursor: pointer;
}

.weather-setting-select:focus {
    border-color: var(--accent-color, #0ea5e9);
}

.weather-setting-hint {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
    padding-left: 4px;
}

.weather-source {
    font-size: 10px;
    color: var(--item-desc-color, rgba(255,255,255,0.5));
    text-align: center;
}

/* Switch */
.custom-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
}

.custom-switch.mini {
    width: 36px;
    height: 20px;
}

.custom-switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.custom-switch .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--control-border, #e2e8f0);
    border-radius: 24px;
    transition: background-color 0.3s ease;
}

.custom-switch .slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: #ffffff;
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.custom-switch.mini .slider:before {
    height: 14px;
    width: 14px;
    left: 3px;
    bottom: 3px;
}

.custom-switch input:checked+.slider {
    background-color: var(--accent-color, #10b981);
}

.custom-switch input:checked+.slider:before {
    transform: translateX(20px);
}

.custom-switch.mini input:checked+.slider:before {
    transform: translateX(16px);
}

.custom-switch input:disabled+.slider {
    cursor: not-allowed;
    opacity: 0.6;
}

.custom-switch input:disabled {
    cursor: not-allowed;
}
</style>
