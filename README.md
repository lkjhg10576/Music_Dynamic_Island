# NetSpeed Dynamic (NSD)

一款基于 **Tauri 2 + Vue 3** 构建的桌面动态网速监控工具，提供灵动岛风格的悬浮窗实时显示网络速度，并支持流量数据统计分析与灵动岛个性化设置。

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
- **PRO 功能预告** — 内置音乐控制器、消息通知接收（微信/QQ）、系统硬件监控等未来功能占位项（标记 PRO，当前不可用）

### 设置与系统集成

- **主题切换** — 支持浅色模式 / 深色模式 / 跟随系统，全局 CSS 变量驱动，设置持久化到 localStorage
- **悬浮窗透明度可调** — 滑块调节灵动岛背景不透明度（0%~100%），实时同步至悬浮窗
- **开机自动启动** — 通过 `tauri-plugin-autostart` 实现跟随系统启动 NSD，静默启动时主窗口默认隐藏
- **系统托盘集成** — 托盘左键唤起主控制台，右键菜单强制退出；关闭主窗口时隐藏至托盘而非退出
- **单实例保证** — 通过 `tauri-plugin-single-instance` 防止重复启动多个进程
- **检查更新** — 通过 GitHub Releases API 检测新版本并引导下载
- **Windows 原生优化** — DWM 圆角裁剪去除、边框隐藏、无边框拖拽区域、Caption 样式移除
- **DPI 缩放适配** — 灵动岛窗口自动获取显示器缩放因子（scaleFactor），精确定位到屏幕顶部中央
- **模态对话框系统** — 统一的确认/提示弹窗组件，支持回调确认机制
- **灵动岛右键菜单** — 支持重置位置（回屏幕顶部居中）和关闭悬浮窗操作

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
| Windows API | windows-sys (DWM / GDI / Messaging) |

## 项目结构

```
NetSpeed-Dynamic/
├── src/                          # 前端源码
│   ├── main.ts                   # 应用入口，挂载 Vue + Router
│   ├── App.vue                   # 根组件（router-view）
│   ├── router/index.ts           # 路由配置：/ → MainPanel, /widget → WidgetIsland
│   ├── views/
│   │   ├── MainPanel.vue         # 主控制台面板（网速仪表盘 + 设置 + 图表 + 数据统计 + 灵动岛设置）
│   │   └── WidgetIsland.vue      # 灵动岛悬浮窗组件（网速显示 + 动画 + 右键菜单 + DPI适配 + 主题色）
│   └── assets/
│       ├── logo.png              # 应用 Logo
│       └── screenshot.png        # 效果截图
├── src-tauri/                    # Tauri 后端（Rust）
│   ├── src/
│   │   ├── main.rs               # Rust 入口
│   │   └── lib.rs                # 核心逻辑：Tauri 命令、托盘、窗口管理、Windows DWM 样式
│   ├── Cargo.toml                # Rust 依赖声明
│   ├── tauri.conf.json           # Tauri 配置（双窗口：main + widget）
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
| 主控制台 | `main` | 700×550px，不可调整大小 | 网速总览、设置面板、图表展示、数据统计、灵动岛设置 |
| 灵动岛 Widget | `widget` | 210×36px（运行时 260×42px），无边框透明、置顶、不在任务栏显示 | 实时网速悬浮条 |

两个窗口通过 **Tauri Event** 进行通信：
- `control-island-visibility` — 控制台 → 灵动岛显隐指令
- `control-island-opacity` — 控制台 → 灵动岛透明度同步
- `control-island-theme` — 控制台 → 灵动岛主题色（暗色/亮色）同步
- `island-status-sync` — 灵动岛 → 控制台状态回传

### 后端命令（Tauri Commands）

| 命令 | 说明 |
|------|------|
| `get_network_stats` | 通过 `sysinfo::Networks` 获取所有网卡累计收发字节数，前端做差分计算瞬时速度 |
| `get_network_latency` | TCP 连接 `223.5.5.5:53`（阿里 DNS）测量网络延迟，超时 1.5s |
| `is_widget_visible` | 查询灵动岛窗口当前可见性 |

### 灵动岛动画

入场/退场动画使用 JavaScript `requestAnimationFrame` 实现，公式源自 After Effects 弹簧表达式转换：

```
scale = 1 - cos(2πft) × e^(-dt)    // f=2.0, d=10.5, duration=600ms
```

退场动画完成后才触发 Tauri 窗口隐藏，确保视觉连贯。

### 灵动岛主题色机制

灵动岛支持暗色/亮色两套主题，通过胶囊式开关切换：

1. 用户在「灵动岛设置面板」中选择暗色或亮色主题
2. 主题值写入 localStorage（`nsd_island_theme`）
3. 通过 `control-island-theme` 事件实时推送至 WidgetIsland 组件
4. WidgetIsland 根据主题值计算内联样式：
   - **暗色**: `rgba(0, 0, 0, α)` 背景 + 白色文字
   - **亮色**: `rgba(255, 255, 255, α)` 背景 + 黑色文字

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
   - **右键菜单** — 重置位置 / 关闭悬浮窗
5. 在控制台中可调整**主题**和悬浮窗**透明度**（透明度实时同步至悬浮窗，0% 时文字自动变深色）
6. 点击顶部 **「灵动岛设置」按钮** 进入灵动岛专属设置面板：
   - **灵动岛颜色** — 切换暗色/亮色主题，实时生效并持久化
   - **PRO 功能预览** — 音乐控制器、消息通知接收、系统硬件监控（即将推出）
7. 控制台右侧面板支持在**常规设置**与**数据统计**之间切换：
   - **常规设置**：显示模式、开机自启、悬浮窗不透明度调节
   - **数据统计**：总上传/下载、本月流量、近 7 天流量趋势图（支持柱状图/折线图切换）
8. 点击底部 **检查更新** 链接可通过 GitHub Releases API 检测新版本

## 开源协议

本项目基于 [MIT License](./LICENSE) 开源。

Copyright (c) 2026 Ryen (GEORGEWU)
