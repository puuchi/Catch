# 性能验证

性能数据区分三个层次：纯业务计算、进程树资源、实际窗口交互。单个计算样本不能代表应用启动、界面响应或内存收益。

## 业务计算

`./tools/rust.ps1 -Action benchmark` 在 release 模式下使用 1000 条合成的已结束事项，对照原有投入计算与缓存投影。缓存预热不计入采样；普通测试另外覆盖原版操作轨迹中的变更、非零投入、跨日边界和时间回拨。

## 进程树观察

使用 `./tools/measure-desktop.ps1 -RootProcessId <PID> -DiagnosticPath <独立测试目录>/window-diagnostics.json -ExpectedPhase expanded -SceneLabel <场景> -OutputPath <本地结果.json>`。

植物模式将 `-ExpectedPhase` 改为 `bubble`。报告包含程序哈希、Windows/WebView2 版本、进程数、私有工作集、私有提交和 CPU；同时检查采样期间进程树、根进程身份及宿主报告的窗口阶段。缺少诊断文件时，场景稳定性标为未验证。

宿主诊断不能替代截图和实际点击验证。私有工作集不包含共享页；提交内存不等于物理内存；多个进程工作集直接相加会重复计算共享页。

前后对比须固定合成数据、窗口大小、DPI、计时状态、WebView2 版本与构建模式；预热后分别测展开、暂停、计时和植物模式。用户正在操作的预览只作为观察，不当成受控基线。

## 窗口交互与交付关口

仍需实测首次可见、展开到可操作、收起到 WebView2 退出、反复切换的残留，以及睡眠/恢复。记录 P50/P95 前应先有足够的实际操作样本；不从源码中的定时阈值推断延迟。24 小时稳定性须实际运行后再报告。

重放缓存的 4 MiB 预算只限制请求和编码响应的内容，不是整个 Rust 进程的内存上限。关键业务操作继续在可靠持久化后确认成功。

## 独立构建

用户正在查看旧预览时，可使用 `./tools/desktop.ps1 -Action pack -TargetDirectory artifacts/tauri-optimized-target` 构建独立产物，避免覆盖正在运行的程序。此命令只构建本地安装包，不执行安装或发布。

从当前预览底部退出后，运行 `./tools/review-optimized.ps1` 可打开优化版。脚本使用独立的 `artifacts/catch-optimized-review` 合成数据目录，只在全新目录中创建两条示例待办，之后保留试用修改。原版 Electron 无需退出；脚本不会读取原版数据。
