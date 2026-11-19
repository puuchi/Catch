// A paused or finished foreground card remains visible until another matter takes over.
function foreground(activities, retainedId) {
  return activities.find(item => item.status === 'running')
    || activities.find(item => item.id === retainedId)
    || activities.filter(item => item.status === 'paused')
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))[0]
    || null
}

function statusText(activity) {
  return activity ? {running:'正在投入', paused:'准备继续', completed:'这件事告一段落'}[activity.status] : '从一件小事开始'
}

module.exports = {foreground, statusText}
