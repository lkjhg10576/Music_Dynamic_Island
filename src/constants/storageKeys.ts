/**
 * localStorage 键名集中管理
 * 避免散落在各处的魔法字符串，防止拼写错误难以排查
 */

// 灵动岛外观
export const NSD_ISLAND_OPACITY = 'nsd_island_opacity';
export const NSD_ISLAND_THEME = 'nsd_island_theme';
export const NSD_ISLAND_WIDTH = 'nsd_island_width';
export const NSD_ISLAND_POSITION = 'nsd_island_position';

// 音乐频谱颜色
export const NSD_SPECTRUM_COLOR_MODE = 'nsd_spectrum_color_mode'; // 'album' | 'theme' | 'custom'
export const NSD_SPECTRUM_CUSTOM_COLOR = 'nsd_spectrum_custom_color'; // hex, 默认 '#b6e0ee'

// 功能开关
export const NSD_MUSIC_CTRL = 'nsd_music_ctrl';
export const NSD_GLOW_BORDER = 'nsd_glow_border';
export const NSD_MSG_NOTIFY = 'nsd_msg_notify';
export const NSD_MSG_MODE = 'nsd_msg_mode';
export const NSD_ROTATION_MODE = 'nsd_rotation_mode';
export const NSD_PIN_TASKBAR = 'nsd_pin_taskbar';
export const NSD_POSITION_LOCKED = 'nsd_position_locked';
export const NSD_DESTROY_ON_CLOSE = 'nsd_destroy_on_close';

// 自动隐藏
export const NSD_AUTO_HIDE_ENABLED = 'nsd_auto_hide_enabled';
export const NSD_AUTO_HIDE_DELAY = 'nsd_auto_hide_delay';

// 自动折叠
export const NSD_AUTO_COLLAPSE_ENABLED = 'nsd_auto_collapse_enabled';
export const NSD_AUTO_COLLAPSE_DELAY = 'nsd_auto_collapse_delay';

// 全屏自动隐藏
export const NSD_AUTO_HIDE_FS = 'nsd_autohide_fs';

// 番茄钟
export const NSD_POMODORO_ACTIVE = 'nsd_pomodoro_active';
export const NSD_POMODORO_STARTED = 'nsd_pomodoro_started';
export const NSD_POMODORO_VISIBLE = 'nsd_pomodoro_visible';
export const NSD_POMODORO_TIME = 'nsd_pomodoro_time';
export const NSD_POMODORO_FOCUS_SECS = 'nsd_pomodoro_focus_secs';
export const NSD_POMODORO_BREAK_SECS = 'nsd_pomodoro_break_secs';
export const NSD_POMODORO_CYCLES = 'nsd_pomodoro_cycles';

// 倒计时
export const NSD_COUNTDOWN_VISIBLE = 'nsd_countdown_visible';
export const NSD_COUNTDOWN_SECS = 'nsd_countdown_secs';

// 控制台
export const NSD_THEME_MODE = 'nsd_theme_mode';
export const NSD_TARGET_PLAYER = 'nsd_target_player';
export const NSD_TRAFFIC_STATS = 'nsd_traffic_stats';
export const NSD_CHART_METRIC = 'nsd_chart_metric'; // 实时状态下拉选择：speed | cpu | ram
export const NSD_CHECK_BETA = 'nsd_check_beta'; // 是否检测 Pre-release / Beta 版本

// 硬件监控配置（后端推送模式下）
export const NSD_HW_ENABLED = 'nsd_hw_enabled';
export const NSD_HW_MODE = 'nsd_hw_mode'; // 'dual' | 'rotation' | 'single'
export const NSD_HW_DEFAULT_METRIC = 'nsd_hw_default_metric'; // 'cpu' | 'mem'
export const NSD_HW_ROTATION = 'nsd_hw_rotation'; // 轮换模式开关（兼容旧名）
export const NSD_HW_DUAL_RING = 'nsd_hw_dual_ring'; // 双圆环模式开关（兼容旧名）

// 健康提醒
export const NSD_SITTING_REMINDER_ENABLED = 'nsd_sitting_reminder_enabled';
export const NSD_SITTING_REMINDER_SECS = 'nsd_sitting_reminder_secs';
export const NSD_WATER_REMINDER_ENABLED = 'nsd_water_reminder_enabled';
export const NSD_WATER_REMINDER_SECS = 'nsd_water_reminder_secs';

// 实时活动优先级映射（多活动并行时的显示顺序）。值示例: {"pomodoro":1,"countdown":2,"hardware":3,"health":4}
export const NSD_ACTIVITY_PRIORITY = 'nsd_activity_priority';

// 系统动态感知配置
export const NSD_SYSMSG_ENABLED = 'nsd_sysmsg_enabled';
export const NSD_SYSMSG_VOLUME_ENABLED = 'nsd_sysmsg_volume_enabled';
export const NSD_SYSMSG_POWER_ENABLED = 'nsd_sysmsg_power_enabled';
export const NSD_SYSMSG_BATTERY_ENABLED = 'nsd_sysmsg_battery_enabled';
export const NSD_SYSMSG_UNLOCK_ENABLED = 'nsd_sysmsg_unlock_enabled';
