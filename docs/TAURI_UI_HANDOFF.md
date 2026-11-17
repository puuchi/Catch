# Catch Tauri 界面修改交接

更新日期为 2026 年 11 月 17 日，仓库位于 `D:\Data\Documents\Project\Catch`，当前分支为 `work/catch-next`。“正在做”收尾版保存为 `feat(current): finalize progress workflow and allow narrower windows`，可用 Git 日志定位；时间轴后续改动与该版本分开。

## 用户的工作偏好

- 每次修改完成都要说明怎么实现，包括使用的交互方式、保存方式和验证结果。
- 只修改用户提出的内容。其他已经满意的界面和功能以当前版本为基础。
- 用户会在新对话继续设计“正在做”及后续页面，需结合用户接下来给出的体验反馈确定具体修改。
- 用户希望先体验具体结果，再指出满意和不满意的地方。此前待办页已通过可交互样板讨论，随后接入真实程序。

## 当前实现和入口

| 内容 | 代码入口 |
| --- | --- |
| Vue 应用和导航 | `frontend/src/App.vue` |
| 待办新增、展开、菜单、归档、恢复 | `frontend/src/components/Todos.vue` |
| 正在做、前台事项和后台暂停事项 | `frontend/src/components/Current.vue` |
| 时间轴 | `frontend/src/components/Timeline.vue` |
| 事项及投入详情 | `frontend/src/components/Timeline.vue` 中的 `matter-detail-body` |
| 待办、事项、单次投入编辑 | `frontend/src/components/Editors.vue` |
| 资料关联、修改、移除 | `frontend/src/components/Resources.vue`、`ResourceDialog.vue` |
| 图标菜单和截止日期控件 | `frontend/src/components/ActionMenu.vue`、`DatePicker.vue` |
| 外观、今日成长和植物浏览 | `frontend/src/components/Settings.vue`、`PlantSprite.vue` |
| 界面状态、调用和提示计时 | `frontend/src/store.ts`、`models.ts` |
| 投入、待办、资料和成长规则 | `crates/catch-core/src/` |
| 命令入口和状态版本校验 | `crates/catch-app/src/lib.rs` |
| 文件保存及中断恢复 | `crates/catch-storage/src/` |
| Tauri 宿主 | `src-tauri/src/main.rs` |
| 原生悬浮植物 | `src-tauri/src/native.rs` |

Electron 代码仍在 `src/`，用作已有行为的参考。当前改动接入 Vue、Tauri 2 和 Rust 这套实现。较早的需求评估、两版盘点和样板约定分别见 `docs/REQUIREMENTS_REVIEW_2026-11-15.md`、`docs/TAURI_ELECTRON_COMPARISON.md`、`docs/TODO_INTERACTION_PLAN.md`。这些文件记录过候选方案，涉及最终交互时请以本文件和最新代码为准。

## 已确认并接入的行为

- 单击待办标题展开或收起备注、日期、重复和资料，双击编辑。长备注默认两行，可展开全文。
- 紧急待办排列在普通待办前面。日期位于资料左侧。
- 待办保留轮廓播放图标和三个点。开始投入优先继续同一待办已有的未结束事项，关联依据为待办 ID。
- 待办和资料的操作菜单使用图标，名称放在 Tooltip 和无障碍标签中。菜单支持键盘移动及关闭后的焦点返回。
- 资料菜单为“修改”和“移除”。修改窗口可以一起修改名称和地址，文件、文件夹也可编辑名称和路径，并可重新选择路径。
- 文件、文件夹和网页分别添加。Windows 的文件和文件夹使用不同选择器。
- 待办编辑的紧急与日历位于取消、保存同一行。备注快捷键提示放在占位符中。
- 日历使用独立可见按钮，悬停显示应用自己的“截止日期”Tooltip，点击后通过 `showPicker()` 打开原生日期选择器。透明日期输入框不再覆盖按钮接收鼠标。
- 新增待办的工具栏包含紧急、日历、重复和资料曲别针。重复仅显示图标，Tooltip 为“重复”或“修改重复”。
- 新增资料暂存在 `ui.composerResources`，随界面草稿保存。普通待办带资料时使用 `todos/create-with-resources` 一次保存标题、备注和资料。重复待办沿用 `recurrence/create` 的资料字段。取消时清空资料，创建失败保留草稿。
- “历史”入口已改为归档图标，位于应用底部垃圾桶旁。“归档已完成”图标位于“今日已完成”标题旁，原有待办计数工具行已删除。归档页左上方有独立的返回箭头，Tooltip 为“返回待办”。归档记录右侧显示日期和恢复图标。恢复保留原 ID、备注、资料、重复关联和扩展字段。
- 归档恢复成功与失败的全部提示均在 2 秒后消失，包含周期冲突。回收站恢复操作的错误提示也使用 2 秒。新的提示会重新计时。
- 周期事项继续遵循同一规则最多一个未完成实例，恢复冲突会显示原因。
- 页面刷新时，当前窗口可以重复通知绘制完成，旧窗口版本和已收起窗口的回调仍被拦截。
- 今日投入展开独立的成长面板，保留每日目标。选定植物后展示五阶段并突出当前阶段，右上角图标可浏览所有植物，浏览只改变预览。
- 绿芽五阶段选取素材帧 `0, 8, 17, 26, 35`，依次为土壤、一片叶、两片叶、三片叶、四片叶，Vue 和原生悬浮植物使用相同帧。
- 用户在体验后要求回退玻璃卡片和缩放。当前悬浮形态恢复为 `ccf11f2` 的 36 × 36 逻辑像素植物、浅色圆底板和状态点，跟随系统 DPI。原生实现和偏好逻辑已恢复该版本，玻璃背景窗口、卡片底部阶段点、Ctrl+滚轮及菜单缩放已随回退删除。

## 本轮“正在做”已确认并接入的行为

- 进展回车或失焦保存，成功后显示正文和铅笔，点击铅笔重新编辑。失败保留草稿和编辑态，并阻止随后暂停、继续或开始新事项。草稿继续使用 `ui.progressDrafts`。
- “下一步”允许空进展，沿用同一事项划分下一次投入。前端保存锁与点击次数拦截双击，Rust 同时校验版本和最后 sessionId，旧 session 不能重复分段。
- 后台暂停事项显示事项自身颜色，按 `updatedAt` 最近更新优先；标题只保留“后台 数量”，删除重复的两竖线和中间的点。按用户确认，不修改顶部“正在做”标签的提示点规则。
- 结束卡片删除“重新开始”，显示最后一条非空进展；“查看过程”进入现有事项详情，展示每次投入的时间、时长和进展。
- 只在已结束卡片增加关闭。关闭只收起卡片、不删除记录、不自动顶上其他暂停事项；`ui.foregroundClosed` 随界面保存，重开保持收起，明确继续或开始活动才重新显示卡片。界面保存失败回退卡片。共用保存队列不再产生未处理的拒绝。
- 结束关联未完成待办的事项时询问“仅结束”或“结束并完成待办”；关闭确认框或 Escape 取消，不提前保存或清除草稿。前台、后台结束入口统一调用新 `finish` 命令。无有效未完成关联待办时直接仅结束。
- `finish` 的 command 为 `{ activityId, sessionId?, progress?, completeTodo, expectedTodosRevision }`，外层 `expectedRevision` 为 tracking 版本。Rust 同一操作保存最后进展、结束状态、可选待办完成及周期规则变化。
- `crates/catch-storage/src/finish.rs` 使用 `catch-finish-pending.json` 保存 before/after，再提交活动、待办、周期规则和回执。失败尝试恢复，不能确认时阻止后续写入，重启先恢复；仅整体完成才返回成功。`catch-finish-receipts.json` 支持成功请求跨重启幂等重放，目前没有回执清理策略。
- `FitTimer.vue` 根据计时器实际可用宽度及字体测量缩小字号，按钮组固定宽度；窗口最低可缩到 260 CSS 像素可用视口宽度。默认打开宽度仍为 320 逻辑像素，不整体缩放 UI；页面报告后按实际 DPR 校准原生最小宽度，保留系统文字缩放，高度最低仍为 300 逻辑像素。没有靠隐藏横向滚动条裁掉计时数字。
- 已有事项主卡片和详情继续复用待办同款 `Resources.vue` / `ResourceDialog.vue`，支持文件、文件夹、网页和原有修改/移除行为。用户确认不在“开始新活动”区域新增资料草稿入口。

## 后续“正在做”设计时需要了解的业务

- 同时只有一个事项运行，可以有多个后台暂停事项。
- 前台卡片会保留手动暂停的事项，继续时沿用同一事项及累计投入。
- 计时以 Rust 的有效投入区间和 sessions 为准，切换 WebView 或收起窗口后仍由宿主管理。
- 待办完成和计时结束保持独立；用户明确选择“结束并完成待办”时，通过本轮新增的可靠组合事务联动。
- 本次进展保存、下一步、暂停、结束、结束摘要和详情入口均有现有调用方，修改时需要检查对应正常入口。

## 验证和运行

本轮通过 Rust 工作区 77 项测试（0 失败，原有性能采样 1 项忽略）、工作区 Clippy、桌面 DPI/异常比例 2 项测试、桌面 Clippy、Vue 类型检查、格式检查、前端 5 项测试，以及 Windows release 编译。

真实 Vue 隔离宿主通过 20 组交互/渲染检查，覆盖保存成功后切正文、保存失败保留草稿并阻止后续动作、确认取消、仅结束/联动完成、后台结束、空进展下一步、慢保存连续点击和快速保存真实双击、最后进展及过程详情、关闭持久化及失败回退、同款资料关联和失败重试。覆盖 320×300、320×640、480×640、740×800，5/100/1000/999999 小时计时及按钮边界；长进展、详情、相关弹窗、三个主页面、归档/回收站/设置均未发现横向溢出，浏览器未处理错误为零。

另用临时数据目录运行真实 release Tauri/WebView2/Rust：验证待办开始、进展保存、空下一步、防双击、网页资料、结束取消、联动完成、三次投入详情、关闭后重载；直接核对活动/待办/回执文件整体保存，事务日志已清理。原生 DPI=2、WebView DPR≈2.56 时，旧 260 宽度偏好被校准到 820 物理像素，视口报告 321 CSS 像素，相关渲染无横向溢出。测试实例已通过正常退出关闭，没有访问正式数据。Windows 文件/文件夹选择器及跨屏缩放仍请用户实机体验。

用户反馈后将最低可用宽度由 320 放宽到 260，并保留默认打开尺寸；后台标题只保留文字。收尾版重跑 20 组真实 Vue 检查，覆盖 260×300、260×640、280×640 及更宽尺寸，进展、资料、长计时、弹窗、详情及其他主页面未发现横向溢出，计时按钮宽度不变。Vue 类型/格式、前端 5 项测试、桌面 2 项尺寸测试通过；尚未在本收尾提交中重建原生 exe，最终时间轴版另行构建。

上一轮隔离界面检查覆盖归档入口位置、归档页独立返回按钮、已完成分组内的归档按钮、删除原工具行、归档成功和不同恢复错误的 2 秒提示、连续提示重新计时、日历悬停及打开选择器、普通和重复待办带资料创建、失败保留草稿、资料名称与路径修改、窄窗口布局。回退时已核对三个原生文件与 `ccf11f2` 完全一致。

常用命令在仓库根目录运行。

```powershell
npm.cmd --prefix frontend run check
npm.cmd --prefix frontend run format:check
npm.cmd --prefix frontend test
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\rust.ps1 -Action test
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\rust.ps1 -Action clippy
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\desktop.ps1 -Action build -TargetDirectory artifacts/tauri-next-target
```

本轮测试程序为 `artifacts/tauri-next-target/release/catch-next.exe`。运行前从旧 Catch Next 底部退出，应用使用单实例和数据目录锁。默认数据目录为 `%APPDATA%\io.github.puuchi.catch.next`。

本地的 `artifacts/requirements-review/` 中保留了隔离宿主和 Playwright 检查脚本，这些文件受 Git 忽略，仅在当前工作区可用。入口为 `ui-review.vite.mjs`、`check-real-ui.cjs`、`check-resource-paths.cjs` 和 `check-final-feedback.cjs`。示例日期固定在 11 月 16 日。

本轮 `check-current.cjs`、`check-native.cjs`、隔离宿主、截图和构建日志放在会话临时目录（不会随 Git 保留）；核心/应用/存储的 `tests/finish.rs`、桌面尺寸测试及前端投影测试随代码保留。

旧的植物重置测试脚本只针对 2026 年 11 月 16 日，后续测试直接使用当前程序即可。

## 可复制给新对话的提示词

> 继续 Catch 的 Tauri 2 / Rust 版本，在 `D:\Data\Documents\Project\Catch` 工作。请先阅读 `docs/TAURI_UI_HANDOFF.md`，查看最近的 Git 提交和“正在做”的真实代码。我们现在开始设计“正在做”及后续页面，我会给具体反馈。只修改我明确提出的内容，并在每次完成后告诉我具体怎么实现、怎么验证，以及需要我测试什么。
