const icons=require('../shared/icons')
const {matterColor,humanDuration,duration}=require('../../domain/tracking/time')
module.exports=function renderBackground(list,background,command) {
    list.replaceChildren()
    for (const item of background.sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))) {
      const row = document.createElement('div')
      row.className = 'background-item'
      row.dataset.activityId = item.id
      const content = document.createElement('div')
      content.className = 'background-content'
      const name = document.createElement('div')
      name.className = 'background-name'
      name.textContent = item.name
      name.title = item.name
      const heading = document.createElement('div')
      heading.className = 'background-title'
      const dot = document.createElement('span')
      dot.className = 'background-color-dot'
      dot.style.backgroundColor = matterColor(item.id,item.color).hex
      dot.setAttribute('aria-hidden','true')
      heading.append(dot,name)
      const total = document.createElement('div')
      total.className = 'background-duration'
      total.textContent = humanDuration(duration(item)).replace(/\s/g,'')
      total.title = '已暂停'
      content.append(heading, total)
      const actions = document.createElement('div')
      actions.className = 'background-actions'
      const resume = document.createElement('button')
      resume.className = 'background-resume'
      resume.append(icons.create('play'))
      resume.title = '继续'
      resume.setAttribute('aria-label', `继续：${item.name}`)
      resume.addEventListener('click', () => command('resume', {id:item.id}))
      const end = document.createElement('button')
      end.className = 'background-end'
      end.append(icons.create('stop'))
      end.title = '结束'
      end.setAttribute('aria-label', `结束：${item.name}`)
      end.addEventListener('click', () => command('end', {id:item.id}))
      actions.append(resume, end)
      row.append(content, actions)
      list.append(row)
    }
}
