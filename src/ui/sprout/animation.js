module.exports = function mountSprout(api) {
  const art = document.getElementById('sproutArt')
  const image = document.getElementById('sproutFrames')
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)')
  const frames = Number(image.dataset.frames)
  const startFrame = Number(image.dataset.startFrame || 0)
  const size = 36
  const duration = Math.round(Number(image.dataset.duration) * (frames-1-startFrame)/(frames-1))
  const firstFrame = `translateX(-${startFrame*size}px)`
  const lastFrame = `translateX(-${(frames-1)*size}px)`
  let state = {status:'idle', activityId:null}
  let growthMatterId = null
  let growth = null
  let ready = false
  let failed = false
  const hasNativeVisibility = typeof api?.onBubbleVisibility === 'function'
  let nativeVisible = !hasNativeVisibility && !document.hidden
  let visibilityRevision = 0

  function stopGrowth() {
    if (growth) growth.cancel()
    growth = null
  }
  function still() {
    stopGrowth()
    image.style.transform = lastFrame
    art.classList.remove('is-swaying')
    art.dataset.phase = failed ? 'fallback' : 'still'
  }
  function synchronizeMotion() {
    const active = state.status === 'running' && nativeVisible && !document.hidden && !reducedMotion.matches
    art.dataset.windowVisible = String(nativeVisible)
    art.dataset.motion = active ? 'playing' : 'paused'
    if (growth && growth.playState !== 'finished') {
      if (active && growth.playState !== 'running') growth.play()
      else if (!active && growth.playState !== 'paused') growth.pause()
    }
  }
  function startGrowth() {
    stopGrowth()
    art.classList.remove('is-swaying')
    art.dataset.phase = 'growing'
    image.style.transform = firstFrame
    growth = image.animate([
      {transform:firstFrame},
      {transform:lastFrame}
    ], {duration,easing:`steps(${frames-1-startFrame}, end)`,fill:'forwards'})
    growth.pause()
    growth.currentTime = 0
    const visit = growth
    visit.finished.then(() => {
      if (growth !== visit) return
      art.dataset.phase = 'grown'
      art.classList.add('is-swaying')
      synchronizeMotion()
    }).catch(error => {
      if (error.name !== 'AbortError') console.error('[sprout] Animation failed', error)
    })
  }
  function render() {
    art.dataset.activityState = state.status
    if (!ready || failed) { synchronizeMotion(); return }
    if (state.status === 'idle') {
      growthMatterId = null
      still()
    } else if (reducedMotion.matches) {
      if (state.status === 'running') growthMatterId = state.activityId
      still()
    } else if (state.status === 'running') {
      if (growthMatterId !== state.activityId) {
        growthMatterId = state.activityId
        startGrowth()
      } else if (!growth) {
        art.dataset.phase = 'grown'
        art.classList.add('is-swaying')
      }
    }
    synchronizeMotion()
  }
  const sprout = {
    update(next) {
      state = {status:next.status, activityId:next.activityId || null}
      render()
    }
  }
  document.addEventListener('visibilitychange', () => {
    if (!hasNativeVisibility) nativeVisible = !document.hidden
    synchronizeMotion()
  })
  reducedMotion.addEventListener('change', render)
  if (hasNativeVisibility) {
    api.onBubbleVisibility(visible => {
      visibilityRevision++
      nativeVisible = visible === true
      synchronizeMotion()
    })
    const initialRevision = visibilityRevision
    api.getBubbleVisibility().then(visible => {
      if (visibilityRevision !== initialRevision) return
      nativeVisible = visible === true
      synchronizeMotion()
    }).catch(error => console.error('[sprout] Unable to read native visibility', error))
  }
  image.decode().then(() => {
    ready = true
    art.classList.add('is-ready')
    render()
  }).catch(error => {
    failed = true
    still()
    console.error('[sprout] Sprite unavailable; using static sprout', error)
  })
return sprout
}
