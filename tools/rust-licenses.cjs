const fs=require('node:fs'),path=require('node:path'),{spawnSync}=require('node:child_process')
const root=path.resolve(__dirname,'..')
const metadata=spawnSync('cargo',['metadata','--manifest-path','src-tauri/Cargo.toml','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],{cwd:root,encoding:'utf8',maxBuffer:16*1024*1024})
if(metadata.status!==0)throw Error(metadata.stderr||'cargo metadata failed')
const graph=JSON.parse(metadata.stdout),packages=new Map(graph.packages.map(p=>[p.id,p])),nodes=new Map(graph.resolve.nodes.map(n=>[n.id,n]))
const selected=new Set(),visit=id=>{if(selected.has(id))return;selected.add(id);for(const dep of nodes.get(id)?.deps||[])if(dep.dep_kinds.some(k=>k.kind===null))visit(dep.pkg)}
visit(graph.resolve.root)
const manifest=[],texts=['Catch Next — resolved Rust dependency licenses, including related code-generation dependencies.\nFonts and plant art retain their separate bundled licenses.\n'],missing=[]
for(const id of [...selected].sort()){
  const pkg=packages.get(id),directory=path.dirname(pkg.manifest_path)
  const files=fs.readdirSync(directory).filter(name=>/^(LICEN[CS]E|COPYING|NOTICE)([._-]|$)/i.test(name)&&fs.statSync(path.join(directory,name)).isFile()).map(name=>path.join(directory,name))
  if(pkg.license_file){const resolved=path.resolve(directory,pkg.license_file);if(resolved.startsWith(directory+path.sep)&&fs.existsSync(resolved)&&!files.includes(resolved))files.push(resolved)}
  if(pkg.source===null&&pkg.name.startsWith('catch-'))files.push(path.join(root,'LICENSE'))
  const supplement=path.join(root,'assets/licenses/rust',`${pkg.name}-${pkg.version}-LICENSE.txt`)
  if(!files.length&&fs.existsSync(supplement))files.push(supplement)
  if(!files.length){missing.push(pkg.name+' '+pkg.version);continue}
  const vcs=path.join(directory,'.cargo_vcs_info.json'),revision=fs.existsSync(vcs)?JSON.parse(fs.readFileSync(vcs,'utf8')).git?.sha1:null
  const source=pkg.source?`https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download`:null
  manifest.push({name:pkg.name,version:pkg.version,license:pkg.license,repository:pkg.repository,revision,source})
  texts.push('\n'+'='.repeat(72)+`\n${pkg.name} ${pkg.version}\n${pkg.license||''}\nRepository: ${pkg.repository||''}\nSource revision: ${revision||''}\nUnmodified source: ${source||'Catch project source'}\n`)
  for(const file of [...new Set(files)])texts.push('\n'+path.basename(file)+'\n'+fs.readFileSync(file,'utf8'))
}
if(missing.length)throw Error('Missing runtime license text: '+missing.join(', '))
const destination=path.join(root,'frontend/public/licenses');fs.mkdirSync(destination,{recursive:true});fs.writeFileSync(path.join(destination,'RUST.txt'),texts.join('\n'));fs.writeFileSync(path.join(destination,'RUST-DEPENDENCIES.json'),JSON.stringify(manifest,null,2)+'\n')
console.log(`Collected ${manifest.length} Rust runtime license sets`)
