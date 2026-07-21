# 复刻上游「灵动岛个性化」+「控制台布局优化」实施计划

## 0. 结论先行

上游这两个新功能本质是**「一套设置中心 UI + 布局承载」**。MDI 分支已具备大部分骨架（布局三视图切换、灵动岛颜色、置顶 API、`start_island_animation` 动画命令、`emit`/`listen` 事件总线），因此**不是从零抄代码，而是把 MDI 现写死的东西改成可配置 + 新增一个 `PersonalizeCenter` 视图**。

- **后端 `src-tauri/src/lib.rs` 零改动**：弹簧是纯前端入场动画，尺寸/圆角/缩放/置顶全在前端落地，沿用事件总线、无新增命令。
- **风险等级：低~中**，改动集中在 `WidgetIsland.vue` 单文件 + 一个新组件。

### 用户已拍板的 4 个决策
| 决策点 | 结论 |
|---|---|
| 承载位置 | **新增独立视图 `personalize`**，加入 `main→island→live→personalize` 循环 |
| 弹簧范围 | **纯前端入场动画**（onEnter 分支），不动 Rust |
| 事件风格 | **单个打包事件 `sync-dynamic-settings`**（贴上游） |
| 高度模型 | **保留 MDI 各状态高度（34/38/42）**，砍掉上游 baseHeight 滑块，只做宽度/缩放可配 |

---

## 1. 要新增的子功能（MDI 缺口）

MDI 已有：灵动岛颜色（`control-island-theme`）、置顶 API、布局切换、动画命令。**本次需新增**：

1. 物理弹簧 快速/果味（stiff/bouncy）—— onEnter 参数分支
2. 边缘圆角 经典胶囊(100)/圆角矩形(12)
3. 始终置顶开关（复用 `setAlwaysOnTop`）
4. 尺寸/缩放滑块（**不含高度**）：
   - 常规宽度 baseWidth（默认 150）
   - 媒体常态宽度 musicBaseWidth（默认 260）
   - 媒体展开宽度 musicExpandedWidth（默认 320）
   - 消息展开宽度 msgExpandedWidth（默认 360）
   - 全局缩放 appScale（默认 1.0）

> 上游「灵动岛颜色」MDI 已有，**不重复**加入个性化中心（避免与设置网格重复）。

---

## 2. 数据流（总架构）

```
PersonalizeCenter.vue  ──写──> localStorage (NSD_* 常量)
        │
        └─emit('sync-dynamic-settings', {springStyle, borderRadius,
                isAlwaysOnTop, baseWidth, musicBaseWidth,
                musicExpandedWidth, msgExpandedWidth, appScale})
                              │
                              ▼
              WidgetIsland.vue  onMounted 里 listen(...)
                              │
        ┌──────────┬──────────┬──────────┬──────────┬──────────┐
        ▼          ▼          ▼          ▼          ▼          ▼
   onEnter 弹簧  islandStyle  setAlways  getBaseSize 展开宽度   html.zoom
   stiff/bouncy  圆角切换     OnTop      宽度        320/360    appScale
```

---

## 3. 分文件改动清单

### A. `src/constants/storageKeys.ts`（追加常量，外观组 L6-10 后）
遵循 `export const NSD_XXX = 'nsd_xxx';` 约定，新增 8 个：
- `NSD_SPRING_STYLE = 'nsd_spring_style'`
- `NSD_BORDER_RADIUS = 'nsd_border_radius'`
- `NSD_ALWAYS_ON_TOP = 'nsd_always_on_top'`
- `NSD_BASE_WIDTH = 'nsd_base_width'`
- `NSD_MUSIC_BASE_WIDTH = 'nsd_music_base_width'`
- `NSD_MUSIC_EXPANDED_WIDTH = 'nsd_music_expanded_width'`
- `NSD_MSG_EXPANDED_WIDTH = 'nsd_msg_expanded_width'`
- `NSD_APP_SCALE = 'nsd_app_scale'`

### B. 新建 `src/components/PersonalizeCenter.vue`
- 参照 `Original/src/components/DynamicSet.vue` 的卡片式布局（上部 3 宫格 + 下部滑块列表），但：
  - **移除** baseHeight 滑块与「灵动岛颜色」项
  - 文案**中文硬编码**（MDI 无 i18n 框架）
  - 存储一律用 storageKeys 常量，不搬裸 `nsd_` 字符串
- 控件与默认值（= 上游默认 = MDI 现状，保证向后兼容）：
  - 弹簧选择器：`stiff`/`bouncy`（默认 `bouncy`）
  - 圆角切换：`100`/`12`（默认 `100`）
  - 置顶 switch：默认 `true`
  - 滑块（各带复位钮）：baseWidth 140–300/150；musicBaseWidth 200–400/260；musicExpandedWidth 260–480/320；msgExpandedWidth 300–600/360；appScale 1.0–1.75 step0.25/1.0
- 单个 `watch` 监听全部值 → 写 localStorage + `emit('sync-dynamic-settings', {...})`

### C. `src/views/MainPanel.vue`（接入新视图，4 处）
- L337 `currentView` 联合类型加 `'personalize'`
- L340-344 `toggleDynamicSet` 循环加 personalize（main→island→live→personalize→main）
- L31 按钮文案/图标适配 4 态循环
- L248 后加 `v-else-if="currentView==='personalize'"` 渲染 `<PersonalizeCenter />` + import
- **布局 CSS 零改动**（`.dynamicset-layout` 已对所有非 main 视图生效）

### D. `src/views/WidgetIsland.vue`（消费端，核心）
1. 引入新常量，初始化各 ref（从 localStorage 读，默认值同上）。
2. onMounted 注册 `listen('sync-dynamic-settings', ...)`（仿现有 `control-island-theme` 监听 L2737 区块），更新各 ref 并触发对应应用。
3. **弹簧**：`onEnter`（L1942-1972）硬编码 `freq/decay/duration` 改为按 `springStyle` 分支：
   - stiff：`freq 3.2 / decay 18 / dur 350`
   - bouncy：`freq 2.0 / decay 10.5 / dur 600`（现状）
4. **圆角**：`islandStyle`（L1044）与 `coreContentStyle`（L1055）的**收起态**圆角由写死 `100px/98px` 改读 `borderRadius`（100→100px/98px；12→12px/10px）；**展开态保持 24px/22px**。
5. **置顶**：启动时按 `isAlwaysOnTop` 调 `getCurrentWindow().setAlwaysOnTop(...)`；listen 变化时同步（复用现有 L3171 调用点）。
6. **宽度**（高度一律不变）：
   - `getBaseSize()`（L1352）：速度态 `w:150→baseWidth`；音乐态 `w:260→musicBaseWidth`；实时活动 `{250,38}` 保持不动。
   - 音乐展开宽度：L2591 调用点 `320→musicExpandedWidth`（高 150 不变）。
   - 消息展开宽度：L3142 调用点 `360→msgExpandedWidth`（高 65 不变）。
   - 宽度变更后，在非展开/非 toast 时触发 `animateIslandSize` 重新形变。
7. **缩放**：`appScale → document.documentElement.style.zoom`；启动时应用 + listen 更新。
8. 未配置任何新键时，行为与当前完全一致（向后兼容）。

### E. 后端 `src-tauri/src/lib.rs`
- **不改动**。
- 备注：L682-705 的 `DWMWCP_DONOTROUND` 是 OS 级窗口圆角，与本功能的 CSS 圆角无关，无需动。

---

## 4. 默认值与向后兼容

所有新键默认值 = 上游默认 = MDI 现状（150/260/320/360/1.0/100/bouncy/true）。用户未进个性化中心前，灵动岛行为与当前**完全相同**。

---

## 5. 实施顺序（建议每步一提交）
1. storageKeys 常量
2. `PersonalizeCenter.vue` 新组件
3. `MainPanel.vue` 视图接入
4. `WidgetIsland.vue` 消费端（弹簧/圆角/置顶/宽度/缩放）
5. 构建 + 类型校验：`vue-tsc --noEmit && vite build`，多状态自测（待机/音乐/展开/消息/实时活动）

---

## 6. 风险与注意点
- `getBaseSize()` 有 15+ 调用点、展开宽度改动点分散，但都集中在 `WidgetIsland.vue` 单文件，需回归各状态尺寸。
- **勿覆盖** MDI 现有 `DynamicSet.vue`（与上游同名但完全不同组件）。
- 命名统一走 storageKeys 常量。
- `appScale` 用 `zoom` 时注意对灵动岛内百分比/fixed 布局的影响，需多状态自测。
- 版本号：属新功能（次版本级），但当前 **0.4.0 测试未结束**，并入 0.4.0-x 还是另起版本由你定（遵循 `docs/NAMING_AND_VERSION.md`）。

---

## 7. 关键文件
| 文件 | 动作 |
|---|---|
| `NetSpeed-Dynamic/src/constants/storageKeys.ts` | 追加 8 常量 |
| `NetSpeed-Dynamic/src/components/PersonalizeCenter.vue` | **新建** |
| `NetSpeed-Dynamic/src/views/MainPanel.vue` | 接入 personalize 视图 |
| `NetSpeed-Dynamic/src/views/WidgetIsland.vue` | 消费端：弹簧/圆角/置顶/宽度/缩放 |
| `NetSpeed-Dynamic/src-tauri/src/lib.rs` | 不改动 |
| 参考：`Original/src/components/DynamicSet.vue` | 布局/默认值蓝本（勿直接覆盖） |
