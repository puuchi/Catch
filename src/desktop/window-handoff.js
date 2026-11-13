// Keep the outgoing native window visible until the incoming renderer has painted.
// A timeout leaves the caller's original window in place instead of showing a blank desktop.
async function rendererReady(contents, source) {
  let timeout
  try {
    return await Promise.race([
      contents.executeJavaScript(source),
      new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error('窗口内容准备超时')), 3000) })
    ])
  } finally { clearTimeout(timeout) }
}

async function showPaintedWindow(window) {
  const contents = window.webContents
  if (contents.isLoadingMainFrame()) {
    await new Promise((resolve, reject) => {
      const cleanup = () => {
        clearTimeout(timer)
        contents.removeListener('did-finish-load', loaded)
        contents.removeListener('did-fail-load', failed)
        contents.removeListener('destroyed', destroyed)
      }
      const loaded = () => { cleanup(); resolve() }
      const failed = (_, code, description) => { cleanup(); reject(new Error(`窗口加载失败：${code} ${description}`)) }
      const destroyed = () => { cleanup(); reject(new Error('窗口已关闭')) }
      const timer = setTimeout(() => { cleanup(); reject(new Error('窗口加载超时')) }, 5000)
      contents.once('did-finish-load', loaded)
      contents.once('did-fail-load', failed)
      contents.once('destroyed', destroyed)
    })
  }
  // load completion alone does not guarantee the avatar or local fonts are decoded.
  await rendererReady(contents, `Promise.all([
    document.fonts.ready,
    ...Array.from(document.images, image => image.decode().catch(() => {}))
  ]).then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))`)
  await new Promise((resolve, reject) => {
    const cleanup = () => {
      clearTimeout(timeout)
      contents.removeListener('destroyed', destroyed)
      if (!contents.isDestroyed()) contents.endFrameSubscription()
    }
    const destroyed = () => { cleanup(); reject(new Error('窗口已关闭')) }
    const timeout = setTimeout(() => { cleanup(); reject(new Error('窗口绘制超时')) }, 2000)
    contents.once('destroyed', destroyed)
    contents.beginFrameSubscription(false, image => {
      if (!window.isVisible() || image.isEmpty()) return
      cleanup()
      resolve()
    })
    if (window.isMinimized()) window.restore()
    window.showInactive()
    contents.invalidate()
  })
  // A static bubble need not repaint a second time. Allow the presented frame to
  // reach the compositor without requiring another content change or an animation.
  await rendererReady(contents, 'new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))')
}

module.exports = { showPaintedWindow }
