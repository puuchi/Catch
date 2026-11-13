# Catch 本地演示网站

单页产品介绍与交互演示。网页复用 Catch 的纯计时业务模型和绿芽动画，但使用独立的内存示例数据，不读取桌面应用文件，不保存访客输入。

在项目根目录安装开发依赖后：

Windows 可以双击本目录的 `preview.cmd` 启动，也可以在项目根目录运行：

```powershell
node site/tools/serve.cjs
```

终端会打印本地地址，默认 `http://127.0.0.1:4173/`。修改 HTML 后刷新；修改 CSS、`site/src/` 或共享代码后，运行 `node site/tools/build.cjs` 再刷新。

`site/app.js` 和 `site/app.css` 是构建结果，不手工修改。页面全部使用相对资源路径，可用于 GitHub Pages 的项目子路径。

验证演示状态逻辑：

```powershell
node --test site/test/demo-state.test.cjs
```

将来发布所需的文件为 `index.html`、构建后的 `app.js`、`app.css` 和 `assets/`。开发源码、工具与测试无需作为网站页面发布。

演示包含前后台事项切换、暂停后保留前台、日期切换、颜色说明、结束摘要与重新开始。计时与时间轴规则复用领域模块；前台选择规则和表单提示复用软件的 UI 模块。网页补录按钮仅作悬停提示，完整补录在桌面版中使用。

字体和绿芽的原许可文件随资源保留；代码及素材说明见 `assets/THIRD_PARTY_NOTICES.md`。

仓库 CI 会构建并测试演示网站，将静态发布文件保存为 `Catch-Demo-site` 工作流产物。下载并解压该产物后，根目录包含 `index.html`、`app.js`、`app.css` 与 `assets/`。这一步不会自动部署 GitHub Pages。

## GitHub Pages

`.github/workflows/pages.yml` 已准备好独立的 Pages 构建与部署流程。启用仓库 Settings → Pages → GitHub Actions 后，推送 `main` 上的网站或共享模块变更会触发部署，也可手动运行工作流。构建只安装前端所需的开发依赖，跳过 Electron 二进制下载；部署写权限只授予部署任务。

本地查看最终部署目录：

```powershell
node site/tools/prepare-pages.cjs
```

输出为已被 Git 忽略的 `artifacts/pages/`，包括构建文件、素材许可和 `.nojekyll`，不包含开发源码或测试。准备脚本会先清理这个专用输出目录。

计划地址为 `https://puuchi.github.io/Catch/`。配置和 README 入口已就绪；本地文件准备不代表网站已发布，需完成仓库发布与 Pages 启用，再核对实际地址。
