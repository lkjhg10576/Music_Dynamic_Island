# NetSpeed Dynamic Pro (NSD)

一款基于 **Tauri 2 + Vue 3** 构建的桌面动态网速监控工具，提供灵动岛风格的悬浮窗实时显示网络速度，并支持流量数据统计分析、灵动岛个性化设置、网易云音乐控制、系统消息通知接收与系统硬件监控。

**音乐控制器**
![NetSpeed Dynamic 效果图](./src/assets/screenshot2.png)
![NetSpeed Dynamic 效果图](./src/assets/screenshot3.png)
**灵动岛通知**
![NetSpeed Dynamic 效果图](./src/assets/screenshot4.jpg)
**系统资源监控**
![NetSpeed Dynamic 效果图](./src/assets/screenshot5.png)
![NetSpeed Dynamic 效果图](./src/assets/screenshot6.png)
**标准网速显示**
![NetSpeed Dynamic 效果图](./src/assets/screenshot.png)

## 功能特性

### 网速监控

- **实时网速监控** — 每秒刷新上传/下载速度，支持 B/s、KB/s、MB/s 自动单位切换
- **灵动岛悬浮窗** — 仿 macOS Dynamic Island 设计的透明悬浮窗，支持拖拽移动、弹簧动画进出场
- **网络状态指示灯** — 绿色（延迟 <150ms）/ 黄色（高延迟或大流量遮挡）/ 红色（断网），带智能防抖逻辑避免误判
- **流量高亮提示** — 当流量超过 1MB/s 时，悬浮窗箭头自动高亮提醒
- **速度趋势图表** — 控制台内置 ECharts 迷你折线图，展示最近 15 秒下载速度走势
- **透明度自适应文字颜色** — 当悬浮窗不透明度调至 0% 时，文字自动切换为深色以保持可读性

### 数据统计面板

- **流量数据持久化** — 每次采样自动累计上传/下载流量，数据存储于 `localStorage`（`nsd_traffic_stats`）
- **总流量概览** — 实时展示累计总上传量、总下载量
- **本月流量统计** — 自动按月汇总当月的上传+下载总流量
- **近 7 天流量图表** — ECharts 驱动的双系列图表，同时展示每日上传/下载流量（单位 MB）
- **图表类型切换** — 支持柱状图 / 折线图一键切换，适应不同查看偏好
- **设置/统计面板切换** — 控制台右侧面板可在「常规设置」与「数据统计」之间自由切换

### 灵动岛设置面板 (DynamicSet)

- **独立设置入口** — 点击顶部「灵动岛设置」按钮进入专属设置面板，采用双列网格布局
- **灵动岛主题色切换** — 胶囊式开关支持暗色（黑底白字）/ 亮色（白底黑字）两种主题，设置持久化到 `localStorage`（`nsd_island_theme`）并实时同步至悬浮窗
- **网易云音乐控制器** — 内置音乐控制功能，支持在灵动岛内直接操控网易云音乐播放：
  - **播放/暂停 / 上一首 / 下一首** — 通过 Windows 全局多媒体按键（`keybd_event`）发送系统级媒体控制指令
  - **歌曲信息显示** — 实时获取当前播放的歌名与歌手名称，展示于灵动岛内
  - **专辑封面旋转** — 播放状态时封面自动旋转动画，暂停时停止；未播放时显示渐变色默认背景
  - **多源封面获取** — 优先从网易云官方搜索 API 获取封面 → **Deezer Search API** → iTunes Search API 兜底 → 内联 Base64 SVG 渐变占位图保底，带 5 秒超时保护
  - **封面缓存机制** — 最多缓存 50 张封面避免重复请求，超限时自动清空重建
  - **智能交互** — 鼠标悬停音乐区域显示播放控件，离开后 800ms 自动切换为歌曲信息展示
  - **窗口标题捕获** — 通过 Windows `EnumWindows` API 枚举网易云进程窗口标题（匹配 `Orpheus` / `CloudMusic` 类名），解析 `歌名 - 歌手` 格式
  - **使用提示** — 需将网易云音乐保持运行（最小化即可），不可完全关闭窗口
- **彩虹流光边框 (NEW)** — 音乐控制器开启时，灵动岛自动启用彩虹渐变流光边框效果：
  - 基于 CSS `conic-gradient` 实现 8 色顺时针匀速旋转动画（3s/周期）
  - 通过 `backdrop-filter: blur(20px)` 高斯模糊裁切，仅露出边缘流光
  - 边框透明度随悬浮窗不透明度联动（gamma 校正映射）
  - 支持右键菜单手动开关（仅在音乐控制器启用状态下可用）
  - 关闭音乐控制器时流光边框同步关闭，开启时默认跟随启用
- **系统消息通知接收 (NEW)** — 通过 Windows 原生通知中心 API 实时捕获系统 Toast 通知并在灵动岛上展示：
  - **Windows Notification Listener** — 使用 `windows::UI::Notifications::Management::UserNotificationListener` 读取系统通知中心
  - **灵动岛动态扩展** — 收到新通知时，灵动岛从标准尺寸（260x42）弹性动画扩展至通知尺寸（360x65），展示应用图标 + 标题 + 内容
  - **智能过滤** — 自动过滤微信（WeChat）相关通知，避免干扰
  - **增量检测** — 以首次启动时的最新通知 ID 为基准线，仅推送启动后的新通知
  - **自动消失** — 通知在灵动岛上停留 5 秒后自动收回至原始尺寸
  - **多应用兼容** — 支持 QQ、钉钉、飞书等所有发出 Toast 通知的应用
  - **使用方式** — 在「灵动岛设置面板」中开启「消息通知接收」开关即可激活
- **PRO 功能预告** — 系统硬件监控（CPU/内存占用预警）等未来功能占位项（标记 PRO，当前不可用）

### 设置与系统集成

- **主题切换** — 支持浅色模式 / 深色模式 / 跟随系统，全局 CSS 变量驱动，设置持久化到 localStorage
- **悬浮窗透明度可调** — 滑块调节灵动岛背景不透明度（0%~100%），实时同步至悬浮窗
- **开机自动启动** — 通过 `tauri-plugin-autostart` 实现跟随系统启动 NSD，静默启动时主窗口默认隐藏
- **系统托盘集成** — 托盘左键唤起主控制台，右键菜单强制退出；关闭主窗口时隐藏至托盘而非退出
- **单实例保证** — 通过 `tauri-plugin-single-instance` 防止重复启动多个进程
- **检查更新** — 通过 GitHub Releases API 检测新版本并引导下载（含超时/网络错误精细化提示）
- **Windows 原生优化** — DWM 圆角裁剪去除、边框隐藏、无边框拖拽区域、Caption 样式移除
- **DPI 缩放适配** — 灵动岛窗口自动获取显示器缩放因子（scaleFactor），精确定位到屏幕顶部中央
- **模态对话框系统** — 统一的确认/提示弹窗组件，支持回调确认机制
- **灵动岛右键菜单** — 支持重置位置（回屏幕顶部居中）、开启/关闭流光边框和关闭悬浮窗操作

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri 2](https://tauri.app/) (Rust) |
| 前端框架 | [Vue 3](https://vuejs.org/) + TypeScript |
| 构建工具 | [Vite 6](https://vite.dev/) |
| 路由 | [Vue Router 5](https://router.vuejs.org/) |
| 图表 | [ECharts 6](https://echarts.apache.org/) |
| 图标 | [Lucide Vue Next](https://lucide.dev/) |
| 网络监控 | [sysinfo](https://docs.rs/sysinfo) (Rust) |
| 异步运行时 | [Tokio](https://tokio.rs/) (Rust) |
| HTTP 客户端 | [reqwest](https://docs.rs/reqwest) (Rust) |
| URL 编码 | [urlencoding](https://docs.rs/urlencoding) (Rust) |
| Windows API | windows-sys + winapi (DWM / GDI / Messaging / EnumWindows / keybd_event) |
| Windows 通知 | windows-rs (UI Notifications Management) |

## 项目结构

```
NetSpeed-Dynamic/
├── src/                          # 前端源码
│   ├── main.ts                   # 应用入口，挂载 Vue + Router
│   ├── App.vue                   # 根组件（router-view）
│   ├── router/index.ts           # 路由配置：/ → MainPanel, /widget → WidgetIsland
│   ├── views/
│   │   ├── MainPanel.vue         # 主控制台面板（网速仪表盘 + 设置 + 图表 + 数据统计 + 灵动岛设置 + 音乐/通知/硬件监控控制开关）
│   │   └── WidgetIsland.vue      # 灵动岛悬浮窗组件（网速显示 + 音乐控制器 + 消息通知 + 系统硬件监控 + 流光边框 + 动画 + 右键菜单 + DPI适配 + 主题色 + 动态尺寸）
│   └── assets/
│       ├── logo.png              # 应用 Logo
│       ├── screenshot.png        # 效果截图
│       ├── screenshot2.png       # 补充截图
│       ├── screenshot3.png       # 补充截图
│       ├── wechat-pay.png        # 微信收款码
│       └── alipay.jpg            # 支付宝收款码
├── src-tauri/                    # Tauri 后端（Rust）
│   ├── src/
│   │   ├── main.rs               # Rust 入口
│   │   └── lib.rs                # 核心逻辑：Tauri 命令、托盘、窗口管理、Windows DWM 样式、网易云音乐信息捕获、媒体控制、多源封面获取、系统通知监听、系统硬件监控数据采集
│   ├── Cargo.toml                # Rust 依赖声明（含 reqwest, urlencoding, winapi, windows-rs 等）
│   ├── tauri.conf.json           # Tauri 配置（双窗口：main + widget, v2.1.0）
│   └── icons/                    # 应用图标（全平台）
├── index.html                    # HTML 入口（含主题预加载脚本）
├── vite.config.ts                # Vite 配置（Tauri 开发模式适配）
├── package.json                  # 前端依赖与脚本
└── tsconfig.json                 # TypeScript 配置
```

## 架构说明

### 双窗口架构

应用采用 **双窗口** 设计，通过 Tauri 多窗口 + Vue Router 分发：

| 窗口 | 标签 | 尺寸 | 用途 |
|------|------|------|------|
| 主控制台 | `main` | 700x550px，不可调整大小 | 网速总览、设置面板、图表展示、数据统计、灵动岛设置、音乐/通知控制开关 |
| 灵动岛 Widget | `widget` | 210x36px（运行时 260x42px，通知模式 360x65px），无边框透明、置顶、不在任务栏显示 | 实时网速悬浮条 / 音乐控制器 / 消息通知展示 |

两个窗口通过 **Tauri Event** 进行通信：
- `control-island-visibility` — 控制台 → 灵动岛显隐指令
- `control-island-opacity` — 控制台 → 灵动岛透明度同步
- `control-island-theme` — 控制台 → 灵动岛主题色（暗色/亮色）同步
- `control-music-ctl` — 控制台 → 灵动岛音乐控制器显隐同步
- `island-status-sync` — 灵动岛 → 控制台状态回传

### 后端命令（Tauri Commands）

| 命令 | 说明 |
|------|------|
| `get_network_stats` | 通过 `sysinfo::Networks` 获取所有网卡累计收发字节数，前端做差分计算瞬时速度 |
| `get_network_latency` | TCP 连接 `223.5.5.5:53`（阿里 DNS）测量网络延迟，超时 1.5s |
| `is_widget_visible` | 查询灵动岛窗口当前可见性 |
| `fetch_netease_music_info` | 通过 Windows `EnumWindows` API 枚举网易云窗口标题，解析当前播放歌曲的「歌名 - 歌手」信息 |
| `control_system_media` | 通过 Windows `keybd_event` 发送全局多媒体按键（播放/暂停、上一首、下一首） |
| `get_random_cover_url` | 多源获取专辑封面 URL：网易云搜索 API → **Deezer Search API** → iTunes Search API → **Base64 SVG 渐变**兜底，内置 5 秒超时 |
| `fetch_latest_notification` | 通过 Windows `UserNotificationListener` 读取系统通知中心最新 Toast 通知，返回应用名和内容（自动过滤微信） |
| `get_hardware_stats` | 通过 `sysinfo::System` 获取 CPU 使用率（%）、已用内存（bytes）、总内存（bytes），前端计算内存占用百分比并智能模拟 GPU 趋势值 |

### 灵动岛动画

入场/退场动画使用 JavaScript `requestAnimationFrame` 实现，公式源自 After Effects 弹簧表达式转换：

```
scale = 1 - cos(2pi*ft) x e^(-dt)    // f=2.0, d=10.5, duration=600ms
```

退场动画完成后才触发 Tauri 窗口隐藏，确保视觉连贯。音乐控制箱入场额外带有 `translateY` 位移动画（20px -> 0），退场采用 150ms 快速淡出。

**灵动岛动态尺寸机制**：当收到系统消息通知时，灵动岛通过弹性动画从标准胶囊尺寸（260x42px）平滑扩展为通知展示尺寸（360x65px），通知消失后自动收回。内部子元素（网速/音乐/通知/硬件监控）的切换均采用独立的弹簧入场动画。灵动岛支持**四种显示模式**，按优先级从高到低依次为：系统通知 > 音乐控制器 > 系统硬件监控 > 网速监控。

### 灵动岛主题色机制

灵动岛支持暗色/亮色两套主题，通过胶囊式开关切换：

1. 用户在「灵动岛设置面板」中选择暗色或亮色主题
2. 主题值写入 localStorage（`nsd_island_theme`）
3. 通过 `control-island-theme` 事件实时推送至 WidgetIsland 组件
4. WidgetIsland 根据主题值计算内联样式：
   - **暗色**: `rgba(0, 0, 0, alpha)` 背景 + 白色文字
   - **亮色**: `rgba(255, 255, 255, alpha)` 背景 + 黑色文字

### 彩虹流光边框机制

流光边框是音乐控制器的视觉增强效果，与音乐控制器状态联动：

1. **启用条件**：音乐控制器开启时默认启用流光边框，关闭时同步关闭
2. **实现原理**：
   - 底层放置一个 500x500 的超大正方形 `div`，填充 `conic-gradient` 8 色彩虹渐变
   - CSS `animation: rotate 3s linear infinite` 驱动匀速旋转
   - 上层核心内容区通过 `border-radius: 98px` + `overflow: hidden` 裁切，只露出边缘一圈流光
   - 外层容器通过 `backdrop-filter: blur(20px)` 提供高斯模糊扩散效果
3. **透明度联动**：边框可见度 = 悬浮窗不透明度的 gamma 校正值（`alpha = pow(linear, 1/2.2)`），确保视觉协调
4. **用户控制**：右键菜单提供「开启/关闭流光边框」选项，仅在音乐控制器启用时可操作

### 音乐控制器机制

音乐控制器是灵动岛的内置模式，与网速显示互斥（同一时间只显示其一）：

1. **开启流程**：用户在「灵动岛设置面板」中打开音乐控制器开关 -> 状态写入 localStorage（`nsd_music_ctrl`）-> 通过 `control-music-ctl` 事件推送至 WidgetIsland -> 灵动岛从网速模式切换到音乐模式（带弹簧进场动画 + 流光边框）
2. **信息采集**：Rust 后端每秒调用 `fetch_netease_music_info`，通过 `EnumWindows` 遍历窗口查找网易云进程（匹配类名 `Orpheus` / `CloudMusic`），提取窗口标题并解析为歌名和歌手
3. **播放控制**：前端调用 `control_system_media` 命令，Rust 端通过 `keybd_event` 发送 `VK_MEDIA_PLAY_PAUSE` / `VK_MEDIA_NEXT_TRACK` / `VK_MEDIA_PREV_TRACK` 全局虚拟按键
4. **封面获取**：检测到歌曲变化时，依次尝试四个数据源：
   - 网易云官方搜索接口（`music.163.com/api/search/get/web`，POST 表单提交）— 最精准
   - **Deezer Search API**（`api.deezer.com/search`）— 新增！极其稳定，中文歌曲支持优秀，返回 cover_medium (250x250) / cover_big (500x500)
   - iTunes Search API（`itunes.apple.com/search`）— 稳定备用方案
   - **内联 Base64 SVG 渐变** — 新增终极兜底！不再依赖外部图片服务（Unsplash 在国内可能加载慢或被墙），使用蓝绿渐变的纯矢量 SVG 保证永远有图显示
5. **封面缓存**：成功获取的封面 URL 以 `歌名 - 歌手` 为 key 缓存，上限 50 条，超限自动清空
6. **交互逻辑**：鼠标进入音乐区域 -> 显示播放控件；鼠标离开 -> 800ms 后自动切回歌曲信息展示；点击播放/暂停时锁定状态同步 1500ms 避免轮询覆盖
7. **视觉反馈**：播放中封面持续旋转（CSS `is-playing` 动画）；未检测到网易云运行时显示「未在播放歌曲 - 网易云音乐」提示文字

### 系统消息通知机制

消息通知是灵动岛的第三种展示模式，与网速显示、音乐控制器互斥（同一时间只显示其一）：

1. **开启流程**：用户在「灵动岛设置面板」中开启「消息通知接收」开关 -> 状态写入 localStorage（`nsd_msg_notify`）
2. **通知采集**：Rust 后端每秒调用 `fetch_latest_notification`，通过 Windows Runtime API 操作系统通知中心：
   - 使用 `UserNotificationListener::Current()` 获取监听器实例
   - 调用 `RequestAccessAsync()` 请求通知读取权限
   - 通过 `GetNotificationsAsync(NotificationKinds::Toast)` 获取所有 Toast 类型通知
3. **增量检测**：首次启动时以当前最新通知 ID 为基准线（`LAST_NOTIFICATION_ID`），之后仅推送 ID 大于基准的新通知
4. **内容解析**：提取 Toast 通知的 Visual Binding 文本元素，`text[0]` 为应用名/标题，`text[1..]` 拼接为正文内容
5. **智能过滤**：自动识别并忽略微信（WeChat）相关通知，避免频繁的消息弹窗干扰
6. **灵动岛展示**：收到新通知时：
   - 触发 `animateIslandSize(360, 65)` 将灵动岛弹性扩展至通知尺寸
   - 显示应用图标头像 + 通知标题（14px 加粗）+ 通知正文（12.5px）
   - 5 秒后自动收回至原始胶囊尺寸（260x42）
7. **优先级**：通知 > 音乐 > 网速（即通知到来时优先展示通知内容）

### 系统硬件监控机制

系统硬件监控是灵动岛的第四种展示模式，与网速显示、音乐控制器、消息通知互斥（同一时间只显示其一）：

1. **开启流程**：用户在「灵动岛设置面板」中开启「系统硬件监控」开关 -> 状态写入 localStorage（`nsd_hardware_mon`）-> 通过事件推送至 WidgetIsland -> 灵动岛从当前模式切换到硬件监控模式（带弹簧进场动画）
2. **数据采集**：
   - Rust 后端每秒调用 `get_hardware_stats` 命令
   - 使用 `sysinfo::System` 统一管理系统状态
   - `refresh_cpu_usage()` 获取全局 CPU 使用率（返回 f32 浮点数）
   - `refresh_memory()` 刷新内存信息，返回已用内存和总内存（u64 字节数）
3. **前端处理**：
   - CPU 使用率：直接取整后显示为百分比
   - 内存占用率：`(used_memory / total_memory) * 100`，保留整数
   - GPU 占用率：采用智能模拟算法 —— 基于当前 CPU 使用率的 40% 作为基础值，叠加 0-5 的随机偏移，限制在 1%-99% 范围内。此方案避免引入额外的 GPU 监控依赖库（如 NVML / AMD ADL），同时提供视觉上的动态反馈
4. **高占用预警逻辑**：
   - 实时监听 CPU/GPU/RAM 数值
   - 当任意指标 ≥ 90% 时，为该数值元素动态添加 `.high-usage` CSS 类
   - 触发苹果标准警示红色（#f06861），带 0.3s 颜色过渡动画
   - 当数值回落至 90% 以下时自动移除警告样式
5. **UI 布局**：
   - 采用绝对定位的横向流式布局（`display: flex; align-items: center; justify-content: flex-start`）
   - 每个指标项包含：10px 小字号标签（opacity: 0.5）+ 13px 粗体数值 + 分隔线
   - 整体左对齐，右侧留出空间给网络状态指示灯
6. **模式切换记忆**：开启硬件监控时，若音乐控制器正在运行，系统会记录原状态；关闭硬件监控后可选择性恢复音乐控制器（通过 `wasMusicEnabledBeforeHardware` 变量实现）

### 数据统计机制

流量统计数据采用前端 localStorage 持久化方案：

1. **采集频率**：每秒调用 `get_network_stats` 获取网卡累计字节，做差分得到瞬时速度
2. **累计存储**：每次采样将上行/下行字节数按日期（YYYY-MM-DD）累加至 `trafficData` 对象
3. **节流写入**：每 5 次采样后批量写入 localStorage（`nsd_traffic_stats`），减少 I/O 开销
4. **可视化**：
   - 总上传/总下载 = 所有日期数据的求和
   - 本月流量 = 当前月份（YYYY-MM）所有日期的上行+下行总和
   - 近 7 天图表 = 取最近 7 天的每日数据，转换为 MB 单位后渲染为 ECharts 双系列图表

## 开发环境准备

### 前置依赖

- **Node.js** >= 18
- **Rust** >= 1.70（[安装指南](https://www.rust-lang.org/tools/install)）
- **Tauri 2 CLI**

### 安装与运行

```bash
# 1. 克隆项目
git clone https://github.com/GEORGEWWWU/NetSpeed-Dynamic.git
cd NetSpeed-Dynamic

# 2. 安装前端依赖
npm install

# 3. 启动开发模式（同时运行 Vite dev server 和 Tauri 窗口）
npm run tauri dev
```

### 构建发布版本

```bash
# 构建前端 + 编译 Rust + 打包安装包
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`。

## 使用方式

1. 启动应用后，默认仅显示**主控制台**窗口（开机自启时默认隐藏）
2. 点击系统托盘图标可随时唤起**主控制台**
3. 在控制台中开启 **Widget 开关**，屏幕顶部中央出现灵动岛悬浮窗
4. 灵动岛支持：
   - **左键拖拽** — 自由移动位置
   - **右键菜单** — 重置位置 / 开启或关闭流光边框 / 关闭悬浮窗
5. 在控制台中可调整**主题**和悬浮窗**透明度**（透明度实时同步至悬浮窗，0% 时文字自动变深色）
6. 点击顶部 **「灵动岛设置」按钮** 进入灵动岛专属设置面板：
   - **灵动岛颜色** — 切换暗色/亮色主题，实时生效并持久化
   - **音乐控制器** — 开启后灵动岛切换为音乐控制模式：
     - 显示正在播放的歌曲名称和歌手
     - 专辑封面播放时自动旋转
     - **彩虹流光边框** — 音乐模式下自动启用，8 色渐变匀速旋转的炫酷边框效果，可通过右键菜单独立开关
     - 支持上一首 / 播放暂停 / 下一首控制
     - 鼠标悬停显示控制按钮，离开后显示歌曲信息
     - *需保持网易云音乐运行（可最小化），不可关闭窗口*
   - **消息通知接收 (PRO)** — 开启后灵动岛将实时展示系统通知：
     - 自动捕获 QQ、钉钉、飞书等应用的 Toast 推送通知
     - 收到通知时灵动岛自动扩展并显示应用图标 + 标题 + 正文
     - 5 秒后自动收回原始尺寸
     - 自动过滤微信通知避免干扰
   - **系统硬件监控 (PRO)** — 开启后灵动岛将显示实时系统性能数据：
     - **CPU 占用率** — 实时显示全局 CPU 使用百分比（基于 sysinfo 精确采集）
     - **内存占用率** — 显示已用内存占总内存的百分比
     - **GPU 占用率** — 基于 CPU 趋势智能模拟的动态估算值（避免安装额外 GPU 驱动插件）
     - **高占用预警** — 当任意指标 ≥ 90% 时自动变为红色警示，带平滑颜色过渡动画
     - **紧凑布局** — 三列式横向设计完美适配灵动岛空间，支持暗色/亮色主题自适应
     - *注：开启硬件监控时会自动暂停音乐控制器（如有运行），关闭后可手动恢复*
   - **PRO 功能预览** — 系统硬件监控（即将推出）
7. 控制台右侧面板支持在**常规设置**与**数据统计**之间切换：
   - **常规设置**：显示模式、开机自启、悬浮窗不透明度调节
   - **数据统计**：总上传/下载、本月流量、近 7 天流量趋势图（支持柱状图/折线图切换）
8. 点击底部 **检查更新** 链接可通过 GitHub Releases API 检测新版本（含超时/网络错误精细化提示）

## 开源协议

本项目基于 [MIT License](./LICENSE) 开源。

Copyright (c) 2026 Ryen (GEORGEWU)

## 捐赠 / Sponsor

如果 NetSpeed Dynamic Pro 对你有帮助，欢迎请作者喝杯咖啡 ，你的支持是项目持续开发的动力！

| 方式 | 信息 |
|------|------|
| 微信支付 | ![微信](./src/assets/wechat-pay.png) |
| 支付宝 | ![支付宝](./src/assets/alipay.jpg) |
| GitHub Sponsors | [前往支持](https://github.com/sponsors/GEORGEWWWU) |

---

> 感谢每一位支持者！
