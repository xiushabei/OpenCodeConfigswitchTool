# 预设切换器 Design System

Windows 桌面工具（Tauri 2 + 原生 TypeScript/CSS）。视觉规范：Material Design 3 基线配色与组件语法，克制、紧凑、可长期使用。本文件是所有颜色、字号、间距、形状与组件状态的唯一来源；组件代码不得出现 `DESIGN.md` 之外的原始色值或魔法数值。

## 0. Research Log

- Embedded refs: 用户指定 Material Design 3 与 Material Symbols，覆盖 skill 默认的 Layer B 品牌技能与 shadcn/Lucide 组件建议，因此不加载品牌参考；执行纪律采用 frontend skill 的 operational brief（`taste-skill` lane 的克制原则）与 `layout-skill.md`（app shell / list-detail / 滚动归属）。
- MD3 tokens: 采用 Material Web `v0.192` 官方 `_md-sys-color.scss` 与 `_md-ref-palette.scss` 基线值（light/dark 全角色），非模型记忆值。
- Material Symbols: 官方 `google/material-design-icons/symbols/web/<name>/materialsymbolsoutlined/*_24px.svg` 路径数据，落地在 `src/icons/paths.ts` 并以内联 SVG 渲染，无字体、无 CDN、无运行时网络。
- Lazyweb: 未运行 —— 用户已给出完整视觉系统（MD3），外部站点调研不增加决策信息。
- Imagen drafts: 未运行 —— 桌面工具界面由 MD3 组件语法直接定义，不需要图像稿。
- Skipped lanes: 品牌 Layer B / 图片生成 / 克隆 URL —— 与用户指定的 MD3 冲突或无关。

## 1. Atmosphere & Identity

一间安静的控制室：左侧是秩序井然的预设清单，右侧是唯一正在编辑的文档。视觉签名是 **层级靠色调、层级不靠边框**：MD3 的 surface-container 阶梯把顶栏、清单、编辑区、对话框分成可感知但几乎看不见的层次；颜色只在“状态”和“动作”上出现——当前生效的预设、未保存的修改、将被写入的目标。没有渐变、玻璃、装饰性图形。

### Screen Brief（唯一主界面：预设工作台）

- **Primary action（唯一）**：`保存`（filled button）。它把所有编辑写回预设存储；同级的 `应用` 是独立动作，只有已保存且无未保存修改时可用。
- **Information priorities**：
  1. 当前选中的目标（opencode / omo）与将写入的真实路径；
  2. 预设名 + 当前生效标记 + 未保存状态；
  3. 预设内容（JSON 编辑 / 与真实目标的差异）。
- **Left out on purpose**：统计卡片、欢迎语、副标题、装饰性徽章、预设数量环形图、搜索框（列表规模为个位数）、远程字体、任何“示例预设”占位数据。
- **Required content**：预设列表（名称、描述、目标、生效标记）、目标过滤器（全部/opencode/omo）、新建按钮、刷新、设置、顶栏 `导入配置文件`（原生文件选择，支持 .json/.jsonc、可多选）、`导入当前配置`（读取两个目标文件的当前内容，每次调用都由后端追加新预设）、`还原至原配置`（仅启动基线可还原时启用）、启动警告提示、拖放导入提示、预设名称、描述、目标选择、JSON 内容、保存、应用、复制、重命名（通过名称字段 + 保存）、导入、导出、删除（确认）、编辑/差异标签、无效 JSON 提示、未保存状态与离开保护、设置对话框（两个目标路径 + 主题）、桌面运行时缺失提示、错误提示。
- **Data shape -> component**：预设 = 有序清单（导航列表项）→ `list-detail` 左清单右详情；单个预设 = 详情表单 + 原始 JSON 编辑；草稿 vs 真实目标 = 统一差异列表（+/- 行），而非卡片。
- **Layout**: `Layout: fixed-sidenav-shell + list-detail — grid rows [auto auto minmax(0,1fr) auto], columns [20rem minmax(0,1fr)]; scroll owners: preset list owns its own scroll, editor body owns its own scroll; header/footer fixed.`

## 2. Color

MD3 基线方案（baseline），全部角色双主题。浅色以 `surface` #FEF7FF 为底，深色以 #141218 为底；容器层级逐级抬升。

### Palette — MD3 roles

| Role | Token | Light | Dark | Usage |
|---|---|---|---|---|
| primary | `--md-sys-color-primary` | #6750A4 | #D0BCFF | 主动作、焦点环、选中强调 |
| on-primary | `--md-sys-color-on-primary` | #FFFFFF | #381E72 | 主动作文字 |
| primary-container | `--md-sys-color-primary-container` | #EADDFF | #4F378B | 选中项底色 |
| on-primary-container | `--md-sys-color-on-primary-container` | #21005D | #EADDFF | 选中项文字 |
| secondary | `--md-sys-color-secondary` | #625B71 | #CCC2DC | 次要强调 |
| on-secondary | `--md-sys-color-on-secondary` | #FFFFFF | #332D41 | — |
| secondary-container | `--md-sys-color-secondary-container` | #E8DEF8 | #4A4458 | tonal 按钮、标签 |
| on-secondary-container | `--md-sys-color-on-secondary-container` | #1D192B | #E8DEF8 | tonal 按钮文字 |
| tertiary | `--md-sys-color-tertiary` | #7D5260 | #EFB8C8 | 差异新增行强调 |
| on-tertiary | `--md-sys-color-on-tertiary` | #FFFFFF | #492532 | — |
| tertiary-container | `--md-sys-color-tertiary-container` | #FFD8E4 | #633B48 | — |
| on-tertiary-container | `--md-sys-color-on-tertiary-container` | #31111D | #FFD8E4 | — |
| error | `--md-sys-color-error` | #B3261E | #F2B8B5 | 错误文字、删除动作 |
| on-error | `--md-sys-color-on-error` | #FFFFFF | #601410 | — |
| error-container | `--md-sys-color-error-container` | #F9DEDC | #8C1D18 | 错误横幅、差异删除行 |
| on-error-container | `--md-sys-color-on-error-container` | #410E0B | #F9DEDC | — |
| surface | `--md-sys-color-surface` | #FEF7FF | #141218 | 应用背景 |
| on-surface | `--md-sys-color-on-surface` | #1D1B20 | #E6E0E9 | 正文、标题 |
| surface-variant | `--md-sys-color-surface-variant` | #E7E0EC | #49454F | 弱底 |
| on-surface-variant | `--md-sys-color-on-surface-variant` | #49454F | #CAC4D0 | 次要文字、图标 |
| surface-dim | `--md-sys-color-surface-dim` | #DED8E1 | #141218 | 窗口背衬 |
| surface-bright | `--md-sys-color-surface-bright` | #FEF7FF | #3B383E | — |
| surface-container-lowest | `--md-sys-color-surface-container-lowest` | #FFFFFF | #0F0D13 | 编辑器底 |
| surface-container-low | `--md-sys-color-surface-container-low` | #F7F2FA | #1D1B20 | 左侧清单 |
| surface-container | `--md-sys-color-surface-container` | #F3EDF7 | #211F26 | 顶栏、字段 |
| surface-container-high | `--md-sys-color-surface-container-high` | #ECE6F0 | #2B2930 | 对话框 |
| surface-container-highest | `--md-sys-color-surface-container-highest` | #E6E0E9 | #36343B | 悬停底、代码块 |
| outline | `--md-sys-color-outline` | #79747E | #938F99 | 控件描边、输入框 |
| outline-variant | `--md-sys-color-outline-variant` | #CAC4D0 | #49454F | 分隔线、静态描边 |
| inverse-surface | `--md-sys-color-inverse-surface` | #322F35 | #E6E0E9 | — |
| inverse-on-surface | `--md-sys-color-inverse-on-surface` | #F5EFF7 | #322F35 | — |
| inverse-primary | `--md-sys-color-inverse-primary` | #D0BCFF | #6750A4 | — |
| shadow | `--md-sys-color-shadow` | #000000 | #000000 | 仅对话框投影 |
| scrim | `--md-sys-color-scrim` | #000000 | #000000 | 模态遮罩 |

### Extended roles（MD3 无 success 角色，按 MD3 custom color 规则扩展）

| Role | Token | Light | Dark | Usage |
|---|---|---|---|---|
| success | `--app-color-success` | #386A20 | #B1D18A | 保存/应用成功图标 |
| on-success-container | `--app-color-on-success-container` | #0C1F00 | #C8E6AD | 成功横幅文字 |
| success-container | `--app-color-success-container` | #D7F0C2 | #2C4A16 | 成功横幅底 |
| diff-add-bg | `--app-color-diff-add-bg` | #E6F4E0 | #1E3312 | 差异新增行底 |
| diff-add-ink | `--app-color-diff-add-ink` | #24520C | #C8E6AD | 差异新增文字 |
| diff-remove-bg | `--md-sys-color-error-container` | #F9DEDC | #8C1D18 | 差异删除行底 |
| diff-remove-ink | `--md-sys-color-on-error-container` | #410E0B | #F9DEDC | 差异删除文字 |
| state-hover | `--state-hover-opacity` | 0.08 | 0.08 | 状态层 hover（`currentColor` 叠加） |
| state-focus | `--state-focus-opacity` | 0.10 | 0.10 | 状态层 focus |
| state-pressed | `--state-pressed-opacity` | 0.10 | 0.10 | 状态层 pressed |

### Rules
- 除 `DESIGN.md` 外不得出现原始 hex；`src/styles/tokens.css` 是唯一落地处。
- `primary` 只用于交互（主按钮、焦点、选中）；不用作装饰色块、不用作单侧色条。
- 对比度按 WCAG 2.2 AA：正文 4.5:1，大字号/图标/控件边界 3:1。MD3 baseline 配对均满足。
- 深色模式由 `html[data-theme="dark"]` 切换角色值；`system` 在运行时解析为 light/dark，不在 CSS 中依赖媒体查询切换（避免与用户选择冲突）。

## 3. Typography

MD3 type scale，字体使用 Windows 本地系统栈（离线要求，不下载 Roboto）：`"Segoe UI Variable Text", "Segoe UI", system-ui, "Microsoft YaHei UI", sans-serif`。等宽：`"Cascadia Mono", "Cascadia Code", Consolas, "Microsoft YaHei UI", monospace`（JSON 编辑器与差异视图）。

| Level | Size | Weight | Line Height | Tracking | Usage |
|---|---|---|---|---|---|
| headline-small | 24px | 400 | 32px | 0 | 空状态标题、对话框标题 |
| title-large | 22px | 400 | 28px | 0 | 应用名 |
| title-medium | 16px | 500 | 24px | 0.15px | 编辑区标题、清单标题 |
| title-small | 14px | 500 | 20px | 0.1px | 预设名 |
| body-large | 16px | 400 | 24px | 0.5px | 编辑内容 |
| body-medium | 14px | 400 | 20px | 0.25px | 正文、说明 |
| body-small | 12px | 400 | 16px | 0.4px | 路径、元信息 |
| label-large | 14px | 500 | 20px | 0.1px | 按钮、字段标签 |
| label-medium | 12px | 500 | 16px | 0.5px | 过滤器、徽章 |
| label-small | 11px | 500 | 16px | 0.5px | 差异行号、计数 |

### Rules
- 正文不小于 12px；11px 仅用于差异行号与紧凑标签。可交互正文 14px。行宽在编辑区不受限（代码），说明文字 `max-inline-size: 68ch`。
- 中文使用系统字体，字距不额外压缩；数字与行号使用 tabular-nums。
- 标题层级每级至少 1.25×；不使用全大写英文标签替代中文标签。

## 4. Spacing & Layout

### Base Unit
4px 栅格。间距 token：`--space-1` 4, `--space-2` 8, `--space-3` 12, `--space-4` 16, `--space-5` 20, `--space-6` 24, `--space-8` 32, `--space-10` 40, `--space-12` 48。

### Shell
- 顶栏 64px；底部状态栏 40px；左清单 320px（`--rail-width`），随窗口在 820px 以下堆叠为单列（Tauri 最小窗口 860px 时仍保持左清单右编辑）。
- 应用高度 `100dvb`，`grid-template-rows: auto auto minmax(0, 1fr) auto`（顶栏 / 横幅 / 工作区 / 状态栏）。清单与编辑正文各自拥有滚动，均设 `min-block-size: 0`。
- 编辑区最小宽度 0，长 JSON 行在 `<textarea>` 内部横向滚动，不推挤布局；路径用 `overflow-wrap: anywhere`。

### Shape（MD3 shape scale）
`--shape-xs` 4px（输入框）、`--shape-sm` 8px（标签）、`--shape-md` 12px（清单项、卡片）、`--shape-lg` 16px（面板内块）、`--shape-xl` 28px（对话框）、`--shape-full` 999px（按钮、分段按钮、徽章）。

### Rules
- 间距只用 token；`auto`/`%`/`minmax(min(…),100%)`/`1fr` 等浏览器力学值保持原始。
- 分区靠 `surface-container` 色调与 1px `outline-variant` 分隔；不使用彩色单侧色条、不用双色渐变描边。
- 分区标题上方间距 ≥ 1.5× 下方；组间距 ≥ 2× 组内间距。

## 5. Components

### Top app bar（`.top-bar`）
- **Structure**: 品牌图标 + 应用名 + 根目录路径（`folder_open` + 等宽小字）+ 右侧 `导入配置文件` / `导入当前配置` / `还原至原配置` 文字按钮 + `刷新` / `设置` 图标按钮。副标题仅用于加载中、读取失败或浏览器预览等临时状态，数据就绪后不显示。
- **States**: default / hover（状态层）/ focus-visible（3px primary 环）/ disabled（38% 透明）。`导入配置文件` 在空预设状态仍可用；`导入当前配置` 读取两个目标文件的当前内容并由后端追加预设，不合并重复内容，有未保存编辑时先确认放弃；`还原至原配置` 仅在 `Snapshot.canRestoreStartup` 为 true 且无忙碌操作时启用。还原确认只说明启动时锁定的原始路径与内容，不展示当前设置路径；设置变更不改变还原基线。
- **Accessibility**: `header` + `h1`；图标按钮带 `aria-label` 与 `title`；两个文字按钮保留 `aria-label`，窄窗口隐藏可见文字时仍可被读屏识别。
- **Layout**: `cluster`，固定不滚动；根目录路径在视口宽度大于 1100px 时可见，1100px 及以下时隐藏，仅保留 `title` 提示；860px 下三个文字按钮保留完整标签，通过 16px 动作内边距与隐藏根目录保证顶栏不横向溢出；760px 及以下隐藏三个文字按钮的可见标签，仅保留图标与可访问名称；560px 及以下把应用名标题改为视觉隐藏，避免顶栏横向溢出。

### Preset list（`.preset-list`）
- **Structure**: 目标分段过滤器（全部 / opencode / omo，带计数）→ `role="list"` 的按钮行（名称、描述单行、目标徽章、生效 `check_circle`）→ 空状态。
- **Variants**: 选中（`primary-container` 底 + `on-primary-container`）、生效（success 图标 + “当前生效”文字）、普通。
- **States**: default / hover / focus / selected / empty / 过滤后为空。
- **Accessibility**: 每行是 `<button aria-pressed>`，名称唯一可辨；生效状态有文字，不只靠颜色。
- **Layout**: `stack`；列表自身滚动，`min-block-size:0`。

### Editor pane（`.editor`）
- **Structure**: 头部（预设名输入 + 目标选择 + 生效/未保存状态）→ 描述输入 → 动作条（保存 filled、应用 tonal、复制/导入/导出/删除 icon buttons）→ `tablist`（编辑 / 差异）→ 内容区（textarea 或差异列表）→ JSON 校验行。
- **States**: default / dirty / invalid JSON / busy / desktop-required / no-selection / read error。
- **Accessibility**: 每个字段有可见 `<label for>`；`Ctrl+S` 保存；删除与应用需确认对话框；对话框 Esc 关闭、焦点回归触发按钮。

### Tabs（`.tabs`）
- **Structure**: `role="tablist"` + 两个 `role="tab"`（`code` 编辑 / `difference` 差异）。
- **States**: selected（`secondary-container`）、hover、focus-visible；方向键左右切换。
- **Accessibility**: `aria-selected`、`tabindex` roving、`aria-controls`。

### Diff view（`.diff`）
- **Structure**: 统一差异列表，每行 = 行号槽 + `+`/`−` 记号 + 文本；新增用 success 容器、删除用 error 容器；顶部说明对比对象路径。
- **States**: identical（“内容与目标一致”）、target-missing、target-error、truncated（超过渲染上限提示）、empty。
- **Accessibility**: 记号有 `aria-label`（新增行/删除行）；文本 `white-space: pre` 横向滚动。
- **Bound**: 差异算法先裁剪公共前后缀；剩余部分用唯一行锚点（patience）递归分割；无锚点时降级为整段删除+新增；每段有规模上限，避免二次方内存。

### Text field（`.field`）
- **Structure**: 可见 label（label-large）+ `input`/`textarea` + 支持文本（body-small）。
- **States**: default / hover / focus（2px primary 描边 + 1px outline 内衬）/ disabled / invalid（error 描边 + 错误文本）。
- **Accessibility**: `aria-describedby` 指向支持文本；`aria-invalid`。

### Buttons
- **Filled**（保存、确认应用）：primary / on-primary，`--shape-full`，高 40px。
- **Tonal**（应用）：secondary-container / on-secondary-container。
- **Outlined**（重试）：outline 描边 + on-surface。
- **Text**（取消、设置对话框次要动作）：primary 文字，无底色。
- **Icon button**（刷新、设置、复制、删除等）：44×44 命中区（图标 20px），`--shape-full`。
- **States**: default / hover / focus-visible / pressed / disabled / busy（保持文字与宽度，顶部显示不确定进度条，不插入 spinner 改变布局）。

### Segmented filter（`.segmented`）
- **Structure**: 左侧清单内边距 16px 的容器（`.rail__filters`）承载 `role="group"` 的三个 `<button aria-pressed>`；每项 = 文本（label-medium 12px/16px）+ 计数（label-small 11px/16px），选中项另有 12px `check` 图标，绝对定位在左侧 8px 处；未选中时 `display:none`，不保留空槽位。
- **States**: selected / hover / focus / disabled；固定高度 40px；每项左右内边距 8px，按内容分配宽度并均分剩余空间，默认与最小窗口下均不裁剪。
- **Layout**: 标签与计数作为一组在整项内水平居中；`check` 位于左侧留白，选中 `OpenCode`（含两位数计数）时与标签矩形不得重叠；按钮之间的 1px 分隔线保留，分段条左右各留 16px，不贴清单边缘。

### Badges（`.badge`）
- **Structure**: 目标徽章（`opencode` / `omo`）、生效徽章（“当前生效”）、未保存徽章（“未保存”）。
- **Rule**: 只在状态可变化时出现；纯装饰标签不进入界面。

### Banner（`.banner`）
- **Structure**: 图标 + 文本 + 关闭按钮；`role="status"`（成功）/ `role="alert"`（错误）。`detail` 保留换行，用于逐条展示启动警告或失败文件。
- **States**: success / error / info；错误常驻直到手动关闭。首次加载后的 `Snapshot.startupWarnings` 以 info 横幅展示，缺失文件不阻断使用，无效配置文本直接可见。

### Drop overlay（`.drop-zone`）
- **Structure**: 绝对定位覆盖工作区，向内 16px；虚线 primary 边框 + `surface-container-high` 底 + `upload_file` 图标 + “松开以导入配置文件” + 支持格式说明。
- **States**: hidden / visible。原生拖入（Tauri `onDragDropEvent` 的 enter/over）或浏览器 File 拖入时显示，leave/drop 后立即隐藏；覆盖层 `pointer-events: none`，不拦截放下动作。
- **Accessibility**: 顶栏 `导入配置文件` 是键盘与单指针替代路径；拖放不是唯一入口。

### Dialog（`.dialog`）
- **Structure**: 原生 `<dialog>`，`--shape-xl`，`surface-container-high`，scrim 32%（深色 50%），shadow 3 级；标题 headline-small，正文 body-medium，动作右对齐。
- **States**: settings / confirm-apply / confirm-delete / confirm-discard / desktop-required 提示。
- **Accessibility**: 原生模态焦点圈定；Esc 关闭（表单取消）；初始焦点在第一个字段或取消按钮；关闭后焦点回到触发元素。

### Empty state（`.empty-state`）
- **Structure**: 图标（`description` / `warning`）+ 标题 + 说明 + 可选主行动按钮。
- **States**: no-presets（“新建预设”）、no-selection、desktop-required（无假数据）、load-error（显示真实错误 + 重试）。

## 6. Motion & Interaction

| Type | Duration | Easing | Usage |
|---|---|---|---|
| Micro | 100ms | `cubic-bezier(0.2, 0, 0, 1)` | 按下、状态层淡入 |
| Standard | 200ms | `cubic-bezier(0.2, 0, 0, 1)` | 横幅出现 |
| Emphasis | 300ms | `cubic-bezier(0.05, 0.7, 0.1, 1)` | 对话框进入 |
| Progress | 1100ms | `linear` | 不确定进度条 |

### Rules
- 只动 `transform` / `opacity`；不动布局属性。
- 每个交互元素具备 hover / pressed / focus-visible / disabled。
- `prefers-reduced-motion: reduce` 下移除过渡和进度条动画（进度条改为静态填充）。
- 无不服务状态的动画；悬停不改变信息时不加动效。

## 7. Depth & Surface

**Strategy: tonal-shift + 单一投影层级。**

| Level | Value | Usage |
|---|---|---|
| 0 | 无投影，`surface` / `surface-container-*` 分色 | 顶栏、清单、编辑区 |
| 3 | `0 8px 24px rgba(0,0,0,.18)` | 模态对话框 |

- 不使用 `backdrop-filter`、玻璃拟态、网格渐变、彩色光晕。
- 分隔优先 1px `outline-variant`；卡片/清单项不叠边框 + 投影。

## 8. Accessibility Constraints & Accepted Debt

### Constraints
- WCAG 2.2 AA：正文 4.5:1，大字号 3:1，图标与控件边界 3:1，双主题。
- 所有交互元素有可见 focus-visible 环（3px primary，offset 2px），不被顶栏/滚动容器遮挡。
- 指针目标 ≥ 24×24 CSS px；图标按钮 44×44，相邻间距 4px。
- 全键盘可达：Tab 顺序为顶栏 → 过滤器 → 列表 → 编辑字段 → 动作 → 标签 → 内容；对话框 Esc 关闭；不困住焦点。
- 320px 宽度下无横向滚动的首要内容；编辑器内部文本横向滚动属于二维内容（代码），允许。
- 显示语言中文；错误消息来自后端原文，不翻译、不吞掉。
- 文件拖放提供顶栏 `导入配置文件` 作为键盘与单指针替代；无登录、无帮助重复说明、无多步表单。

### Accepted Debt
| Item | Location | Why accepted | Owner / Exit |
|---|---|---|---|
| 使用 Windows 系统字体替代 Roboto | `DESIGN.md` §3、`src/styles/tokens.css` | 离线桌面应用不下载远程字体；Segoe UI 在 Windows 上的中英文覆盖与可读性达标 | 若后续内置 Roboto 可变字体则替换字体栈 |
| 版本号/更新日期等元数据不显示 | 预设清单 | 契约未提供该数据，不发明字段 | 后端契约扩展后再加 |
| 无虚拟滚动 | 差异视图 | 单文件配置规模有限；以行数上限 + 截断提示保证内存有界，避免引入复杂虚拟列表 | 若出现超大配置再引入窗口化 |
