const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict')
const root=path.resolve(__dirname,'../src'),graph=new Map()
function walk(directory){return fs.readdirSync(directory,{withFileTypes:true}).flatMap(entry=>entry.isDirectory()?entry.name==='generated'?[]:walk(path.join(directory,entry.name)):[path.join(directory,entry.name)])}
for(const file of walk(root).filter(file=>file.endsWith('.js'))) {
  const relative=path.relative(root,file).replaceAll('\\','/'),source=fs.readFileSync(file,'utf8'),deps=[]
  if(relative.startsWith('domain/'))assert.ok(!/\b(window|document|process)\./.test(source),relative+' depends on runtime globals')
  for(const [,specifier] of source.matchAll(/require\(['"]([^'"]+)['"]\)/g)) {
    if(relative.startsWith('domain/'))assert.ok(specifier.startsWith('.'),relative+' imports '+specifier)
    if(relative.startsWith('ui/'))assert.ok(specifier.startsWith('.'),relative+' imports runtime '+specifier)
    if(!specifier.startsWith('.'))continue
    let target=path.resolve(path.dirname(file),specifier)
    if(fs.existsSync(target)&&fs.statSync(target).isDirectory())target=path.join(target,'index.js')
    else if(!target.endsWith('.js'))target+='.js'
    assert.ok(fs.existsSync(target),'Missing module: '+target)
    const dest=path.relative(root,target).replaceAll('\\','/')
    if(relative.startsWith('domain/'))assert.ok(dest.startsWith('domain/'),relative+' crosses domain boundary')
    if(relative.startsWith('application/'))assert.ok(/^(application|domain|contracts)\//.test(dest),relative+' imports concrete adapter '+dest)
    if(relative.startsWith('ui/'))assert.ok(/^(ui|domain|contracts)\//.test(dest),relative+' imports privileged '+dest)
    deps.push(target)
  }
  graph.set(file,deps)
}
function visit(file,trail=new Set(),done=new Set()) {
  assert.ok(!trail.has(file),'Circular dependency: '+file)
  if(done.has(file))return
  trail.add(file);for(const dependency of graph.get(file)||[])visit(dependency,trail,done)
  trail.delete(file);done.add(file)
}
const done=new Set();for(const file of graph.keys())visit(file,new Set(),done)
console.log(`PASS dependency boundaries and cycles (${graph.size} modules)`)
