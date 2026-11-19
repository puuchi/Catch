# Catch 本地演示网站

单页产品介绍与交互演示。演示窗口使用桌面版的 Vue 组件、样式和素材。浏览器实现提供独立的示例数据，刷新或重置会清除本次输入。

在项目根目录安装 `npm ci` 和 `npm --prefix frontend ci` 后：

Windows 可以双击本目录的 `preview.cmd` 启动，也可以在项目根目录运行：

```powershell
node site/tools/serve.cjs
```

终端会打印本地地址，默认 `http://127.0.0.1:4173/`。修改官网 HTML 后刷新；修改 CSS、演示源代码或桌面组件后，运行 `node site/tools/build.cjs` 再刷新。

`site/app.js`、`site/app.css` 和 `site/desktop-demo/` 由构建生成。页面全部使用相对资源路径，可用于 GitHub Pages 的项目子路径。

验证演示状态逻辑：

```powershell
node --test site/test/demo-state.test.cjs
```

静态发布目录包含 `index.html`、`app.js`、`app.css`、`assets/` 和 `desktop-demo/`。

演示可关联示例文件、文件夹和网页，设置并预览重复待办，选择植物并模拟投入成长。还可在前后台事项之间切换，结束计时后查看每段投入和进展。文件路径仅作为示例文字；补录和完整记录编辑请使用桌面版。

字体和绿芽的原许可文件随资源保留；代码及素材说明见 `assets/THIRD_PARTY_NOTICES.md`。

仓库 CI 会构建并测试演示网站，将静态发布目录保存为 `Catch-Demo-site` 工作流产物。部署由独立的 Pages 工作流处理。

## GitHub Pages

`.github/workflows/pages.yml` 已准备好独立的 Pages 构建与部署流程。启用仓库 Settings → Pages → GitHub Actions 后，推送 `main` 上的网站或共享模块变更会触发部署，也可手动运行工作流。构建只安装前端所需的开发依赖；部署写权限只授予部署任务。

本地查看最终部署目录：

```powershell
node site/tools/prepare-pages.cjs
```

输出为已被 Git 忽略的 `artifacts/pages/`，包括构建文件、素材许可和 `.nojekyll`，不包含开发源码或测试。准备脚本会先清理这个专用输出目录。

计划地址为 `https://puuchi.github.io/Catch/`。配置和 README 入口已就绪；本地文件准备不代表网站已发布，需完成仓库发布与 Pages 启用，再核对实际地址。
