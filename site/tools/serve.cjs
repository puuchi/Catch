const http=require('node:http'),fs=require('node:fs'),path=require('node:path')
const {build}=require('./build.cjs')
const root=path.resolve(__dirname,'..')
build()
const types={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.png':'image/png','.svg':'image/svg+xml','.ttf':'font/ttf','.txt':'text/plain; charset=utf-8','.md':'text/plain; charset=utf-8'}
const server=http.createServer((request,response)=>{
  let pathname
  try{pathname=decodeURIComponent(new URL(request.url,'http://localhost').pathname)}catch{response.writeHead(400).end();return}
  const file=path.resolve(root,'.'+(pathname==='/'?'/index.html':pathname))
  if(!file.startsWith(root+path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()){response.writeHead(404).end('Not found');return}
  response.writeHead(200,{'Content-Type':types[path.extname(file)] || 'application/octet-stream','Cache-Control':'no-store','X-Content-Type-Options':'nosniff'})
  fs.createReadStream(file).pipe(response)
})
let port=Number(process.env.CATCH_SITE_PORT || 4173)
server.on('error',error=>{if(error.code==='EADDRINUSE'){port++;server.listen(port,'127.0.0.1')}else throw error})
server.listen(port,'127.0.0.1',()=>console.log(`Catch local preview: http://127.0.0.1:${port}/`))
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>server.close(()=>process.exit()))
