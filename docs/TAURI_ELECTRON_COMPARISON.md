# Catch：Tauri / Rust 与 Electron 静态对照

> 历史记录。文中的 Electron 路径可在 [v2.0.0 标签](https://github.com/puuchi/Catch/tree/v2.0.0) 查阅。

盘点日期：2026-11-15（Asia/Shanghai）。源码基线：`dad9efa`。

本次只阅读源码、配置和现有测试定义，没有启动软件、运行测试、构建安装包或测量性能。文中的“有”表示存在可追踪的界面 → 命令 → 业务实现，不表示已经通过运行验收。视觉结论来自模板与 CSS，不代表实际渲染截图一致。

## 1. 对照范围与结论

| 实现 | 代码位置 | 标识 |
| --- | --- | --- |
| 当前 Electron | `src/`、根目录 `package.json`、`test/` | Catch 2.0.0，Electron 45.3.1 |
| Tauri / Rust | `frontend/`、`src-tauri/`、`crates/` | Catch Next 0.1.0，Tauri 2.12.5，Vue 3 / TypeScript |
| 重构前备份 | 工作区同级的 `Catch-backup-1.8.3-before-rebuild/` | Electron 1.8.3；用于交叉回查早期能力 |

主要结论：Tauri 已接入原有四个主要页面和计时业务，也新增周期待办、资料关联、每日植物与目标、界面草稿持久化。但尚未完整保留 Electron 的操作行为与信息展示，不能仅凭业务命令齐全就视为功能完全对等。

优先差异是：从待办重复开始会新建事项、恢复提示未显示、时间轴空档没有穿插排序、时间轴菜单失去显式按钮与原有键盘导航。另有聚焦统计、跨日详情、时间编辑快捷控件和旧绿芽动画等差异，需明确是否保留旧体验。

状态说明：**有**＝当前有实现链路；**变化**＝主要能力存在，行为或界面不同；**部分**＝只保留部分体验；**无**＝在所检查的入口和实现中未发现此能力。没有使用完成百分比，避免把不同大小的功能当作等权项目。

## 2. 功能对照

### 待办

| 功能 | Electron 2.0.0 | Tauri / Rust | 差异与代码依据 |
| --- | --- | --- | --- |
| 创建、编辑标题与备注 | 有 | 有 | E1 / T1 / T5；标题 80 字、备注 2000 字限制仍在 |
| Enter 快速创建 | 有 | 有 | Electron 显式按键处理；Vue 表单提交 |
| 紧急、截止日期 | 有 | 有 | E1 / T1；排序继续使用紧急 → 今日截止 → 其他 → 完成 |
| 截止日期相对说明 | 有 | 部分 | Electron 显示“今天截止 / 已逾期 N 天 / N 天后”；新版只显示月日与颜色，见 D09 |
| 完成、撤销完成 | 有 | 有 | 当日完成继续留在主列表，历史投影与归档存在 |
| 今日已完成分隔线 | 有 | 无 | 新版仍排序到后面，但没有旧分组标题 |
| 完成历史与按月分组 | 有 | 部分 | 新版历史仍按完成日期倒序，有逐条日期，没有月份分组 |
| 一键归档已完成 | 有 | 有 | E1 / T1 / T11 |
| 删除、回收站恢复、永久删除 | 有 | 有 | E4 / T6 / T11 |
| 从待办开始计时 | 有 | 有，但行为变化 | 重复点击不会复用已有未结束事项，见 D01 |
| 单击文字编辑 | 有 | 变化 | 新版单击展开备注/资料，双击或编辑按钮进入弹窗 |
| 备注默认展示 | 有 | 变化 | 新版默认折叠，展开后展示 |
| 新建草稿取消并清空 | 有 | 变化 | 旧版“取消”/Esc 重置；新版为“收起”且保留草稿，无对应 composer Esc 重置处理 |
| 周期待办 | 无 | 新增 | 每天、指定星期、间隔天数、每月日期，见 N01 |
| 修改周期范围、停止重复 | 无 | 新增 | 编辑支持“仅本次 / 本次及以后”，重复弹窗支持停止 |
| 关联文件、文件夹、网页 | 无 | 新增 | 已有资料选择、关联、打开、重新定位、移除链路，见 N02 |

### 正在做与计时

| 功能 | Electron 2.0.0 | Tauri / Rust | 差异与代码依据 |
| --- | --- | --- | --- |
| 不经待办直接开始活动 | 有 | 有 | E2 / T2 / T10 |
| 一个运行事项、多项后台暂停 | 有 | 有 | 开始/继续会暂停当前事项；累计与 sessions 结构保留 |
| 暂停、继续、结束 | 有 | 有 | Tauri 的业务状态由 Rust 管理，不依赖 WebView 是否存在 |
| 暂停后保留前台卡片 | 有 | 有 | 两版 foreground 规则一致；1.8.3 还没有此体验 |
| 结束摘要与原事项重新开始 | 有 | 有 | 保留事项 ID、累计投入与历史；1.8.3 没有当前的结束摘要入口 |
| 结束计时不自动完成待办 | 有 | 有 | tracking 与 todos 完成操作独立 |
| 本次进展 Enter 保存 | 有 | 有 | 新版还在失焦、暂停/结束等 action 前尝试保存 |
| 进展保存后的展示 | 有 | 变化 | 旧版切到正文和铅笔按钮；新版保持输入框 |
| 完成后在前台查看最后进展 | 有 | 无此展示 | 新版完成状态隐藏进展表单，历史数据仍在详情中 |
| “下一步”允许空进展 | 有 | 变化 | Rust 命令允许空进展，Vue 按钮要求非空，见 D08 |
| 卡片“开始”时间 | 本次投入开始 | 整个事项最初开始 | 新版使用 `current.startTime` 而非 `lastSession.startTime`，见 D07 |
| 后台列表最近更新优先、颜色点 | 有 | 部分 | 新版列表未沿用 updatedAt 倒序，未显示原颜色点 |
| 导航中的暂停状态提示点 | 有 | 部分 | 新版只有存在 running 项才显示导航点；原版还区分 paused |
| 长计时数字动态缩字 | 有 | 部分 | 新版有 CSS 自适应字号，但没有旧版按实际文本宽度再次缩字逻辑；溢出与否未实测 |

### 时间轴与回收站

| 功能 | Electron 2.0.0 | Tauri / Rust | 差异与代码依据 |
| --- | --- | --- | --- |
| 日期选择、前后一天、今天 | 有 | 有 | E2 / T3 |
| 当日总投入、跨日裁切 | 有 | 有 | T7 保留按日期边界裁切和累计计算；存在与旧版对照的测试定义 |
| 今天倒序、历史正序 | 有 | 有 | 记录 entries 排序保留；不包括新版独立渲染的空档 |
| 空档计算与一键预填补录 | 有 | 有 | 排列与展示阈值变化，见 D03 |
| 空档穿插到正确时间位置 | 有 | 无 | 新版先渲染全部记录，再渲染全部空档 |
| 事项颜色、时长深浅、图例 | 有 | 有，图例简化 | 原图例有分级色块，新版为文字区间；逻辑区间仍为 15/30/60 分钟 |
| 聚焦同一事项 | 有 | 有，但变化 | 原版弱化其他记录；新版过滤掉其他事项，并隐藏空档 |
| 聚焦后的当日次数与时长摘要 | 有 | 无 | 新版只有“取消聚焦”，顶部仍是整日总量，见 D04 |
| 再次投入标记、行内状态 | 有 | 部分 | 新版未保留旧版返回标记和 paused/running 状态徽标 |
| 时间轴显式“⋯”操作按钮 | 有 | 无 | 新版通过右键打开菜单，见 D05 |
| 菜单方向键、Home/End、Esc、焦点回退 | 有 | 部分 | Vue 未实现原有菜单交互；普通详情按钮仍可使用 |
| 事项详情与累计、投入次数 | 有 | 有 | E3 / T3 |
| 详情按日分组、定位来源记录 | 有 | 无原体验 | 新版平铺“第 N 次”卡片，无原来的按日分组与来源定位 |
| 跨日投入的逐日详情 | 有 | 部分 | 时长按日计算仍有；详情卡片主要显示开始日与时分，跨日结束日不明确，见 D06 |
| 事项改名、改状态 | 有 | 有 | T5 / T10 |
| 补录新事项 / 现有事项 | 有 | 有 | 新版还在详情页提供“补录一次投入”快捷入口 |
| 补录事项下拉区分同名项目 | 有 | 部分 | 旧版附状态、同名时附 ID 尾号；新版选项仅名称 |
| 单次投入编辑、有效时长预览 | 有 | 有 | 暂停空段保留处理仍在 T7；输入控件不同 |
| 编辑运行中的投入 | 自动暂停后打开 | 需先手动暂停 | 新版在详情/菜单禁用未结束 session 的编辑按钮 |
| 自定义时间选择器、“现在”、当天/次日 | 有 | 无旧控件 | 新版采用两个 `datetime-local`，允许分别指定日期，不等于失去跨日编辑能力 |
| 删除单次 / 整个事项进回收站 | 有 | 有 | 新版软删除入口没有沿用旧版二次点击确认；仍可恢复 |
| 回收站按类型过滤、恢复 | 有 | 有 | 待办 / 事项 / 单次投入均有 |
| 回收站按删除时间混合倒序 | 有 | 无原排序 | 新版先待办、后 tracking.trash，没有统一 deletedAt 倒序 |
| 回收站显示删除时分 | 有 | 部分 | 新版主要显示日期，单次记录不显示旧版完整起止时分摘要 |
| 永久删除确认 | 二次点击 | 确认弹窗 | 两版都有确认机制 |

### 桌面集成、外观与数据

| 功能 | Electron 2.0.0 | Tauri / Rust | 差异与代码依据 |
| --- | --- | --- | --- |
| 无边框、透明背景、置顶、最小化 | 有 | 有 | E5 / T8 / T9 |
| 主窗口缩放、拖动、位置恢复 | 有 | 有，持久化时机不同 | 新版宽高在收起时更新；直接调整大小后退出未沿用旧版 resize 保存链路，见 D10 |
| 置顶状态跨重启保存 | 无对应保存 | 新增 | Electron 新建窗口默认置顶；新版 Preferences 持久化 pinned |
| 背景透明度 0–100% | 有 | 有 | 沿用相同背景色与透明度公式 |
| 36 逻辑像素悬浮植物、拖动、点击展开、右键退出 | 有 | 有 | 新版原生 Win32 分层窗口；含 DPI/工作区裁切逻辑 |
| 旧版连续生长、长成后轻摆 | 有 | 无原动画 | 新版按每日投入选择静态阶段帧，见 D11 |
| 悬浮状态颜色点 | 有 | 有 | running / paused / idle 仍有不同颜色 |
| 悬浮提示中的事项名与分钟数 | 有 | 无原提示 | 新版窗口/无障碍名称为植物与状态，未见原版同等 title 提示实现 |
| 每日五种植物选择、投入目标与达成反馈 | 无 | 新增 | 见 N03 |
| 收起后主界面的资源生命周期 | 隐藏但保留窗口 | 保存草稿后销毁 WebView | 新版再展开创建 WebView，Rust 状态持续运行；未据此断言具体内存收益 |
| 页面、草稿、滚动位置跨重启恢复 | 无完整机制 | 新增 | catch-ui-state.json；见 N04 |
| 单实例 / 二次启动唤起 | 有 | 有 | Electron 单实例锁 / Tauri single-instance 插件 |
| 锁屏、睡眠、系统结束暂停 | 有 | 有 | 原生 WTS、电源与会话消息接入 interrupt |
| 计时中退出确认、退出前保存 | 有 | 有 | 主界面确认弹窗 / 悬浮时原生确认 |
| 检查点、异常恢复、不计全部离线时间 | 有 | 有 | 30 秒检查点；恢复提示展示缺口见 D02 |
| 本地 JSON、原子写入、备份与版本保护 | 有 | 有 | Rust 另有外部修改检测、目录锁、恢复事务等防护 |
| 旧数据读取 | 默认继续使用 float-todo 目录 | 独立目录 + 显式导入 | 见 M01；不是安装后自动原位接管 |
| 旧 config.json 设置迁移 | 原地使用 | 无 | 当前导入范围仅三份记录文件 |
| 安装包与免安装 ZIP 配置 | NSIS + ZIP | NSIS | 新版有 pack 脚本，但未配置等价 ZIP / SHA256 发布链路 |
| 自动更新、系统托盘、全局快捷键、账号同步、搜索、CSV 导出 | 未发现 | 未发现 | 本次检查未发现有效产品入口；Tauri 的 tray-icon 编译特性不代表已创建托盘 |

## 3. 关键差异与处理建议

### D01：同一待办再次开始，事项会被拆成多个

Electron 在收到 `start-todo-activity` 后，先找同 todoId 的未完成事项：已经运行就转到该页，暂停则恢复，否则才 start。1.8.3 也有此逻辑。

Tauri 的 [Todos.vue](../frontend/src/components/Todos.vue#L67) 每次都发送 start；[Rust Start 分支](../crates/catch-core/src/tracking/commands.rs#L185) 每次创建新 Matter，应用层只补充权威待办标题和资料，不负责复用。

影响示例：待办 A 开始 → 暂停 → 回待办再次开始。旧版增加原事项的投入次数，新版得到新的事项 ID、独立累计与颜色。这是优先补齐的行为差异，建议在应用服务层统一处理复用规则。

### D02：异常恢复已经做了，但提示没有显示出来

[Rust TrackingStore](../crates/catch-storage/src/tracking.rs#L68) 会生成备份恢复、检查点恢复等 notice；应用快照将它作为 trackingNotice 返回。Vue [models.ts](../frontend/src/models.ts#L126) 声明该字段，但当前展示链路未消费它。

[App.vue](../frontend/src/App.vue#L243) 只显示 message 或 storageBlocked。Electron 的 [setState](../src/ui/tracking/controller.js#L111) 则显示 state.error / state.notice。因此用户可能看到恢复后的数据，却不知道它已回退到备份或检查点。建议接通并保留可关闭提示。

### D03：时间轴记录与空档没有合并排序

Electron 将 entries 和 gaps 合并排序后渲染。[Timeline.vue](../frontend/src/components/Timeline.vue#L205) 的记录循环结束后，才开始 [空档循环](../frontend/src/components/Timeline.vue#L264)。因此空档补录能力仍在，但空档不在对应记录之间。

此外旧 UI 过滤小于 10 秒空档，新版直接展示 gaps 算出的至少 1 秒空档。建议合并为同一时间轴列表，并明确展示阈值。

### D04：聚焦从“保留上下文并统计”变成“只筛选”

Electron [applyFocus](../src/ui/tracking/controller.js#L228) 弱化其他条目，显示所选事项的当日投入次数和累计时长。新版 [list/blanks](../frontend/src/components/Timeline.vue#L31) 过滤其他事项、隐藏空档，但没有聚焦摘要，total 仍统计整日所有事项。建议补回明确的选中事项次数/时长；是否隐藏其他记录可以作为独立设计选择。

### D05：时间轴菜单只剩右键入口

旧版每条记录都有“⋯”按钮，并有方向键、Home/End、Esc、Tab、焦点回退处理。新版 [Timeline.vue](../frontend/src/components/Timeline.vue#L209) 只在 article 的 contextmenu 事件打开行内菜单，没有等价的显式触发按钮。

详情入口仍在名称按钮上，删除也可进入详情操作，所以不是所有操作完全消失；但菜单可发现性与键盘操作能力减少。建议恢复可聚焦菜单按钮及键盘关闭/导航。

### D06：跨日详情、来源定位与时间编辑控件被简化

旧版 [details-view.js](../src/ui/tracking/details-view.js#L13) 按 sessionDays 分日展示，并可定位、高亮进入详情的来源投入。新版按 sessions 平铺，主要显示开始日期和开始/结束时分，没有按日拆分，也没有来源 session 锚点。

旧版自定义时间选择器、“现在”、当天/次日按钮被两个 [datetime-local](../frontend/src/components/Editors.vue#L162) 代替。新版仍能编辑跨日记录，不能将其记为“没有时间编辑”；缺的是原有快捷操作和展示细节。

### D07：当前卡片“开始”时间含义改变

Electron 展示最后一次投入 session.startTime；新版 [Current.vue](../frontend/src/components/Current.vue#L91) 展示 matter.startTime。再次继续或下一步后，事项最初开始时间不会变成当前投入开始时间。如果该字段仍表达“本次开始”，应改用 lastSession；若要表达“事项首次开始”，应修改文案。

### D08：“下一步”和完成后的进展展示有变化

新版 [Current.vue](../frontend/src/components/Current.vue#L143) 要求 progress 非空才允许下一步，Rust NextSession 自身并无此限制，旧版按钮也未要求非空。属于新增的 UI 限制。

旧版完成后仍能在卡片看到已保存的进展；新版完成后 [隐藏进展表单](../frontend/src/components/Current.vue#L131)，但数据可在时间轴详情查看。是否保留空进展拆分与完成后的进展摘要，需要明确产品规则。

### D09：待办信息展示减少

新版保留截止日期颜色，但移除“今天截止 / 逾期天数 / 未来天数”；没有“今日已完成”和历史月份分组。备注改为默认折叠；文字单击展开、双击/按钮编辑，旧版为单击内联编辑。

这是已能从模板确认的界面差异；前几项影响快速识别信息，建议补回。折叠和弹窗编辑可单独决定。

### D10：窗口尺寸保存时机与显示器变化处理不同

Electron [windows.js](../src/desktop/windows.js#L33) 监听 moved/resized，退出 flush，并在显示器增删/参数变化时统一 fit。Tauri 在 [collapse](../src-tauri/src/main.rs#L207) 才把当前宽高写入 preferences；窗口事件只处理 Moved 和 CloseRequested，退出保存已有 preferences，未重新读取当前窗口尺寸。

因此代码上存在“改大小后直接退出，下次仍使用先前尺寸”的路径。原生悬浮窗有 DPI/显示变化裁切，主窗口在展开时也 fit，但未见与旧版同等的已展开窗口显示器变化处理。后者是接入差异，实际多屏行为未运行核验。

### D11：绿芽连续动画改为每日阶段帧

Electron [animation.js](../src/ui/sprout/animation.js#L47) 在新运行事项时播放连续生长，完成后轻摆，并处理暂停、可见性与减少动态效果。

新版 [native.rs](../src-tauri/src/native.rs#L308) 从五个阶段选一帧，阶段由当日有效投入与目标比例决定；相同 plant/stage/status/size 会跳过重绘。主界面 PlantSprite 也使用阶段帧。不是“没有悬浮植物”，而是成长规则与动画表现都换了；尚未保留旧版连续生长与摆动。

### D12：软删除确认与回收站呈现不同

旧版时间轴单次/事项移入回收站有二次确认交互，新版按钮直接发删除命令。数据仍进入回收站；永久删除另有确认弹窗，并非直接永久丢失。

旧版回收站混合所有类型按 deletedAt 倒序，新版分两段渲染且只显示较简略日期。建议补回统一排序，再决定是否保留软删除的二次确认。

## 4. Tauri 新增能力的接通情况

### N01：周期待办

前端 [RepeatDialog](../frontend/src/components/RepeatDialog.vue)、编辑范围和待办标记，连接到应用层 RecurrenceRequest、[recurrence.rs](../crates/catch-core/src/recurrence.rs) 和存储事务。启动与跨日刷新均已接入宿主。

规则包括月末不足日期取最后一天、一个模板最多一个未完成实例、漏过多期取最近一期；完成推进后续计划。界面有创建、转换、改规则、停止入口。本次未发现独立的全部模板管理页，不把它计为已有功能。

### N02：资料关联

[Resources.vue](../frontend/src/components/Resources.vue) / ResourceDialog → host 选择/打开 → 应用服务 → [resources.rs](../crates/catch-core/src/resources.rs)。支持文件、文件夹、http/https 网页；拖放文件亦已接入；每项最多 32 份，重复关联校验存在。

从待办开始会复制资料快照到事项，之后待办改址或删除不改事项快照。打开根据已保存的关联 ID 查找目标；失效文件可重新定位；移除关联不删除源文件。这是路径/URL 关联，不是把附件内容复制进 Catch，也没有关联文件的全文搜索。

### N03：每日植物与投入目标

[Settings.vue](../frontend/src/components/Settings.vue)、顶部成长条 → growth 命令 → [growth.rs](../crates/catch-core/src/growth.rs) → GrowthStore。五种植物，每天选择一次，目标可关闭或设为 1–1440 分钟，按每日有效投入分为五阶段；宿主每秒观察，达成记录先保存再发界面反馈。

每日记录存在，但当前没有日历式历史植物/成就浏览页面；没有系统通知、声音奖励等产品入口。

### N04：持久化界面会话

[store.ts](../frontend/src/store.ts#L76) 将页面、历史模式、待办草稿、进展草稿、编辑器、资料/重复草稿、展开项、滚动等保存到 [view_session.rs](../crates/catch-app/src/view_session.rs)。350ms 防抖保存，收起前明确 flush。窗口代际检查防止旧 WebView 回调修改新窗口。

旧 Electron 在收起时保留隐藏窗口，因此同进程草稿自然保留；它没有同等的完整跨重启界面状态文件。Tauri 的新能力同时也是销毁 WebView 后保留体验所需的支撑。

## 5. 数据迁移与发布切换

### M01：记录导入存在，但不是完整原位升级

Electron 默认 `%APPDATA%/float-todo`。Tauri 默认按 identifier 使用独立应用目录；desktop.ps1 预览默认使用 artifacts 下隔离目录。宿主明确拒绝直接以名为 float-todo 的目录启动。

[LegacyImport.vue](../frontend/src/components/LegacyImport.vue) 提供选择目录 → 数量预览 → 导入；[legacy_import.rs](../src-tauri/src/legacy_import.rs) 比较源文件快照是否变化，Rust 存储以恢复事务提交，保留源数据原件和导入回执。

当前限制：

- 仅在新目录记录为空、无周期模板时导入，不支持向已有记录合并。
- 导入文件为 activity-log.json、todos.json、archive.json，包含原有回收站记录。
- 不导入 config.json 的透明度、窗口位置等偏好；新偏好文件是 catch-preferences.json。
- 不导入新版周期/成长的完整备份，拒绝把该入口当新版备份恢复器。
- 源目录只作只读快照检查；历史旧二进制并不自动获得当前共享目录锁保护，仍需用户先退出旧版。

### M02：正式发布链路仍以 Electron 为主

根 package.json 的 start/build/dist 和启动Catch.cmd 仍启动/构建 Electron；README、现有截图工具也是 Electron 路线。Tauri 有 tools/desktop.ps1 的 build/pack/run/dev，但产品名仍是 Catch Next、版本 0.1.0，主窗口系统标题仍是“Catch · 本地预览”。

.github/workflows/ci.yml 已有 Rust 核心测试、Vue 构建和 Tauri cargo check；生成/上传 Windows 分发包的 verify 作业仍使用 npm run dist，即 Electron。Tauri 当前配置只有 NSIS，并配置 WebView2 downloadBootstrapper；干净系统缺少 WebView2 时可能需要联网补装，不能直接沿用旧版“完整离线即用”的发布说明。

没有检查远程 GitHub 状态，也未把本地 dist/artifacts 中既有文件当作本次验收结果。

### M03：后续不能直接删除整个 src 目录

本次发现这些实际依赖：

- site/src/demo-state.js 引用 src/domain 的计时、待办与 foreground。
- site/src/app.js 与 site/src/styles.css 引用原 UI 的表单验证及绿芽动画/样式。
- frontend/test/display.test.ts 直接 require 原 JavaScript 时间投影与 session 调整函数。
- tools/tracking-parity.cjs、todos-parity.cjs 使用原版业务代码生成对照数据。
- Electron 的测试、截图与发布工作流仍使用 src。

所以正式切换默认分支时，需要先把仍使用的共享逻辑迁到明确的共享/兼容目录，或改写网站和测试依赖；然后移除 Electron 专用窗口、preload、bootstrap、构建依赖与运行时。历史版本通过 Git 标签/维护分支保留。本报告不执行清理。

## 6. 1.8.3 回查结论与证据边界

回查了备份的 renderer.js、activity-ui.js、recycle-bin.js、preload.js、main.js 和存储/样式入口。上述待办复用、时间轴聚焦摘要、日期分组、时间选择器与绿芽动画在 1.8.3 已经存在，不是仅 Electron 2.0 后来新增的能力。

Electron 2.0 相对 1.8.3 增加/完善了暂停前台保留、完成摘要和重新开始体验；这些在 Tauri 中已有对应实现。备份早期待办由 renderer 保存整份列表，当前 Electron 与 Rust 则经过应用命令边界，因此不应把早期代码架构直接当作新版功能标准。

未在本次回查范围发现需要单列为“1.8.3 独有、当前 Electron 和 Tauri 都已删除”的独立功能模块；这不表示备份每一条 CSS、每个边界交互均已逐字相同。

现有 Rust parity 测试与 Vue 投影测试能说明项目已有对照机制，不能覆盖所有页面编排。例如旧版 todoId 复用在 UI 控制器中，而非纯 TrackingModel；空档和记录单独计算正确也不保证模板组合顺序正确。本次没有运行这些测试，也不引用历史通过数量作为本次结果。

建议顺序：先补 D01 / D02 / D03 / D05 的行为与提示链路；再确定 D04 / D06–D12 中哪些交互要恢复、哪些是主动改版；最后统一数据升级说明、产品标识、文档、截图和 Tauri 发布工作流。当前无需先删除 Electron 代码。

## 7. 主要源码索引

| 编号 | 位置 |
| --- | --- |
| E1 | [Electron 待办界面](../src/ui/todos/controller.js)、[待办领域投影](../src/domain/todos/index.js) |
| E2 | [Electron 计时与时间轴控制器](../src/ui/tracking/controller.js) |
| E3 | [Electron 事项详情](../src/ui/tracking/details-view.js) |
| E4 | [Electron 回收站](../src/ui/trash/controller.js) |
| E5 | [Electron 窗口](../src/desktop/windows.js)、[启动与系统事件](../src/bootstrap/main.js) |
| E6 | [Electron 主界面模板](../src/ui/index.html)、[主题](../src/ui/shared/theme.css) |
| T1 | [Vue 待办](../frontend/src/components/Todos.vue) |
| T2 | [Vue 正在做](../frontend/src/components/Current.vue) |
| T3 | [Vue 时间轴与详情](../frontend/src/components/Timeline.vue) |
| T4 | [Vue 外观与成长](../frontend/src/components/Settings.vue) |
| T5 | [Vue 编辑器](../frontend/src/components/Editors.vue) |
| T6 | [Vue 回收站](../frontend/src/components/Trash.vue) |
| T7 | [Vue 时长与列表投影](../frontend/src/display.ts) |
| T8 | [Tauri 命令与窗口宿主](../src-tauri/src/main.rs) |
| T9 | [Win32 悬浮表面](../src-tauri/src/native.rs) |
| T10 | [Rust tracking 命令](../crates/catch-core/src/tracking/commands.rs) |
| T11 | [Rust 应用边界](../crates/catch-app/src/lib.rs)、[待办存储](../crates/catch-storage/src/todos.rs) |

两版 CSS 的颜色、字体、圆角与基础导航结构基本延续：Manrope/Fraunces、暖白背景、绿色强调色、三个主标签与底部设置/回收站/退出。新版新增成长条、资料区和更大的设置面板；页面内容分组和信息密度有变化。未做实际渲染，不断言两版像素一致或窄窗布局已通过验收。
