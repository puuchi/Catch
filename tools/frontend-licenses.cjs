const fs=require('node:fs'),path=require('node:path')
const root=path.resolve(__dirname,'..'),modules=path.join(root,'frontend/node_modules')
const names=['vue','@vue/shared','@vue/reactivity','@vue/runtime-core','@vue/runtime-dom','@vue/compiler-core','@vue/compiler-dom','@vue/compiler-sfc','@tauri-apps/api']
const parts=['Catch next interface — third-party license texts.\nFonts and plant art retain the separate licenses in assets/.\n']
for(const name of names){
  const directory=path.join(modules,name),pkg=JSON.parse(fs.readFileSync(path.join(directory,'package.json'),'utf8'))
  const files=fs.readdirSync(directory).filter(file=>/^(LICENSE|COPYING|NOTICE)(\.|$|_)/i.test(file)&&fs.statSync(path.join(directory,file)).isFile())
  if(!files.length)throw Error('No license text for '+name)
  parts.push(`\n${'='.repeat(72)}\n${name} ${pkg.version} (${pkg.license})\n`)
  for(const file of files)parts.push('\n'+file+'\n'+fs.readFileSync(path.join(directory,file),'utf8'))
}
const destination=path.join(root,'frontend/public/licenses/FRONTEND.txt');fs.mkdirSync(path.dirname(destination),{recursive:true});fs.writeFileSync(destination,parts.join('\n'))
console.log(`Collected ${names.length} interface license sets`)
