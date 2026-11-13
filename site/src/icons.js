const paths = {
  chevron:'m7 10 5 5 5-5',
  info:'M12 11v6M12 7h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0',
  download:'M12 3v12m-5-5 5 5 5-5M5 16v5h14v-5',
  reset:'M3 11a9 9 0 1 1 2 7M3 4v7h7',
  list:'M9 5h11M9 12h11M9 19h11M3 5h1M3 12h1M3 19h1',
  timer:'M9 2h6M12 2v3M18 5l2-2M20 13a8 8 0 1 1-16 0 8 8 0 0 1 16 0M12 9v5l3 2',
  timeline:'M5 3v18M5 6h4M5 12h4M5 18h4M13 5h7M13 11h7M13 17h7',
  collapse:'M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
  plus:'M12 5v14M5 12h14',
  check:'M5 12l4 4L19 6',
  play:'m8 5 11 7-11 7z',
  pause:'M8 5v14M16 5v14',
  stop:'M6 6h12v12H6Z',
  next:'M3 12h14M11 6l6 6-6 6M21 5v14',
  lock:'M7 10V7a5 5 0 0 1 10 0v3M5 10h14v11H5zM12 14v3',
  user:'M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0M4 21v-2a8 8 0 0 1 16 0v2',
  leaf:'M12 21v-9M12 15C5 16 2 11 3 6c6-1 10 3 9 9M12 11c-1-6 4-9 9-8 1 6-3 10-9 8',
  offline:'M3 3l18 18M8 8a10 10 0 0 1 13 1M3 9l1-1M6 13l2-1M13 12l5 1M9 17l3-2 3 2M12 21h.01'
}

function icon(name) {
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg')
  for(const [key,value] of Object.entries({viewBox:'0 0 24 24',fill:'none',stroke:'currentColor','stroke-width':'1.6','stroke-linecap':'round','stroke-linejoin':'round','aria-hidden':'true'}))svg.setAttribute(key,value)
  const line=document.createElementNS(svg.namespaceURI,'path')
  line.setAttribute('d',paths[name] || paths.check)
  if(name==='collapse'){line.setAttribute('fill','currentColor');line.setAttribute('stroke','none')}
  svg.append(line)
  return svg
}

module.exports={icon}
