# Catch Tauri 界面修改交接

更新日期为 2026 年 11 月 17 日，仓库位于 `D:\Data\Documents\Project\Catch`，当前分支为 `work/catch-next`。用 `git log -1 --oneline` 查看本轮提交。

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
- “历史”入口已改为归档图标，位于应用底部垃圾桶旁。“归档已完成”图标位于“今日已完成”标题旁，原有待办计数工具行已删除。归档记录右侧显示日期和恢复图标。恢复保留原 ID、备注、资料、重复关联和扩展字段。
- 归档恢复成功与失败的全部提示均在 2 秒后消失，包含周期冲突。回收站恢复操作的错误提示也使用 2 秒。新的提示会重新计时。
- 周期事项继续遵循同一规则最多一个未完成实例，恢复冲突会显示原因。
- 页面刷新时，当前窗口可以重复通知绘制完成，旧窗口版本和已收起窗口的回调仍被拦截。
- 今日投入展开独立的成长面板，保留每日目标。选定植物后展示五阶段并突出当前阶段，右上角图标可浏览所有植物，浏览只改变预览。
- 绿芽五阶段选取素材帧 `0, 8, 17, 26, 35`，依次为土壤、一片叶、两片叶、三片叶、四片叶，Vue 和原生悬浮植物使用相同帧。
- 悬浮植物采用默认 68 × 76 逻辑像素的圆角暖白半透明玻璃卡片，内部为像素植物、浅色台座和五个成长阶段圆点。一个原生窗口提供系统背景模糊，另一个分层窗口绘制卡片和植物，两者一起移动、缩放、隐藏和销毁。系统模糊不可用时使用半透明暖白底色。
- 悬浮卡片支持 75%–150% 等比缩放，每步 5%。Ctrl+滚轮和右键菜单可以调整，右键可还原 68 × 76。缩放比例保存在 `catch-preferences.json` 的 `bubble_scale` 字段中，旧偏好文件默认使用 100%，展开和收起时按当前缩放宽度居中。

## 后续“正在做”设计时需要了解的业务

- 同时只有一个事项运行，可以有多个后台暂停事项。
- 前台卡片会保留手动暂停的事项，继续时沿用同一事项及累计投入。
- 计时以 Rust 的有效投入区间和 sessions 为准，切换 WebView 或收起窗口后仍由宿主管理。
- 待办完成和计时结束是独立操作，按用户后续明确提出的需求调整联动。
- 本次进展保存、下一步、暂停、结束、结束摘要和详情入口均有现有调用方，修改时需要检查对应正常入口。

## 验证和运行

本轮通过 Rust 工作区测试、Clippy、Vue 类型检查、格式检查、前端测试、隔离宿主中的真实 Vue 交互检查，以及 Windows release 编译。

隔离界面检查覆盖归档入口位置、已完成分组内的归档按钮、删除原工具行、归档成功和不同恢复错误的 2 秒提示、连续提示重新计时、日历悬停及打开选择器、普通和重复待办带资料创建、失败保留草稿、资料名称与路径修改、窄窗口布局。原生测试覆盖旧偏好文件兼容、缩放边界和序列化后恢复。玻璃卡片的实际桌面模糊效果仍需用户确认。

常用命令在仓库根目录运行。

```powershell
npm --prefix frontend run check
npm --prefix frontend run format:check
npm --prefix frontend test
.\tools\rust.ps1 -Action test
.\tools\rust.ps1 -Action clippy
.\tools\desktop.ps1 -Action test -TargetDirectory artifacts/tauri-optimized-target
.\tools\desktop.ps1 -Action build -TargetDirectory artifacts/tauri-optimized-target
```

本轮测试程序为 `artifacts/tauri-optimized-target/release/catch-next.exe`。运行前从旧 Catch Next 底部退出，应用使用单实例和数据目录锁。默认数据目录为 `%APPDATA%\io.github.puuchi.catch.next`。

本地的 `artifacts/requirements-review/` 中保留了隔离宿主和 Playwright 检查脚本，这些文件受 Git 忽略，仅在当前工作区可用。入口为 `ui-review.vite.mjs`、`check-real-ui.cjs`、`check-resource-paths.cjs` 和 `check-final-feedback.cjs`。示例日期固定在 11 月 16 日。

旧的植物重置测试脚本只针对 2026 年 11 月 16 日，后续测试直接使用当前程序即可。

## 可复制给新对话的提示词

> 继续 Catch 的 Tauri 2 / Rust 版本，在 `D:\Data\Documents\Project\Catch` 工作。请先阅读 `docs/TAURI_UI_HANDOFF.md`，查看最近的 Git 提交和“正在做”的真实代码。我们现在开始设计“正在做”及后续页面，我会给具体反馈。只修改我明确提出的内容，并在每次完成后告诉我具体怎么实现、怎么验证，以及需要我测试什么。
